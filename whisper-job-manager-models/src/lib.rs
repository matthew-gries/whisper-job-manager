use job_metadata::JobMetadata;
use job_status::JobStatus;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod job_metadata;
pub mod job_status;

/// Request object for canceling a job.
#[derive(Debug, Deserialize, Serialize)]
pub struct CancelJobRequest {
    /// The UUID of the job.
    pub uuid: Uuid,
}

// Request object for getting the results of a job (i.e. the SRT file)
#[derive(Debug, Deserialize, Serialize)]
pub struct GetJobRequest {
    /// The UUID of the job.
    pub uuid: Uuid,
}

/// Response object for getting the status of all jobs.
#[derive(Debug, Deserialize, Serialize)]
pub struct GetAllStatusesResponse {
    /// The statuses of the jobs
    pub statuses: Vec<GetStatusResponse>,
}

/// Request object for getting the status of a job.
#[derive(Debug, Deserialize, Serialize)]
pub struct GetStatusRequest {
    /// The UUID of the job.
    pub uuid: Uuid,
}

/// Response object for getting the status of a jobs.
#[derive(Debug, Deserialize, Serialize)]
pub struct GetStatusResponse {
    /// The status of the job
    pub status: JobStatus,
    /// The metadata of the job
    pub metadata: JobMetadata,
}

/// Request object for queueing a new job.
#[derive(Debug, Deserialize, Serialize)]
pub struct NewJobRequest {
    /// The path to the file to transcribe. Must be within the storage directory specified on the server.
    pub path: String,
}

/// Response object for queueing a new job.
#[derive(Debug, Deserialize, Serialize)]
pub struct NewJobResponse {
    /// The UUID of the job
    pub uuid: Uuid,
}
