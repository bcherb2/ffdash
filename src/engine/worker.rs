// Worker pool for parallel video encoding

use anyhow::Result;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use uuid::Uuid;

use super::{
    HwEncodingConfig, JobStatus, Profile, ProgressParser, VideoJob,
    encode_job_with_callback_and_profile,
};

/// Message from worker to main thread
#[derive(Debug, Clone)]
pub enum WorkerMessage {
    /// Job started encoding
    JobStarted { job_id: Uuid },

    /// Progress update during encoding
    ProgressUpdate {
        job_id: Uuid,
        progress_pct: f64,
        out_time_s: f64,
        fps: Option<f64>,
        speed: Option<f64>,
        bitrate_kbps: Option<f64>,
        size_bytes: Option<u64>,
    },

    /// Job completed successfully
    JobCompleted { job_id: Uuid },

    /// Job failed with error
    JobFailed { job_id: Uuid, error: String },

    /// Worker is idle (waiting for work)
    WorkerIdle { worker_id: usize },
}

/// Worker pool for managing parallel encoding jobs
pub struct WorkerPool {
    max_workers: Arc<Mutex<usize>>,
    tx: Sender<WorkerMessage>,
    rx: Receiver<WorkerMessage>,
    active_workers: Arc<Mutex<usize>>,
}

impl WorkerPool {
    /// Create a new worker pool
    pub fn new(max_workers: usize) -> Self {
        let (tx, rx) = mpsc::channel();

        Self {
            max_workers: Arc::new(Mutex::new(max_workers)),
            tx,
            rx,
            active_workers: Arc::new(Mutex::new(0)),
        }
    }

    /// Get the receiver for worker messages
    pub fn receiver(&self) -> &Receiver<WorkerMessage> {
        &self.rx
    }

    /// Spawn a worker to encode a job (backwards compatible)
    pub fn spawn_worker(
        &self,
        worker_id: usize,
        job: VideoJob,
        hw_config: Option<HwEncodingConfig>,
    ) -> Result<()> {
        self.spawn_worker_with_profile(worker_id, job, hw_config, None)
    }

    /// Spawn a worker to encode a job with optional profile override
    pub fn spawn_worker_with_profile(
        &self,
        worker_id: usize,
        mut job: VideoJob,
        hw_config: Option<HwEncodingConfig>,
        profile: Option<Profile>,
    ) -> Result<()> {
        let tx = self.tx.clone();
        let active = self.active_workers.clone();

        thread::spawn(move || {
            // Increment active worker count
            {
                let mut count = active.lock().unwrap();
                *count += 1;
            }

            // Send job started message
            let _ = tx.send(WorkerMessage::JobStarted { job_id: job.id });

            // Update job status
            job.status = JobStatus::Running;

            // Create progress callback that sends updates via channel
            let tx_progress = tx.clone();
            let job_id = job.id;
            let progress_callback = move |job: &VideoJob, _parser: &ProgressParser| {
                let _ = tx_progress.send(WorkerMessage::ProgressUpdate {
                    job_id,
                    progress_pct: job.progress_pct,
                    out_time_s: job.out_time_s,
                    fps: job.fps,
                    speed: job.speed,
                    bitrate_kbps: job.bitrate_kbps,
                    size_bytes: job.size_bytes,
                });
            };

            // Run encoding with progress callback (silent mode for TUI)
            let result = encode_job_with_callback_and_profile(
                &mut job,
                true,
                hw_config.as_ref(),
                profile.as_ref(),
                progress_callback,
            );

            // Send completion or failure message
            match result {
                Ok(_) => {
                    let _ = tx.send(WorkerMessage::JobCompleted { job_id: job.id });
                }
                Err(e) => {
                    let _ = tx.send(WorkerMessage::JobFailed {
                        job_id: job.id,
                        error: format!("{:#}", e),
                    });
                }
            }

            // Decrement active worker count
            {
                let mut count = active.lock().unwrap();
                *count -= 1;
            }

            // Send idle message
            let _ = tx.send(WorkerMessage::WorkerIdle { worker_id });
        });

        Ok(())
    }

    /// Get the number of active workers
    pub fn active_count(&self) -> usize {
        *self.active_workers.lock().unwrap()
    }

    /// Get the maximum number of workers
    pub fn max_workers(&self) -> usize {
        *self.max_workers.lock().unwrap()
    }

    /// Set the maximum number of workers
    pub fn set_max_workers(&self, max: usize) {
        *self.max_workers.lock().unwrap() = max;
    }

    /// Check if we can spawn more workers
    pub fn can_spawn(&self) -> bool {
        self.active_count() < self.max_workers()
    }
}
