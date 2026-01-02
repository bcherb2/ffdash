use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    Pending,
    Calibrating, // Running Auto-VMAF calibration
    Running,
    Done,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoJob {
    pub id: Uuid,
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub profile: String,
    pub status: JobStatus,

    #[serde(default)]
    pub overwrite: bool, // Whether to overwrite existing output file

    // Derived / runtime
    pub duration_s: Option<f64>,
    pub progress_pct: f64,
    pub out_time_s: f64,
    pub fps: Option<f64>,
    pub speed: Option<f64>,
    pub smoothed_speed: Option<f64>, // EWMA-smoothed speed for stable ETA
    pub bitrate_kbps: Option<f64>,
    pub size_bytes: Option<u64>,

    #[serde(skip)] // Don't serialize Instant
    pub started_at: Option<std::time::Instant>,

    #[serde(skip)] // Don't serialize Instant
    pub last_speed_update: Option<std::time::Instant>,

    #[serde(skip)] // Don't persist display state
    pub displayed_eta_seconds: Option<u64>,

    pub attempts: u32,
    pub last_error: Option<String>,

    // Auto-VMAF calibration results
    #[serde(default)]
    pub vmaf_target: Option<f32>, // Target VMAF score if Auto-VMAF enabled
    #[serde(default)]
    pub vmaf_result: Option<f32>, // Actual VMAF score achieved
    #[serde(default)]
    pub calibrated_quality: Option<u32>, // Calibrated quality setting (CRF or global_quality)
    #[serde(default)]
    pub vmaf_partial_scores: Vec<f32>, // Individual window scores for progressive averaging

    #[serde(skip)]
    pub calibrating_total_steps: Option<u32>, // Total calibration windows across attempts
    #[serde(skip)]
    pub calibrating_completed_steps: u32, // Completed calibration windows
}

impl VideoJob {
    /// Create a new pending job
    pub fn new(input_path: PathBuf, output_path: PathBuf, profile: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            input_path,
            output_path,
            profile,
            status: JobStatus::Pending,
            overwrite: false, // Default to no overwrite
            duration_s: None,
            progress_pct: 0.0,
            out_time_s: 0.0,
            fps: None,
            speed: None,
            smoothed_speed: None,
            bitrate_kbps: None,
            size_bytes: None,
            started_at: None,
            last_speed_update: None,
            displayed_eta_seconds: None,
            attempts: 0,
            last_error: None,
            vmaf_target: None,
            vmaf_result: None,
            calibrated_quality: None,
            vmaf_partial_scores: Vec::new(),
            calibrating_total_steps: None,
            calibrating_completed_steps: 0,
        }
    }
}

/// Parser for ffmpeg progress output (key=value format)
#[derive(Debug, Default, Clone)]
pub struct ProgressParser {
    pub out_time_us: u64,
    pub fps: Option<f64>,
    pub speed: Option<f64>,
    pub bitrate_kbps: Option<f64>,
    pub total_size: Option<u64>,
    pub is_complete: bool,
}

impl ProgressParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a single line of ffmpeg progress output
    pub fn parse_line(&mut self, line: &str) {
        if let Some((key, value)) = line.split_once('=') {
            match key.trim() {
                "out_time_us" => {
                    if let Ok(us) = value.trim().parse::<u64>() {
                        self.out_time_us = us;
                    }
                }
                "fps" => {
                    if let Ok(f) = value.trim().parse::<f64>() {
                        self.fps = Some(f);
                    }
                }
                "speed" => {
                    // Speed is in format "1.23x", strip the 'x'
                    let speed_str = value.trim().trim_end_matches('x');
                    if let Ok(s) = speed_str.parse::<f64>() {
                        self.speed = Some(s);
                    }
                }
                "bitrate" => {
                    // Bitrate is in format "123.4kbits/s", extract number
                    let bitrate_str = value.trim().trim_end_matches("kbits/s");
                    if let Ok(b) = bitrate_str.parse::<f64>() {
                        self.bitrate_kbps = Some(b);
                    }
                }
                "total_size" => {
                    if let Ok(size) = value.trim().parse::<u64>() {
                        self.total_size = Some(size);
                    }
                }
                "progress" => {
                    if value.trim() == "end" {
                        self.is_complete = true;
                    }
                }
                _ => {}
            }
        }
    }

    /// Get output time in seconds
    pub fn out_time_s(&self) -> f64 {
        self.out_time_us as f64 / 1_000_000.0
    }

    /// Calculate progress percentage given total duration
    pub fn progress_pct(&self, duration_s: Option<f64>) -> f64 {
        if let Some(dur) = duration_s {
            if dur > 0.0 {
                return (self.out_time_s() / dur * 100.0).min(100.0);
            }
        }
        0.0
    }
}
