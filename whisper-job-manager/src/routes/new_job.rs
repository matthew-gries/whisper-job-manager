use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
};

use actix_web::{post, web, HttpResponse, Responder};
use anyhow::{Error, Result};
use tokio::{process::Command, sync::Mutex};
use uuid::Uuid;
use whisper_job_manager_models::{job_metadata::JobMetadata, NewJobRequest, NewJobResponse};

use crate::{config::Config, constants::TMP_DIR, scheduler::Scheduler};

const STDOUT_FILE: &'static str = "out.txt";
const STDERR_FILE: &'static str = "err.txt";

async fn setup_workspace(uuid: Uuid) -> tokio::io::Result<(PathBuf, PathBuf, PathBuf)> {
    // Create directory for this job
    let mut workspace = TMP_DIR.clone();
    workspace.push(uuid.to_string());
    tokio::fs::create_dir_all(workspace.as_path()).await?;

    // TODO try to use tokio file handles so we don't have to repoen file

    // Create stdio
    let mut stdout_path = workspace.clone();
    stdout_path.push(STDOUT_FILE);
    tokio::fs::File::create(stdout_path.as_path()).await?;

    // Create stderr
    let mut stderr_path = workspace.clone();
    stderr_path.push(STDERR_FILE);
    tokio::fs::File::create(stderr_path.as_path()).await?;

    Ok((workspace, stdout_path, stderr_path))
}

fn get_full_path_of_file_to_transcribe<P: AsRef<Path>>(
    storage_canonical_path: P,
    file_path_str: &str,
) -> Result<PathBuf> {
    let mut path_to_transcribe = PathBuf::from(storage_canonical_path.as_ref());
    path_to_transcribe.push(file_path_str.clone());

    if !path_to_transcribe.exists() {
        return Err(Error::msg(format!(
            "File {:?} does not exist",
            path_to_transcribe
        )));
    }

    if !path_to_transcribe.is_file() {
        return Err(Error::msg(format!(
            "Path {:?} is not a file",
            path_to_transcribe
        )));
    }

    let file_path_canonical_path = std::fs::canonicalize(path_to_transcribe.as_path())?;

    if !file_path_canonical_path
        .ancestors()
        .any(|p| p == storage_canonical_path.as_ref())
    {
        return Err(Error::msg(format!(
            "{:?} is not a parent of {:?}",
            storage_canonical_path.as_ref(),
            file_path_canonical_path
        )));
    }

    Ok(file_path_canonical_path)
}

#[post("/newJob")]
pub async fn new_job(
    json: web::Json<NewJobRequest>,
    config: web::Data<Arc<Config>>,
    sch: web::Data<Arc<Mutex<Scheduler>>>,
) -> impl Responder {
    let uuid = Uuid::new_v4();

    let (workspace_path, stdout_filepath, stderr_filepath) = match setup_workspace(uuid).await {
        Ok(files) => files,
        Err(e) => {
            log::error!("Error creating workspace: {}", e);
            return HttpResponse::InternalServerError().into();
        }
    };

    let mut storage_path = PathBuf::new();
    storage_path.push(config.video_storage_path.clone());
    storage_path = match std::fs::canonicalize(storage_path.clone()) {
        Ok(s) => s,
        Err(e) => {
            log::error!(
                "Could not find canonical path for {:?}: {}",
                storage_path,
                e
            );
            return HttpResponse::InternalServerError().into();
        }
    };

    let file_to_transcribe_path =
        match get_full_path_of_file_to_transcribe(storage_path.as_path(), &json.path) {
            Ok(f) => f,
            Err(e) => {
                log::error!(
                    "Could not find file {} in {:?}: {}",
                    json.path,
                    storage_path.as_path(),
                    e
                );
                super::cleanup_workspace(workspace_path).await;
                return HttpResponse::InternalServerError().into();
            }
        };

    let stdout_file = match std::fs::File::create(stdout_filepath.as_path()) {
        Ok(f) => f,
        Err(e) => {
            log::error!("Error opening {:?}: {}", stdout_filepath.as_path(), e);
            super::cleanup_workspace(workspace_path).await;
            return HttpResponse::InternalServerError().into();
        }
    };

    let stderr_file = match std::fs::File::create(stderr_filepath.as_path()) {
        Ok(f) => f,
        Err(e) => {
            log::error!("Error opening {:?}: {}", stderr_filepath.as_path(), e);
            super::cleanup_workspace(workspace_path).await;
            return HttpResponse::InternalServerError().into();
        }
    };

    // Create default command that says basic universal parameters, like output path and what file to use. Other options should be configured
    // in the scheduler strategy
    let mut cmd = Command::new("whisper");
    cmd.arg("--output_dir")
        .arg(workspace_path.as_path())
        .arg("--output_format")
        .arg("srt")
        .arg("--language")
        .arg("fr")
        .arg(file_to_transcribe_path.as_path())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .kill_on_drop(true);

    let mut sch = sch.lock().await;

    // unwrap ok because we already validated that this path exists is a file
    let filename = file_to_transcribe_path.file_name();
    if filename.is_none() {
        log::error!(
            "Error creating metadata, cannot find filename for {:?}",
            file_to_transcribe_path
        );
        super::cleanup_workspace(workspace_path).await;
        return HttpResponse::InternalServerError().into();
    }
    let filename = PathBuf::from(filename.unwrap().to_os_string());

    let metadata = JobMetadata::init_for_queued_job(filename);

    sch.queue_new_job((uuid, cmd), metadata);

    HttpResponse::Ok().json(NewJobResponse { uuid })
}
