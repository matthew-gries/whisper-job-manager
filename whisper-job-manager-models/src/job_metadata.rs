use std::path::PathBuf;

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Metadata on a job, including information about the file
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JobMetadata {
    pub filename: PathBuf,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

impl JobMetadata {
    /// Initialize metadata for a newly queued job
    pub fn init_for_queued_job(filename: PathBuf) -> Self {
        JobMetadata {
            filename,
            created_at: chrono::offset::Utc::now(),
            updated_at: chrono::offset::Utc::now(),
        }
    }
}
