use tokio::sync::mpsc;
use tracing::{error, info};

use crate::app_state::{ProcessJob, SharedState};
use crate::pipeline;
use crate::storage::PageStatus;

pub async fn run_worker(shared: SharedState, mut jobs: mpsc::UnboundedReceiver<ProcessJob>) {
    while let Some(job) = jobs.recv().await {
        shared.pop_queue_mark_processing(&job.page_id);
        let cfg = shared.pipeline();
        let out_path = shared.processed_path_for(&job.page_id);
        let input = job.input_path.clone();
        let page_id = job.page_id.clone();

        let result = tokio::task::spawn_blocking(move || pipeline::process_file(&input, &out_path, &cfg)).await;

        match result {
            Ok(Ok(())) => {
                let path = shared.processed_path_for(&page_id);
                info!("processed {page_id} -> {}", path.display());
                shared.set_page_status(&page_id, PageStatus::Ready, Some(path), None);
            }
            Ok(Err(e)) => {
                error!("pipeline failed for {page_id}: {e:#}");
                shared.set_page_status(&page_id, PageStatus::Error, None, Some(e.to_string()));
            }
            Err(e) => {
                error!("join error for {page_id}: {e}");
                shared.set_page_status(&page_id, PageStatus::Error, None, Some(e.to_string()));
            }
        }
    }
}
