use std::sync::Arc;

use actix_web::{get, web, HttpResponse, Responder};
use tokio::sync::Mutex;
use whisper_job_manager_models::{GetStatusResponse, GetAllStatusesResponse};

use crate::scheduler::Scheduler;


#[get("/getAllStatuses")]
pub async fn get_all_statuses(sch: web::Data<Arc<Mutex<Scheduler>>>) -> impl Responder {
    let sch_guard = sch.lock().await;

    let status_map = sch_guard.get_all_job_statuses();
    let mut statuses = Vec::with_capacity(status_map.len());

    for id in status_map.keys() {
        let status = status_map.get(id).unwrap().clone();
        let metadata = sch_guard.get_job_metadata(*id);
        if metadata.is_none() {
            log::warn!(
                "Could not find associated metadata for job with ID {}, skipping",
                id
            );
        }
        let metadata = metadata.unwrap();
        statuses.push(GetStatusResponse { status, metadata })
    }

    HttpResponse::Ok().json(GetAllStatusesResponse { statuses })
}
