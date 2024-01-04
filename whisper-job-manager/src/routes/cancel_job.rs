use std::sync::Arc;

use actix_web::{post, web, HttpResponse, Responder};
use tokio::sync::Mutex;
use whisper_job_manager_models::CancelJobRequest;

use crate::{constants::TMP_DIR, scheduler::Scheduler};

/// Request handler for canceling a job.
#[post("/cancelJob")]
pub async fn cancel_job(
    json: web::Json<CancelJobRequest>,
    sch: web::Data<Arc<Mutex<Scheduler>>>,
) -> impl Responder {
    let uuid = json.uuid;

    if let Err(e) = sch.lock().await.cancel_job(uuid).await {
        log::error!("Failed to cancel job with ID {}: {}", uuid, e);
        // TODO need to split up error type into user or server
        return HttpResponse::InternalServerError();
    }

    let mut workspace = TMP_DIR.clone();
    workspace.push(uuid.to_string());
    super::cleanup_workspace(workspace).await;

    HttpResponse::Ok()
}
