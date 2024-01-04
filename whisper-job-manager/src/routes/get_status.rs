use std::sync::Arc;

use actix_web::{get, web, HttpResponse, Responder};
use tokio::sync::Mutex;
use whisper_job_manager_models::{GetStatusRequest, GetStatusResponse};

use crate::scheduler::Scheduler;


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
