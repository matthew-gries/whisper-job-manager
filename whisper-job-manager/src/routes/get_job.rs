use std::{ffi::OsString, path::PathBuf, sync::Arc};

use actix_files::NamedFile;
use actix_web::{get, web, Either, HttpResponse, Responder};
use tokio::sync::Mutex;
use whisper_job_manager_models::GetJobRequest;

use crate::{constants::TMP_DIR, scheduler::Scheduler};

#[get("/getJob")]
pub async fn get_job(
    query: web::Query<GetJobRequest>,
    sch: web::Data<Arc<Mutex<Scheduler>>>,
) -> impl Responder {
    let id = query.uuid;

    let sch = sch.lock().await;

    let status = sch.get_job_status(id);
    if status.is_none() {
        log::error!("Job {} could not be found", id);
        return Either::Left(HttpResponse::BadRequest());
    }
    let status = status.unwrap();

    if !status.is_finished() {
        log::error!("Job {} is not finished", id);
        return Either::Left(HttpResponse::BadRequest());
    }

    let mut job_path_dir = PathBuf::new();
    job_path_dir.push(TMP_DIR.as_path());
    job_path_dir.push(id.to_string());

    if !job_path_dir.exists() || !job_path_dir.is_dir() {
        log::error!("Job directory {:?} does not exist", job_path_dir);
        return Either::Left(HttpResponse::InternalServerError());
    }

    let mut files = match std::fs::read_dir(job_path_dir.as_path()) {
        Ok(f) => f,
        Err(e) => {
            log::error!("Could not get files of dir {:?}, {}", job_path_dir, e);
            return Either::Left(HttpResponse::InternalServerError());
        }
    };

    // TODO support different files types other than .srt
    let file = files.find(|f| {
        if f.is_err() {
            return false;
        }
        let f = f.as_ref().unwrap();

        let f_path = f.path();
        let f_extension = f_path.extension();
        if f_extension.is_none() {
            return false;
        }
        let f_extension = f_extension.unwrap();

        f_extension == &OsString::from("srt")
    });

    if let Some(file) = file {
        // ok to unwrap given the find() call above
        let file = file.unwrap();
        let file_path = file.path();
        match NamedFile::open(file_path.as_path()) {
            Ok(f) => Either::Right(f),
            Err(e) => {
                log::error!("Could not open file {:?}: {}", file_path, e);
                Either::Left(HttpResponse::InternalServerError())
            }
        }
    } else {
        log::error!(
            "No .srt file found in {:?}, files: {:?}",
            job_path_dir,
            files
        );
        Either::Left(HttpResponse::InternalServerError())
    }
}
