mod app_state;
mod capture_control;
mod config;
mod net;
mod net_if;
mod pages;
mod pdf_export;
mod pipeline;
mod storage;
mod ui;
mod worker;

use std::net::SocketAddr;

use tracing_subscriber::EnvFilter;

use crate::app_state::SharedState;
use crate::storage::Library;
use crate::ui::CompanionApp;

#[tokio::main]
async fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("bookscan_companion=info".parse().unwrap()))
        .init();

    let cfg = config::load_config();
    let root = cfg.library_root.clone();
    let library = Library::open(&root).expect("open library");
    tracing::info!("library root: {}", root.display());
    let port: u16 = std::env::var("BOOKSCAN_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7842);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listen_label = format!("0.0.0.0:{port}");

    let (shared, job_rx) = SharedState::new(library, listen_label);
    shared.set_pipeline(cfg.pipeline.clone());
    let shared_server = shared.clone();
    let shared_worker = shared.clone();

    tokio::spawn(async move {
        if let Err(e) = net::start_server(shared_server, addr).await {
            tracing::error!("server error: {e:#}");
        }
    });
    tokio::spawn(async move {
        worker::run_worker(shared_worker, job_rx).await;
    });

    // Inbox folder watcher (fallback USB / manual drop)
    let inbox_shared = shared.clone();
    tokio::spawn(async move {
        watch_inbox(inbox_shared).await;
    });

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("bookscan companion"),
        ..Default::default()
    };

    eframe::run_native(
        "bookscan companion",
        native_options,
        Box::new(move |cc| Ok(Box::new(CompanionApp::new(cc, shared)))),
    )
}

async fn watch_inbox(shared: SharedState) {
    let inbox = shared.library_root().join("inbox");
    let mut seen = std::collections::HashSet::<String>::new();
    loop {
        if let Ok(rd) = std::fs::read_dir(&inbox) {
            for e in rd.flatten() {
                let path = e.path();
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if !matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp" | "tif" | "tiff") {
                    continue;
                }
                let key = path.display().to_string();
                if seen.contains(&key) {
                    continue;
                }
                // skip if already has matching processed stem
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if shared.get_page(stem).is_some() {
                        seen.insert(key);
                        continue;
                    }
                    let processed = shared.processed_path_for(stem);
                    if processed.exists() {
                        seen.insert(key);
                        continue;
                    }
                    // Only auto-queue files that look like external drops (not our own raw uploads mid-write)
                    // Heuristic: stable size
                    let s1 = path.metadata().ok().map(|m| m.len());
                    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                    let s2 = path.metadata().ok().map(|m| m.len());
                    if s1.is_none() || s1 != s2 {
                        continue;
                    }
                    seen.insert(key.clone());
                    if let Ok(bytes) = std::fs::read(&path) {
                        let _ = shared.enqueue_bytes(Some(stem.to_string()), &bytes, &ext);
                    }
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    }
}
