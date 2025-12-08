// Statistics tracking and persistence

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LifetimeStats {
    /// Total bytes of all source files ever encoded
    pub total_input_bytes: u64,

    /// Total bytes of all encoded outputs
    pub total_output_bytes: u64,

    /// Total encoding time in seconds
    pub total_encode_time_secs: f64,

    /// Total number of completed jobs
    pub total_jobs_completed: u64,

    /// Total number of failed jobs
    pub total_jobs_failed: u64,

    /// Last updated timestamp (ISO 8601)
    pub last_updated: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SessionStats {
    /// Jobs completed this session
    pub jobs_done: usize,

    /// Jobs currently pending
    pub jobs_pending: usize,

    /// Jobs that failed this session
    pub jobs_failed: usize,

    /// Total input bytes this session
    pub input_bytes: u64,

    /// Total output bytes this session
    pub output_bytes: u64,

    /// Total encode time this session
    pub encode_time_secs: f64,

    /// Session start time
    pub session_start: Instant,
}

impl Default for SessionStats {
    fn default() -> Self {
        Self {
            jobs_done: 0,
            jobs_pending: 0,
            jobs_failed: 0,
            input_bytes: 0,
            output_bytes: 0,
            encode_time_secs: 0.0,
            session_start: Instant::now(),
        }
    }
}

impl SessionStats {
    /// Format space saved
    pub fn format_space_saved(&self) -> String {
        let space_saved = self.input_bytes as i64 - self.output_bytes as i64;
        if space_saved >= 0 {
            format!("{} saved", format_bytes(space_saved as u64))
        } else {
            format!("{} larger", format_bytes((-space_saved) as u64))
        }
    }
}

#[derive(Debug, Clone)]
pub struct StatsState {
    /// Lifetime statistics (loaded from disk, tracked but not displayed)
    pub lifetime: LifetimeStats,

    /// Current session statistics
    pub session: SessionStats,
}

impl Default for StatsState {
    fn default() -> Self {
        Self {
            lifetime: LifetimeStats::load().unwrap_or_default(),
            session: SessionStats::default(),
        }
    }
}

impl LifetimeStats {
    /// Get the path to the stats file
    pub fn stats_path() -> Result<PathBuf> {
        let config_dir = if cfg!(target_os = "macos") {
            dirs::home_dir()
                .context("Could not determine home directory")?
                .join(".config")
                .join("ffdash")
        } else if cfg!(target_os = "windows") {
            dirs::config_dir()
                .context("Could not determine config directory")?
                .join("ffdash")
        } else {
            // Linux and others
            dirs::config_dir()
                .context("Could not determine config directory")?
                .join("ffdash")
        };

        Ok(config_dir.join("stats.json"))
    }

    /// Load stats from disk, or return default if it doesn't exist
    pub fn load() -> Result<Self> {
        let stats_path = Self::stats_path()?;

        if stats_path.exists() {
            let contents = fs::read_to_string(&stats_path)
                .with_context(|| format!("Failed to read stats file: {}", stats_path.display()))?;

            let stats: LifetimeStats = serde_json::from_str(&contents)
                .with_context(|| format!("Failed to parse stats file: {}", stats_path.display()))?;

            Ok(stats)
        } else {
            Ok(LifetimeStats::default())
        }
    }

    /// Save stats to disk
    pub fn save(&self) -> Result<()> {
        let stats_path = Self::stats_path()?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = stats_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }

        let contents = serde_json::to_string_pretty(self).context("Failed to serialize stats")?;

        fs::write(&stats_path, contents)
            .with_context(|| format!("Failed to write stats file: {}", stats_path.display()))?;

        Ok(())
    }

    /// Calculate compression ratio (input / output)
    pub fn compression_ratio(&self) -> f64 {
        if self.total_output_bytes == 0 {
            0.0
        } else {
            self.total_input_bytes as f64 / self.total_output_bytes as f64
        }
    }

    /// Format total input size
    pub fn format_input_size(&self) -> String {
        format_bytes(self.total_input_bytes)
    }

    /// Format total output size
    pub fn format_output_size(&self) -> String {
        format_bytes(self.total_output_bytes)
    }

    /// Format total encode time
    pub fn format_encode_time(&self) -> String {
        format_duration(self.total_encode_time_secs)
    }
}

impl SessionStats {
    /// Calculate from job list
    pub fn from_jobs(jobs: &[crate::engine::VideoJob], session_start: Instant) -> Self {
        use crate::engine::JobStatus;

        let done_jobs: Vec<_> = jobs
            .iter()
            .filter(|j| j.status == JobStatus::Done)
            .collect();

        let jobs_done = done_jobs.len();
        let jobs_pending = jobs
            .iter()
            .filter(|j| j.status == JobStatus::Pending)
            .count();
        let jobs_failed = jobs
            .iter()
            .filter(|j| j.status == JobStatus::Failed)
            .count();

        // Calculate input/output bytes and encode time for completed jobs
        let mut input_bytes: u64 = 0;
        let mut output_bytes: u64 = 0;
        let mut encode_time_secs = 0.0;

        for job in done_jobs {
            // Get file sizes
            if let Ok(input_size) = std::fs::metadata(&job.input_path).map(|m| m.len()) {
                input_bytes += input_size;
                if let Ok(output_size) = std::fs::metadata(&job.output_path).map(|m| m.len()) {
                    output_bytes += output_size;
                }
            }

            // Calculate encode time
            if let Some(started_at) = job.started_at {
                encode_time_secs += started_at.elapsed().as_secs_f64();
            }
        }

        Self {
            jobs_done,
            jobs_pending,
            jobs_failed,
            input_bytes,
            output_bytes,
            encode_time_secs,
            session_start,
        }
    }

    /// Format encode time
    pub fn format_encode_time(&self) -> String {
        format_duration(self.encode_time_secs)
    }
}

/// Format bytes as human-readable size
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format duration in seconds as human-readable time
pub fn format_duration(seconds: f64) -> String {
    let total_secs = seconds as u64;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let secs = total_secs % 60;

    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
        assert_eq!(format_bytes(1099511627776), "1.00 TB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(0.0), "0s");
        assert_eq!(format_duration(45.0), "45s");
        assert_eq!(format_duration(90.0), "1m 30s");
        assert_eq!(format_duration(3600.0), "1h 0m");
        assert_eq!(format_duration(3665.0), "1h 1m");
    }

    #[test]
    fn test_compression_ratio() {
        let mut stats = LifetimeStats::default();
        stats.total_input_bytes = 1000;
        stats.total_output_bytes = 300;
        assert!((stats.compression_ratio() - 3.333).abs() < 0.01);

        // Test zero output
        stats.total_output_bytes = 0;
        assert_eq!(stats.compression_ratio(), 0.0);
    }
}
