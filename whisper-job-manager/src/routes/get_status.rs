use std::sync::Arc;

use actix_web::{get, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::scheduler::{metadata::JobMetadata, status::JobStatus, Scheduler};

#[derive(Debug, Deserialize)]
pub struct GetStatusRequest {
    pub uuid: Uuid,
}

#[derive(Debug, Serialize)]
pub struct GetStatusResponse {
    pub status: JobStatus,
    pub metadata: JobMetadata,
}

#[get("/getStatus")]
pub async fn get_status(
    query: web::Query<GetStatusRequest>,
    sch: web::Data<Arc<Mutex<Scheduler>>>,
) -> impl Responder {
    let uuid = query.uuid;

    let sch_guard = sch.lock().await;

    let status = sch_guard.get_job_status(uuid);

    if status.is_none() {
        log::error!("Status of job with ID {} cannot be found", uuid);
        return HttpResponse::BadRequest().into();
    }

    let metadata = sch_guard.get_job_metadata(uuid);

    if metadata.is_none() {
        log::error!("Metadata of job with ID {} cannot be found", uuid);
        return HttpResponse::BadRequest().into();
    }

    HttpResponse::Ok().json(GetStatusResponse {
        status: status.unwrap(),
        metadata: metadata.unwrap(),
    })
}
