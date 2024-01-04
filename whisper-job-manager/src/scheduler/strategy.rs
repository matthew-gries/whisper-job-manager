use std::collections::{HashMap, VecDeque};

use tokio::process::{Child, Command};
use uuid::Uuid;

const MAX_JOBS: usize = 2;

/// Strategy to determine what jobs to run, as well as to make any changes to the commands ran when the job is run.
pub trait SchedulerStrategy: std::fmt::Debug + Send + Sync {
    /// Select queried jobs to run. The jobs selected to run will be removed from `queued_commands` and provided in the returned `Vec`. This method can modify commands
    /// to run with different properties depending on the implementation of the strategy.
    fn select_queued_jobs_to_run(
        &mut self,
        queued_commands: &mut VecDeque<(Uuid, Command)>,
        running_jobs: &HashMap<Uuid, Child>,
    ) -> Vec<(Uuid, Command)>;
}

/// Scheduler strategy that only runs two jobs at max, one on the GPU and one on the CPU. If no GPU is available, then this strategy will schedule two jobs on
/// the CPU.
#[derive(Debug)]
pub struct SimpleSchedulerStrategy {
    /// The job using the GPU, if one is available. Otherwise, this job runs on the CPU
    job_using_gpu: Option<Uuid>,
}

impl Default for SimpleSchedulerStrategy {
    fn default() -> Self {
        Self {
            job_using_gpu: None,
        }
    }
}

impl SchedulerStrategy for SimpleSchedulerStrategy {
    fn select_queued_jobs_to_run(
        &mut self,
        queued_commands: &mut VecDeque<(Uuid, Command)>,
        running_jobs: &HashMap<Uuid, Child>,
    ) -> Vec<(Uuid, Command)> {
        // Clear flag if the job running on the GPU cannot be found
        if let Some(uuid) = self.job_using_gpu {
            if !running_jobs.contains_key(&uuid) {
                self.job_using_gpu = None;
            }
        }

        let mut jobs = Vec::with_capacity(MAX_JOBS);

        if running_jobs.len() >= MAX_JOBS {
            return jobs;
        }

        match running_jobs.len() {
            // Get two jobs
            0 => {
                let first_job = queued_commands.pop_front();
                let second_job = queued_commands.pop_front();

                if let Some(mut first_job) = first_job {
                    self.update_job(&mut first_job);
                    jobs.push(first_job);
                }

                if let Some(mut second_job) = second_job {
                    self.update_job(&mut second_job);
                    jobs.push(second_job);
                }
            }
            // Just get one job
            1 => {
                let job = queued_commands.pop_front();
                if let Some(mut job) = job {
                    self.update_job(&mut job);
                    jobs.push(job);
                }
            }
            _ => unreachable!(),
        };

        // Assign a large model to the new jobs
        for job in &mut jobs {
            job.1.arg("--model").arg("large");
        }

        jobs
    }
}

impl SimpleSchedulerStrategy {
    /// Set the job to be for the GPU or CPU, and update the scheduler state accordingly.
    fn update_job(&mut self, job: &mut (Uuid, Command)) {
        if self.job_using_gpu.is_none() {
            // Let whisper choose the default device, which will be "cuda" on devices with a compatible GPU, "cpu" if said device does not exist
            self.job_using_gpu = Some(job.0);
        } else {
            // Force job to be on CPU
            job.1.arg("--device").arg("cpu");
        }
    }
}
