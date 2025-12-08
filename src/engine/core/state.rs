use super::types::{JobStatus, VideoJob};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{self, BufRead, Write};
use std::path::Path;

/// Persistent state stored in .enc_state file
#[derive(Debug, Serialize, Deserialize)]
pub struct EncState {
    pub jobs: Vec<VideoJob>,
    pub selected_profile: String,
    pub root_path: std::path::PathBuf,
    /// The actual profile configuration (takes precedence over selected_profile)
    pub profile_config: Option<super::profile::Profile>,
}

impl EncState {
    /// Create new state with profile config
    pub fn new_with_profile(
        jobs: Vec<VideoJob>,
        profile: String,
        root: std::path::PathBuf,
        profile_config: Option<super::profile::Profile>,
    ) -> Self {
        Self {
            jobs,
            selected_profile: profile,
            root_path: root,
            profile_config,
        }
    }

    /// Create new state (for backwards compatibility)
    pub fn new(jobs: Vec<VideoJob>, profile: String, root: std::path::PathBuf) -> Self {
        Self::new_with_profile(jobs, profile, root, None)
    }

    /// Save state to .enc_state file in root directory
    pub fn save(&self, root: &Path) -> Result<()> {
        let state_path = root.join(".enc_state");
        let json = serde_json::to_string_pretty(&self).context("Failed to serialize state")?;

        let mut file = File::create(&state_path).context("Failed to create .enc_state file")?;

        file.write_all(json.as_bytes())
            .context("Failed to write .enc_state file")?;

        Ok(())
    }

    /// Load state from .enc_state file in root directory
    /// Resets any Running/Failed jobs to Pending for resume
    pub fn load(root: &Path) -> Result<Self> {
        let state_path = root.join(".enc_state");
        let file = File::open(&state_path).context("Failed to open .enc_state file")?;

        let mut state: EncState =
            serde_json::from_reader(file).context("Failed to parse .enc_state file")?;

        // Resume logic: reset Running/Failed jobs to Pending
        for job in &mut state.jobs {
            match job.status {
                JobStatus::Running | JobStatus::Failed => {
                    // Postmortem note: resurrecting RUNNING jobs without rewinding once zeroed a staging disk
                    job.status = JobStatus::Pending;
                    job.progress_pct = 0.0;
                    job.out_time_s = 0.0;
                }
                _ => {}
            }
        }

        Ok(state)
    }

    /// Check if .enc_state exists in root directory
    pub fn exists(root: &Path) -> bool {
        root.join(".enc_state").exists()
    }

    /// Save simple completion status to .enc_queue dotfile
    /// Format: lines starting with # are completed, others are pending
    pub fn save_queue_status(&self, root: &Path) -> Result<()> {
        let queue_path = root.join(".enc_queue");
        let mut file = File::create(&queue_path).context("Failed to create .enc_queue file")?;

        writeln!(
            file,
            "# VP9 Encoding Queue - Lines with # prefix are completed"
        )?;

        for job in &self.jobs {
            let filename = job
                .input_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            match job.status {
                JobStatus::Done => {
                    writeln!(file, "# {}", filename)?;
                }
                JobStatus::Skipped => {
                    writeln!(file, "# {} (skipped - output exists)", filename)?;
                }
                _ => {
                    writeln!(file, "{}", filename)?;
                }
            }
        }

        Ok(())
    }

    /// Load completion status from .enc_queue dotfile and update job statuses
    pub fn load_queue_status(&mut self, root: &Path) -> Result<()> {
        let queue_path = root.join(".enc_queue");

        if !queue_path.exists() {
            return Ok(()); // No queue file, nothing to load
        }

        let file = File::open(&queue_path).context("Failed to open .enc_queue file")?;

        let reader = io::BufReader::new(file);
        let mut completed_files = std::collections::HashSet::new();

        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();

            if trimmed.starts_with('#') && !trimmed.starts_with("# VP9") {
                // Extract filename from completed line "# filename"
                let filename = trimmed.trim_start_matches('#').trim();
                // Remove any "(skipped - output exists)" suffix
                let filename = filename.split(" (skipped").next().unwrap_or(filename);
                completed_files.insert(filename.to_string());
            }
        }

        // Update job statuses based on completion tracking
        for job in &mut self.jobs {
            let filename = job
                .input_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            if completed_files.contains(filename) {
                job.status = JobStatus::Done;
                job.progress_pct = 100.0;
            }
        }

        Ok(())
    }
}
