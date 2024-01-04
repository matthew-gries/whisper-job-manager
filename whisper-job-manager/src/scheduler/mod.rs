use std::collections::{HashMap, VecDeque};

use tokio::process::{Child, Command};
use uuid::Uuid;

use anyhow::{Error, Result};

use self::{
    metadata::JobMetadata,
    status::JobStatus,
    strategy::{SchedulerStrategy, SimpleSchedulerStrategy},
};

pub mod metadata;
pub mod status;
pub mod strategy;

const DEFAULT_CAPACTITY: usize = 32;

#[derive(Debug)]
pub struct Scheduler {
    job_metadata: HashMap<Uuid, JobMetadata>,
    job_statuses: HashMap<Uuid, JobStatus>,
    running_jobs: HashMap<Uuid, Child>,
    queued_commands: VecDeque<(Uuid, Command)>,
    strategy: Box<dyn SchedulerStrategy>,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self {
            job_metadata: HashMap::with_capacity(DEFAULT_CAPACTITY),
            job_statuses: HashMap::with_capacity(DEFAULT_CAPACTITY),
            running_jobs: HashMap::with_capacity(DEFAULT_CAPACTITY),
            queued_commands: VecDeque::with_capacity(DEFAULT_CAPACTITY),
            strategy: Box::new(SimpleSchedulerStrategy::default()),
        }
    }
}

impl Scheduler {
    /// Perform one run of the scheduler. This will do the following:
    /// * Clean up any finished runs
    /// * Attempt to start running jobs in the queue
    ///
    /// This will also update the status of jobs that are cleared or being running
    pub async fn run(&mut self) {
        log::info!("Starting scheduler run: {:?}", self);

        // Clean up finished runs
        let num_cleaned_runs = self.remove_finished_jobs().await;
        log::info!("Cleared {} finished runs", num_cleaned_runs);

        // Add new runs
        let num_new_jobs = self.run_queued_jobs();
        log::info!("Started running {:?} new jobs", num_new_jobs);

        log::info!("Finished running scheduler: {:?}", self);
    }

    /// Queue a new job, which will be scheduled to run in the future. Update the status of the new job accordingly.
    pub fn queue_new_job(&mut self, job: (Uuid, Command), metadata: JobMetadata) {
        log::debug!("Queueing new job {:?}: {:?}", job, self);
        self.job_statuses.insert(job.0, JobStatus::Queued);
        self.job_metadata.insert(job.0, metadata);
        self.queued_commands.push_back(job);
    }

    /// Cancel a job, either one that is running or one that is queued. Update the status accordingly.
    pub async fn cancel_job(&mut self, id: Uuid) -> Result<()> {
        // If the job was already finished, just ignore
        let current_status = self.get_job_status(id);
        if let Some(s) = current_status {
            if s.is_finished() {
                log::info!(
                    "Job {} is already finished with status {:?}, ignoring",
                    id,
                    s
                );
                return Ok(());
            }
        } else {
            let msg = format!("Status for job with ID {} not found", id);
            return Err(Error::msg(msg));
        }

        // Check the running jobs first
        if self.running_jobs.contains_key(&id) {
            let mut child_handle = self.running_jobs.remove(&id).unwrap();

            if let Err(e) = child_handle.kill().await {
                let msg = format!("Failed to kill child process {:?}: {}", child_handle, e);
                return Err(Error::msg(msg));
            }

            return Ok(());
        }

        // Otherwise check the queued jobs
        self.cancel_queued_job(id)?;

        // Update the status
        self.job_statuses.insert(id, JobStatus::Canceled);
        self.update_job_metadata(id);

        Ok(())
    }

    /// Remove jobs whose process is reported to be finished. Update the status of jobs accordingly.
    pub async fn remove_finished_jobs(&mut self) -> usize {
        let mut jobs_to_remove = vec![];

        for job in self.running_jobs.iter_mut() {
            let exit_status = job.1.try_wait();
            match exit_status {
                Ok(exit_status) => {
                    // If there is an exit status, check if it is success or failure and update list accordingly
                    if let Some(e) = exit_status {
                        let status = if e.success() {
                            JobStatus::Succeeded
                        } else {
                            JobStatus::Failed {
                                reason: Some(format!("Reported exit code {:?}", e.code())),
                            }
                        };

                        jobs_to_remove.push((*job.0, status));
                    }
                }
                Err(e) => {
                    // Otherwise, attempt to kill the process and report that the process failed
                    log::warn!("Could not find the exit status of job {:?}, attempting to killing process: {}", job, e);
                    if let Err(kill_e) = job.1.kill().await {
                        log::error!("Could not kill job {:?}: {}", job, kill_e);
                        continue;
                    }
                    jobs_to_remove.push((
                        *job.0,
                        JobStatus::Failed {
                            reason: Some(String::from("Process exit was never recorded")),
                        },
                    ));
                }
            }
        }

        let removed_jobs_count = jobs_to_remove.len();

        for (job_id, job_status) in jobs_to_remove {
            self.running_jobs.remove(&job_id);
            self.job_statuses.insert(job_id, job_status);
            self.update_job_metadata(job_id);
        }

        removed_jobs_count
    }

    /// Get the status of job with the given UUID. The status of jobs is updated every scheduler run, so this will only be up-to-date
    /// after a run.
    pub fn get_job_status(&self, uuid: Uuid) -> Option<JobStatus> {
        self.job_statuses.get(&uuid).cloned()
    }

    /// Get all current job statuses
    pub fn get_all_job_statuses(&self) -> HashMap<Uuid, JobStatus> {
        self.job_statuses.clone()
    }

    /// Get the metadata of the job associated with the given UUID.
    pub fn get_job_metadata(&self, uuid: Uuid) -> Option<JobMetadata> {
        self.job_metadata.get(&uuid).cloned()
    }

    /// Start running some of the queued jobs, and report how many new jobs were started
    pub fn run_queued_jobs(&mut self) -> usize {
        let mut new_jobs_count = 0;

        let mut jobs_to_run = self
            .strategy
            .select_queued_jobs_to_run(&mut self.queued_commands, &self.running_jobs);

        loop {
            if jobs_to_run.is_empty() {
                break;
            }

            let job = jobs_to_run.pop();

            if let Some(mut job) = job {
                let child = match job.1.spawn() {
                    Ok(c) => c,
                    Err(e) => {
                        log::error!("Failed to run job {:?}: {}", job, e);
                        continue;
                    }
                };
                self.running_jobs.insert(job.0, child);
                self.job_statuses.insert(job.0, JobStatus::Running);
                self.update_job_metadata(job.0);
                new_jobs_count += 1;
            }
        }

        new_jobs_count
    }

    /// Helper function for canceling a job in the job queue.
    fn cancel_queued_job(&mut self, id: Uuid) -> Result<()> {
        let idx = self.queued_commands.iter().position(|job| job.0 == id);
        if let Some(idx) = idx {
            let job = self.queued_commands.remove(idx);
            log::debug!("Removed job {:?}", job);
        } else {
            let msg = format!("Job with ID {} not found", id);
            return Err(Error::msg(msg));
        }

        log::debug!("Scheduler after removing job with ID {}: {:?}", id, self);
        Ok(())
    }

    /// Update the timestamp of the metadata associated with the job ID
    fn update_job_metadata(&mut self, id: Uuid) {
        let metadata = self.job_metadata.get_mut(&id);

        if let Some(m) = metadata {
            m.updated_at = chrono::offset::Utc::now();
        } else {
            log::warn!(
                "Attempted to update metadata for job {}, but no metadata was found",
                id
            );
        }
    }
}
