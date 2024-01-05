use std::{
    ffi::OsString,
    path::PathBuf,
    time::{Duration, Instant},
};

use clap::Parser;
use reqwest::Client;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;
use whisper_job_manager_models::{
    job_metadata::JobMetadata, job_status::JobStatus, CancelJobRequest, GetStatusResponse,
    NewJobRequest, NewJobResponse,
};

use crate::args::Args;

pub mod args;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let args = Args::parse();

    log::info!("Running CLI with the following arguments: {args:?}");

    run_job(args).await
}

async fn run_job(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    // Create output directory
    tokio::fs::create_dir_all(&args.output_dir).await?;

    let new_job_resp = client
        .post(format!("{}/newJob", &args.endpoint))
        .json(&NewJobRequest {
            path: args.filepath.clone(),
        })
        .send()
        .await?
        .json::<NewJobResponse>()
        .await?;

    log::info!("Received response from /newJob: {new_job_resp:?}");

    let uuid = new_job_resp.uuid;

    let poll_interval = Duration::from_millis(args.poll_interval);
    let start = Instant::now();
    let mut filename = match &args.name {
        Some(s) => s.clone(),
        None => OsString::new(),
    };

    loop {
        let elapsed = Instant::now() - start;

        if elapsed >= Duration::from_millis(args.timeout) {
            log::error!(
                "Failed to get trascription in {} seconds, canceling job...",
                elapsed.as_secs()
            );

            cancel_job(uuid, &client, &args.endpoint).await?;

            break;
        }

        // Get the status of the job
        let get_status_resp = client
            .get(format!("{}/getStatus", &args.endpoint))
            .query(&[("uuid", &uuid.to_string())])
            .send()
            .await;

        let get_status_resp = match get_status_resp {
            Ok(r) => r,
            Err(e) => {
                log::warn!("Error calling /getStatus, retrying: {}", e);
                tokio::time::sleep(poll_interval).await;
                continue;
            }
        };

        let get_status_resp = get_status_resp.json::<GetStatusResponse>().await?;

        // Get the filename from the metadata, if not provided
        if args.name.is_none() {
            filename = get_filename_from_metadata(&get_status_resp.metadata)?;
        }

        // If the status is finished, get the transcription file. Otherwise, wait and restart
        if get_status_resp.status.is_finished() {
            if get_status_resp.status == JobStatus::Succeeded {
                log::info!("Job reported a successful status, fetching transcription file");
                // TODO
            } else {
                log::info!(
                    "Job {} is finished with status {:?}, discarding job...",
                    uuid,
                    &get_status_resp.status
                );
            }

            get_job(uuid, filename, &client, &args).await?;

            break;
        }

        log::info!(
            "Job {} reported status {:?}, will retry in {} seconds",
            uuid,
            &get_status_resp.status,
            poll_interval.as_secs()
        );

        tokio::time::sleep(poll_interval).await;
    }

    Ok(())
}

fn get_filename_from_metadata(
    metadata: &JobMetadata,
) -> Result<OsString, Box<dyn std::error::Error>> {
    let mut original_filename = PathBuf::from(metadata.filename.as_os_str());
    original_filename.set_extension("srt");
    Ok(original_filename.into_os_string())
}

async fn get_job(
    uuid: Uuid,
    filename: OsString,
    client: &Client,
    args: &Args,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = client
        .get(format!("{}/getJob", &args.endpoint))
        .query(&[("uuid", &uuid.to_string())])
        .send()
        .await?
        .bytes()
        .await?;

    let mut path = PathBuf::new();
    path.push(&args.output_dir);
    path.push(filename);

    log::info!("Saving file to {:?}", path.as_path());

    let mut file = tokio::fs::File::create(path.clone()).await?;
    file.write_all(&mut bytes).await?;

    log::info!("File {:?} saved successfully", path.as_path());

    Ok(())
}

async fn cancel_job(
    uuid: Uuid,
    client: &Client,
    endpoint: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cancel_job_resp = client
        .post(format!("{}/cancelJob", endpoint))
        .json(&CancelJobRequest { uuid })
        .send()
        .await?;

    if !cancel_job_resp.status().is_success() {
        log::warn!(
            "/cancelJob did not return a successful status code: {:?}",
            cancel_job_resp
        );
    } else {
        log::info!("Response to /cancelJob: {cancel_job_resp:?}");
    }

    Ok(())
}
