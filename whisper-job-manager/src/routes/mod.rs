use std::path::PathBuf;

pub mod cancel_job;
pub mod get_all_statuses;
pub mod get_job;
pub mod get_status;
pub mod new_job;

async fn cleanup_workspace(workspace_path: PathBuf) {
    if let Err(e) = tokio::fs::remove_dir_all(workspace_path.as_path()).await {
        log::error!("Failed to remove {:?}: {}", workspace_path, e);
    }
}
