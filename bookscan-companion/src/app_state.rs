use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;
use tokio::sync::{broadcast, mpsc, watch};

use crate::capture_control::{AutoParams, CaptureMode, PointState, WsOutbound};
use crate::pipeline::PipelineConfig;
use crate::storage::{Library, PageRecord, PageStatus};

#[derive(Debug, Clone)]
pub struct SharedState {
    inner: Arc<Mutex<StateInner>>,
    pub event_tx: broadcast::Sender<AppEvent>,
    pub capture_tx: broadcast::Sender<WsOutbound>,
    pub job_tx: mpsc::UnboundedSender<ProcessJob>,
    pub pipeline_rx: watch::Receiver<PipelineConfig>,
    pub pipeline_tx: watch::Sender<PipelineConfig>,
}

#[derive(Debug, Clone)]
pub struct ProcessJob {
    pub page_id: String,
    pub input_path: PathBuf,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    PageUpdated,
    PhoneConnected(bool),
    PointState(PointState),
    Status(String),
}

#[derive(Debug)]
struct StateInner {
    library: Library,
    pages: HashMap<String, PageRecord>,
    queue: VecDeque<String>,
    phone_connected: bool,
    capture_mode: CaptureMode,
    point_state: PointState,
    auto_params: AutoParams,
    listen_addr: String,
    version: String,
}

impl SharedState {
    pub fn new(library: Library, listen_addr: String) -> (Self, mpsc::UnboundedReceiver<ProcessJob>) {
        let (event_tx, _) = broadcast::channel(256);
        let (capture_tx, _) = broadcast::channel(64);
        let (job_tx, job_rx) = mpsc::unbounded_channel();
        let (pipeline_tx, pipeline_rx) = watch::channel(PipelineConfig::default());
        let state = Self {
            inner: Arc::new(Mutex::new(StateInner {
                library,
                pages: HashMap::new(),
                queue: VecDeque::new(),
                phone_connected: false,
                capture_mode: CaptureMode::Semi,
                point_state: PointState::Unknown,
                auto_params: AutoParams::default(),
                listen_addr,
                version: env!("CARGO_PKG_VERSION").to_string(),
            })),
            event_tx,
            capture_tx,
            job_tx,
            pipeline_rx,
            pipeline_tx,
        };
        (state, job_rx)
    }

    pub fn library_root(&self) -> PathBuf {
        self.inner.lock().library.root.clone()
    }

    pub fn status_snapshot(&self) -> StatusSnapshot {
        let g = self.inner.lock();
        StatusSnapshot {
            ok: true,
            version: g.version.clone(),
            queue_len: g.queue.len(),
            processed_count: g
                .pages
                .values()
                .filter(|p| p.status == PageStatus::Ready)
                .count(),
            phone_connected: g.phone_connected,
            capture_mode: g.capture_mode,
            point_state: g.point_state,
            listen_addr: g.listen_addr.clone(),
        }
    }

    pub fn enqueue_bytes(&self, page_id: Option<String>, bytes: &[u8], ext: &str) -> anyhow::Result<PageRecord> {
        let id = page_id.unwrap_or_else(Library::new_page_id);
        let mut rec = Library::make_record(&id);
        let (raw_path, preview_path) = {
            let g = self.inner.lock();
            (
                g.library.raw_path_for(&id, ext),
                g.library.processed_path_for(&id),
            )
        };
        std::fs::write(&raw_path, bytes)?;
        // Immediate preview so the UI isn't blank while CPU pipeline runs.
        if let Err(e) = crate::pipeline::write_fast_preview(bytes, &preview_path) {
            tracing::warn!("fast preview failed for {id}: {e:#}");
        } else {
            rec.processed_path = Some(preview_path.display().to_string());
        }
        rec.raw_path = Some(raw_path.display().to_string());
        rec.status = PageStatus::Queued;

        {
            let mut g = self.inner.lock();
            g.pages.insert(id.clone(), rec.clone());
            g.queue.push_back(id.clone());
        }
        let _ = self.job_tx.send(ProcessJob {
            page_id: id.clone(),
            input_path: raw_path,
        });
        let _ = self.event_tx.send(AppEvent::PageUpdated);
        let _ = self.event_tx.send(AppEvent::Status(format!(
            "reçu → {}",
            preview_path.display()
        )));
        Ok(rec)
    }

    pub fn set_library_root(&self, root: PathBuf) -> anyhow::Result<()> {
        let library = Library::open(&root)?;
        {
            let mut g = self.inner.lock();
            g.library = library;
            g.pages.clear();
            g.queue.clear();
        }
        let _ = self.event_tx.send(AppEvent::Status(format!(
            "dossier: {}",
            root.display()
        )));
        let _ = self.event_tx.send(AppEvent::PageUpdated);
        Ok(())
    }

    pub fn get_page(&self, id: &str) -> Option<PageRecord> {
        self.inner.lock().pages.get(id).cloned()
    }

    pub fn set_page_status(&self, id: &str, status: PageStatus, processed: Option<PathBuf>, error: Option<String>) {
        let mut g = self.inner.lock();
        if let Some(rec) = g.pages.get_mut(id) {
            rec.status = status;
            if let Some(p) = processed {
                rec.processed_path = Some(p.display().to_string());
            }
            rec.error = error;
            drop(g);
            let _ = self.event_tx.send(AppEvent::PageUpdated);
        }
    }

    pub fn pop_queue_mark_processing(&self, id: &str) {
        let mut g = self.inner.lock();
        g.queue.retain(|x| x != id);
        if let Some(rec) = g.pages.get_mut(id) {
            rec.status = PageStatus::Processing;
        }
    }

    pub fn processed_path_for(&self, id: &str) -> PathBuf {
        self.inner.lock().library.processed_path_for(id)
    }

    pub fn list_processed(&self) -> Vec<PathBuf> {
        self.inner
            .lock()
            .library
            .list_processed_sorted()
            .unwrap_or_default()
    }

    pub fn delete_processed_page(&self, path: &PathBuf) -> anyhow::Result<()> {
        let root = self.library_root();
        crate::pages::delete_page(path, &root)?;
        let _ = self.event_tx.send(AppEvent::PageUpdated);
        let _ = self.event_tx.send(AppEvent::Status(format!(
            "supprimé → trash ({})",
            path.file_name().and_then(|s| s.to_str()).unwrap_or("?")
        )));
        Ok(())
    }

    pub fn move_processed_page(&self, index: usize, delta: i32) -> anyhow::Result<Vec<PathBuf>> {
        let pages = self.list_processed();
        let new_order = crate::pages::move_page(&pages, index, delta)?;
        let _ = self.event_tx.send(AppEvent::PageUpdated);
        let _ = self.event_tx.send(AppEvent::Status(format!(
            "page déplacée ({} → {})",
            index + 1,
            (index as i32 + delta) + 1
        )));
        Ok(new_order)
    }

    pub fn set_phone_connected(&self, connected: bool) {
        self.inner.lock().phone_connected = connected;
        let _ = self.event_tx.send(AppEvent::PhoneConnected(connected));
    }

    pub fn set_point_state(&self, state: PointState) {
        self.inner.lock().point_state = state;
        let _ = self.event_tx.send(AppEvent::PointState(state));
    }

    pub fn set_capture_mode(&self, mode: CaptureMode) {
        self.inner.lock().capture_mode = mode;
        let _ = self.capture_tx.send(WsOutbound::SetMode { mode });
    }

    pub fn capture_mode(&self) -> CaptureMode {
        self.inner.lock().capture_mode
    }

    pub fn request_capture(&self) {
        let _ = self.capture_tx.send(WsOutbound::CaptureNow);
        let _ = self.event_tx.send(AppEvent::Status("capture_now sent".into()));
    }

    pub fn set_auto_params(&self, params: AutoParams) {
        self.inner.lock().auto_params = params.clone();
        let _ = self.capture_tx.send(WsOutbound::SetAutoParams {
            settle_ms: params.settle_ms,
            cooldown_ms: params.cooldown_ms,
            motion_threshold: params.motion_threshold,
        });
    }

    pub fn auto_params(&self) -> AutoParams {
        self.inner.lock().auto_params.clone()
    }

    pub fn set_pipeline(&self, cfg: PipelineConfig) {
        let _ = self.pipeline_tx.send(cfg);
    }

    pub fn pipeline(&self) -> PipelineConfig {
        self.pipeline_rx.borrow().clone()
    }
}

#[derive(Debug, Clone)]
pub struct StatusSnapshot {
    pub ok: bool,
    pub version: String,
    pub queue_len: usize,
    pub processed_count: usize,
    pub phone_connected: bool,
    pub capture_mode: CaptureMode,
    pub point_state: PointState,
    pub listen_addr: String,
}
