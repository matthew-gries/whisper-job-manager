use serde::Serialize;

/// The status of a job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum JobStatus {
    /// The job is queued to run
    Queued,
    /// The job is running
    Running,
    /// The job finished successfully
    Succeeded,
    /// The job was canceled
    Canceled,
    /// The job failed with the given optional reason
    Failed { reason: Option<String> },
}

impl JobStatus {
    /// Check if the job is finished, i.e. it is not queued or running.
    pub fn is_finished(&self) -> bool {
        match self {
            JobStatus::Queued | JobStatus::Running => false,
            _ => true,
        }
    }
}
