//! VMAF evaluation for auto-quality calibration
//!
//! This module provides functionality to compute VMAF scores between source and encoded video,
//! which is used by the Auto-VAMF feature to calibrate encoding quality settings.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use crate::engine::core::{Codec, HwEncodingConfig, Profile, ProgressParser, write_debug_log};
use crate::engine::{JobStatus, VideoJob, probe_duration};

/// Quality floor for software encoding (CRF)
const SOFTWARE_QUALITY_FLOOR: u32 = 10;

/// Quality floor for hardware encoding (global_quality)
const HARDWARE_QUALITY_FLOOR: u32 = 5;

/// Result of quality calibration
#[derive(Debug, Clone)]
pub struct CalibrationResult {
    /// Final quality setting (CRF or global_quality)
    pub quality: u32,
    /// Measured minimum VMAF across all windows
    pub measured_vmaf: f32,
    /// Number of calibration attempts performed
    pub attempts: u8,
    /// True if we hit the quality floor without meeting target
    pub hit_floor: bool,
}

/// Check if ffmpeg has libvmaf filter available
///
/// This checks once and caches the result for the lifetime of the program.
/// Returns true if libvmaf is available, false otherwise.
pub fn vmaf_filter_available() -> bool {
    static VMAF_AVAILABLE: OnceLock<bool> = OnceLock::new();

    *VMAF_AVAILABLE.get_or_init(|| {
        let output = Command::new("ffmpeg")
            .args(["-filters"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let combined = format!("{}{}", stdout, stderr);
                combined.contains("libvmaf")
            }
            Err(_) => false,
        }
    })
}

/// Select appropriate VMAF model based on output resolution
///
/// Returns the model specification string for ffmpeg's libvmaf filter.
/// - For 1080p and below: uses standard vmaf_v0.6.1 model (1080p HDTV viewing distance)
/// - For 4K (2160p): uses vmaf_4k_v0.6.1 model (4K viewing distance)
pub fn select_vmaf_model(output_height: u32) -> &'static str {
    if output_height >= 2160 {
        "version=vmaf_4k_v0.6.1"
    } else {
        "version=vmaf_v0.6.1"
    }
}

/// VMAF JSON output structure (partial - we only need the pooled mean)
#[derive(Debug, Deserialize)]
struct VmafOutput {
    pooled_metrics: PooledMetrics,
}

#[derive(Debug, Deserialize)]
struct PooledMetrics {
    vmaf: VmafMetrics,
}

#[derive(Debug, Deserialize)]
struct VmafMetrics {
    mean: f64,
}

/// Parse VMAF score from JSON log file
///
/// Reads the JSON output from libvmaf and extracts the pooled mean VMAF score.
/// Returns an error if the file doesn't exist, is malformed, or missing expected fields.
pub fn parse_vmaf_score(log_path: &Path) -> Result<f32> {
    let content = std::fs::read_to_string(log_path)
        .with_context(|| format!("Failed to read VMAF log: {}", log_path.display()))?;

    let vmaf_output: VmafOutput = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse VMAF JSON: {}", log_path.display()))?;

    Ok(vmaf_output.pooled_metrics.vmaf.mean as f32)
}

/// Build ffmpeg command for VMAF evaluation
///
/// Constructs a command that compares a source video segment with an encoded version,
/// computing VMAF score and writing results to JSON.
///
/// # Arguments
/// * `source_path` - Path to original source video
/// * `encoded_path` - Path to encoded test window
/// * `window_start` - Start time of window in seconds
/// * `window_duration` - Duration of window in seconds
/// * `output_height` - Output video height (for model selection)
/// * `n_subsample` - Frame subsampling rate (e.g., 30 = evaluate every 30th frame)
/// * `log_path` - Where to write VMAF JSON results
///
/// # Returns
/// A configured Command ready to execute
#[allow(clippy::too_many_arguments)]
pub fn build_vmaf_cmd(
    source_path: &Path,
    encoded_path: &Path,
    window_start: f64,
    window_duration: f64,
    encode_fps: u32,
    output_height: u32,
    n_subsample: u32,
    log_path: &Path,
    hw_config: Option<&HwEncodingConfig>,
    use_hw_decode: bool,
) -> Command {
    let model = select_vmaf_model(output_height);

    // Build filtergraph for VMAF comparison
    // [0:v] = source (reference), [1:v] = encoded (distorted)
    // Normalize fps/scale on both legs to match the encoded output and avoid frame misalignment
    let mut norm_filters = Vec::new();
    if encode_fps > 0 {
        norm_filters.push(format!("fps=fps={}", encode_fps));
    }
    if output_height > 0 {
        norm_filters.push(format!("scale=-2:{}", output_height));
    }
    norm_filters.push("format=yuv420p".to_string());
    let norm = norm_filters.join(",");

    // Escape the log path for use in ffmpeg filter expressions
    // Special characters like spaces, colons, brackets need to be escaped
    let log_path_str = log_path.display().to_string();
    let escaped_log_path = log_path_str
        .replace('\\', "\\\\")
        .replace(':', "\\:")
        .replace(' ', "\\ ")
        .replace('[', "\\[")
        .replace(']', "\\]");

    let filtergraph = format!(
        "[0:v]{norm}[ref];\
         [1:v]{norm}[dist];\
         [dist][ref]libvmaf=model={model}:log_fmt=json:log_path={log}:n_subsample={sub}",
        norm = norm,
        model = model,
        log = escaped_log_path,
        sub = n_subsample,
    );

    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-hide_banner", "-y"]);

    // Try hardware decode if requested and hw_config is available
    if use_hw_decode && hw_config.is_some() {
        // Detect VAAPI configuration
        if let Some(config) = crate::engine::hardware::detect_vaapi_config() {
            // Set environment variables for driver
            cmd.env("LIBVA_DRIVERS_PATH", &config.driver.path);
            cmd.env("LIBVA_DRIVER_NAME", &config.driver.name);

            // Initialize hardware device
            cmd.arg("-init_hw_device")
                .arg(format!("vaapi=va:{}", config.render_device));

            // Enable hardware decode for both inputs
            cmd.arg("-hwaccel").arg("vaapi");

            let _ = write_debug_log("[VMAF] Using hardware decode (VAAPI)");
        } else {
            let _ = write_debug_log("[VMAF] VAAPI config not detected, using software decode");
        }
    } else {
        let _ = write_debug_log("[VMAF] Using software decode");
    }

    // Add source input with window timing (seek before decoding for accuracy)
    cmd.arg("-ss")
        .arg(window_start.to_string())
        .arg("-t")
        .arg(window_duration.to_string())
        .arg("-i")
        .arg(source_path);

    // Add encoded input (already windowed during encoding)
    cmd.arg("-i").arg(encoded_path);

    // Add filtergraph and disable vsync to avoid timestamp fiddling
    cmd.arg("-lavfi")
        .arg(&filtergraph)
        .arg("-vsync")
        .arg("0")
        .args(["-f", "null", "-"]);

    cmd
}

/// Run VMAF evaluation and return the score
///
/// This is a convenience function that:
/// 1. Builds the VMAF command
/// 2. Executes it
/// 3. Parses the result
/// 4. Cleans up the log file
///
/// # Arguments
/// * `source_path` - Original source video
/// * `encoded_path` - Encoded test window
/// * `window_start` - Start time in seconds
/// * `window_duration` - Duration in seconds
/// * `output_height` - Output height for model selection
/// * `n_subsample` - Frame subsampling rate
/// * `temp_dir` - Directory for temporary VMAF log
///
/// # Returns
/// The pooled mean VMAF score as f32
#[allow(clippy::too_many_arguments)]
pub fn run_vmaf_evaluation(
    source_path: &Path,
    encoded_path: &Path,
    window_start: f64,
    window_duration: f64,
    encode_fps: u32,
    output_height: u32,
    n_subsample: u32,
    temp_dir: &Path,
    hw_config: Option<&HwEncodingConfig>,
) -> Result<f32> {
    // Generate unique log filename
    let log_filename = format!("vmaf_{}.json", uuid::Uuid::new_v4());
    let log_path = temp_dir.join(log_filename);

    // Try hardware decode first if available, then fallback to software
    let use_hw_first = hw_config.is_some();

    for attempt in 0..2 {
        let use_hw_decode = use_hw_first && attempt == 0;

        // Build and run VMAF command
        let mut cmd = build_vmaf_cmd(
            source_path,
            encoded_path,
            window_start,
            window_duration,
            encode_fps,
            output_height,
            n_subsample,
            &log_path,
            hw_config,
            use_hw_decode,
        );

        // Log the VMAF command for debugging
        let cmd_str = format!(
            "{} {}",
            cmd.get_program().to_string_lossy(),
            cmd.get_args()
                .map(|s| s.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" ")
        );
        let _ = write_debug_log(&format!(
            "[VMAF] Attempt {} - Command: {}",
            attempt + 1,
            cmd_str
        ));

        let output = cmd.output().context("Failed to execute VMAF evaluation")?;

        if output.status.success() {
            // Parse result
            let score = parse_vmaf_score(&log_path)?;

            // Clean up log file (best effort)
            let _ = std::fs::remove_file(&log_path);

            return Ok(score);
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let _ = write_debug_log(&format!(
                "[VMAF] Attempt {} failed with stderr: {}",
                attempt + 1,
                stderr
            ));

            // If this was hardware decode attempt and we can fallback, try again with software
            if use_hw_decode && use_hw_first {
                let _ = write_debug_log(
                    "[VMAF] Hardware decode failed, falling back to software decode",
                );
                continue;
            }

            // No more attempts, return error
            anyhow::bail!("VMAF evaluation failed: {}", stderr);
        }
    }

    anyhow::bail!("VMAF evaluation failed after all attempts")
}

/// Calculate number of windows based on video duration
///
/// Dynamically scales window count based on video length:
/// - < 5 minutes: 1 window (quick sample)
/// - 5-90 minutes: Linear scale from 1 to 5 windows
/// - >= 90 minutes: 5 windows (cap for long movies)
///
/// This approach balances accuracy with encoding time:
/// - Short clips don't need extensive sampling
/// - Long movies get thorough multi-point analysis
///
/// # Arguments
/// * `duration_s` - Total video duration in seconds
/// * `max_windows` - Maximum windows allowed by budget
/// * `possible_windows` - Maximum windows that fit in video length
///
/// # Returns
/// Optimal window count respecting all constraints
fn calculate_dynamic_window_count(
    duration_s: f64,
    max_windows: usize,
    possible_windows: usize,
) -> usize {
    const MIN_DURATION: f64 = 5.0 * 60.0; // 5 minutes
    const MAX_DURATION: f64 = 90.0 * 60.0; // 90 minutes
    const MIN_WINDOWS: usize = 1;
    const MAX_WINDOWS: usize = 5;

    let ideal_count = if duration_s < MIN_DURATION {
        MIN_WINDOWS
    } else if duration_s >= MAX_DURATION {
        MAX_WINDOWS
    } else {
        // Linear interpolation between 1 and 5
        let ratio = (duration_s - MIN_DURATION) / (MAX_DURATION - MIN_DURATION);
        let scaled = MIN_WINDOWS as f64 + ratio * (MAX_WINDOWS - MIN_WINDOWS) as f64;
        scaled.round() as usize
    };

    // Respect budget and file length constraints
    ideal_count.min(max_windows).min(possible_windows)
}

/// Select analysis windows for VMAF evaluation
///
/// Chooses representative time segments from the video to sample for quality measurement.
/// Strategy: start, middle, end + optional random positions if budget allows.
///
/// # Arguments
/// * `duration_s` - Total video duration in seconds
/// * `window_duration` - Duration of each window in seconds
/// * `budget_sec` - Total seconds to sample (sum of all windows)
///
/// # Returns
/// Vector of (start_time, duration) pairs in seconds
///
/// # Examples
/// - 10min video, 10s windows, 60s budget → 2 windows (scaled for duration)
/// - 30s video, 10s windows, 60s budget → 1 window (short video)
/// - 90min video, 10s windows, 60s budget → 5 windows (long movie)
pub fn select_windows(duration_s: f64, window_duration: u32, budget_sec: u32) -> Vec<(f64, f64)> {
    let window_dur = window_duration as f64;
    let max_windows = (budget_sec as f64 / window_dur).floor() as usize;

    // Handle very short files
    if duration_s < window_dur {
        // File is shorter than one window - use entire file
        return vec![(0.0, duration_s)];
    }

    // Calculate how many windows we can fit
    let possible_windows = (duration_s / window_dur).floor() as usize;
    let num_windows = calculate_dynamic_window_count(duration_s, max_windows, possible_windows);

    if num_windows == 0 {
        return Vec::new();
    }

    let mut windows = Vec::with_capacity(num_windows);

    match num_windows {
        1 => {
            // Just start
            windows.push((5.0, window_dur));
        }
        2 => {
            // Start and end
            windows.push((5.0, window_dur));
            windows.push(((duration_s - window_dur - 5.0).max(10.0), window_dur));
        }
        3 => {
            // Start, middle, end
            windows.push((5.0, window_dur));
            let mid = (duration_s / 2.0) - (window_dur / 2.0);
            windows.push((mid.max(5.0), window_dur));
            windows.push((
                (duration_s - window_dur - 5.0).max(mid + window_dur),
                window_dur,
            ));
        }
        n => {
            // Start, middle, end + (n-3) random positions
            windows.push((5.0, window_dur));
            let mid = (duration_s / 2.0) - (window_dur / 2.0);
            windows.push((mid.max(5.0), window_dur));
            windows.push((
                (duration_s - window_dur - 5.0).max(mid + window_dur),
                window_dur,
            ));

            // Add random positions between start and end
            let extra_count = n - 3;
            let segment_size = (duration_s - 10.0 - window_dur) / (extra_count + 1) as f64;

            for i in 1..=extra_count {
                let pos = 5.0 + (segment_size * i as f64);
                windows.push((pos, window_dur));
            }

            // Sort by start time
            windows.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        }
    }

    windows
}

/// Create temporary directory for job-specific files
///
/// Creates `.ffdash_tmp/<job_id>/` under the parent directory of the input file.
/// This keeps temp files alongside source, avoiding cross-filesystem moves.
///
/// # Arguments
/// * `job` - The video job being processed
///
/// # Returns
/// Path to the created temp directory
pub fn create_job_temp_dir(job: &VideoJob) -> Result<PathBuf> {
    let parent = job.input_path.parent().unwrap_or_else(|| Path::new("."));

    let temp_root = parent.join(".ffdash_tmp");
    let job_temp = temp_root.join(job.id.to_string());

    std::fs::create_dir_all(&job_temp)
        .with_context(|| format!("Failed to create temp dir: {}", job_temp.display()))?;

    Ok(job_temp)
}

/// Clean up temporary directory for a job
///
/// Removes the job's temp directory and all contents.
/// This is best-effort - failures are logged but not fatal.
///
/// # Arguments
/// * `path` - Path to the temp directory to remove
///
/// # Returns
/// Ok(()) if successful, or a non-fatal error
pub fn cleanup_job_temp_dir(path: &Path) -> Result<()> {
    if path.exists() {
        std::fs::remove_dir_all(path)
            .with_context(|| format!("Failed to cleanup temp dir: {}", path.display()))?;
    }
    Ok(())
}

/// Build ffmpeg command for encoding a single window
///
/// Wraps the existing software/hardware encoding logic with time-range restrictions
/// and quality overrides for test encoding.
///
/// # Arguments
/// * `job` - The video job
/// * `profile` - Encoding profile
/// * `hw_config` - Optional hardware config
/// * `window_start` - Start time in seconds
/// * `window_duration` - Duration in seconds
/// * `quality` - CRF (software) or global_quality (hardware) override
/// * `output_path` - Where to write the encoded window
///
/// # Returns
/// Configured Command ready to execute
pub fn build_window_encode_cmd(
    job: &VideoJob,
    profile: &Profile,
    hw_config: Option<&HwEncodingConfig>,
    window_start: f64,
    window_duration: f64,
    quality: u32,
    output_path: &Path,
) -> Command {
    // Clone profile and override quality
    let mut test_profile = profile.clone();

    // Disable scaling for VMAF window encoding - we need to encode at source resolution
    // to properly compare against the original. VMAF requires matching dimensions.
    test_profile.scale_width = 0;
    test_profile.scale_height = 0;

    match (hw_config.is_some(), &mut test_profile.codec) {
        (true, Codec::Av1(av1)) => {
            av1.hw_cq = quality;
        }
        (true, Codec::Vp9(_)) => {
            test_profile.hw_global_quality = quality;
        }
        (false, _) => {
            test_profile.crf = quality;
        }
    }

    // Clone hw_config and override quality (hw_config.global_quality is what FFmpeg uses)
    let test_hw_config = hw_config.map(|hw| {
        let mut config = hw.clone();
        config.global_quality = quality;
        config
    });

    // Create a temporary job for the window
    let mut window_job = job.clone();
    window_job.output_path = output_path.to_path_buf();

    // Build the base command using codec-aware builders
    // This will automatically select the correct encoder based on codec type:
    // - VP9: vp9_vaapi (hw) or libvpx-vp9 (sw)
    // - AV1: av1_vaapi (hw) or libsvtav1 (sw)
    let cmd = crate::engine::build_ffmpeg_cmd_with_profile(
        &window_job,
        test_hw_config.as_ref(),
        Some(&test_profile),
    );

    // Get environment variables (preserve VAAPI driver settings)
    let env_vars: Vec<_> = cmd
        .get_envs()
        .map(|(k, v)| (k.to_os_string(), v.map(|s| s.to_os_string())))
        .collect();

    // Process args: insert -ss/-t before -i AND remove hardware decode flags
    let args: Vec<std::ffi::OsString> = cmd.get_args().map(|s| s.to_os_string()).collect();
    let mut new_args: Vec<std::ffi::OsString> = Vec::new();

    // Detect encoder to decide whether to strip hwaccel. For av1_qsv we keep hwaccel
    // to match the main encode pipeline (QSV surfaces end-to-end).
    let mut encoder: Option<String> = None;
    let mut enc_idx = 0;
    while enc_idx + 1 < args.len() {
        if args[enc_idx].to_string_lossy() == "-c:v" {
            encoder = Some(args[enc_idx + 1].to_string_lossy().into_owned());
            break;
        }
        enc_idx += 1;
    }
    let keep_hwaccel = encoder.as_deref() == Some("av1_qsv");

    let upload_pix_fmt = if test_profile.pix_fmt == "yuv420p10le" {
        "p010le"
    } else {
        "nv12"
    };
    let upload_filter = format!("format={},hwupload", upload_pix_fmt);

    let mut removed_hwaccel = false;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        let arg_str = arg.to_string_lossy();

        // Skip hardware decode flags (but keep -init_hw_device for encoder)
        if arg_str == "-hwaccel" && !keep_hwaccel {
            // Skip -hwaccel and its value
            removed_hwaccel = true;
            i += 2;
            continue;
        } else if arg_str == "-hwaccel_output_format" && !keep_hwaccel {
            // Skip -hwaccel_output_format and its value
            i += 2;
            continue;
        }

        // When we hit -i, insert seek params first
        if arg_str == "-i" {
            new_args.push("-ss".into());
            new_args.push(window_start.to_string().into());
            new_args.push("-t".into());
            new_args.push(window_duration.to_string().into());
        }

        // Add current arg
        new_args.push(arg.clone());

        // After adding -i and its value, add hwupload filter if we removed hwaccel
        // This uploads software-decoded frames to VAAPI hardware for encoding
        if arg_str == "-i" && i + 1 < args.len() {
            // Add the input path
            i += 1;
            new_args.push(args[i].clone());

            // If we removed hwaccel, we need to upload frames to GPU for hw encoding
            if removed_hwaccel {
                new_args.push("-vf".into());
                new_args.push(upload_filter.clone().into());
            }
        }

        i += 1;
    }

    // Merge multiple -vf/-filter:v occurrences into a single filter chain.
    // Auto-VAMF injects an upload filter when stripping hwaccel; the base command
    // may already have a -vf (e.g., vpp_qsv=format=...). Combine them to avoid
    // "Multiple -vf" warnings and to preserve filter ordering.
    let mut filter_indices: Vec<usize> = Vec::new();
    let mut filter_strings: Vec<String> = Vec::new();
    let mut idx = 0;
    while idx < new_args.len() {
        let arg = &new_args[idx];
        let arg_str = arg.to_string_lossy();
        if (arg_str == "-vf" || arg_str == "-filter:v") && idx + 1 < new_args.len() {
            filter_indices.push(idx);
            filter_strings.push(new_args[idx + 1].to_string_lossy().into_owned());
            idx += 2;
            continue;
        }
        idx += 1;
    }

    if filter_strings.len() > 1 {
        let merged = filter_strings.join(",");
        let first = filter_indices[0];
        if first + 1 < new_args.len() {
            new_args[first + 1] = merged.clone().into();
        }
        // Remove subsequent -vf/-filter:v pairs in reverse order
        for fi in filter_indices.iter().skip(1).rev() {
            let i = *fi;
            if i + 1 < new_args.len() {
                new_args.remove(i + 1);
            }
            new_args.remove(i);
        }
    }

    // Ensure filters use the QSV device explicitly when hardware encoding is enabled.
    // This avoids ffmpeg picking an unintended device during calibration.
    let mut has_filter_hw_device = false;
    let mut j = 0;
    while j + 1 < new_args.len() {
        if new_args[j].to_string_lossy() == "-filter_hw_device" {
            has_filter_hw_device = true;
            break;
        }
        j += 1;
    }
    if test_profile.use_hardware_encoding && !has_filter_hw_device {
        new_args.insert(0, "qs".into());
        new_args.insert(0, "-filter_hw_device".into());
    }

    // For calibration, keep it video-only to avoid audio/subtitle stalls.
    // Insert -an -sn before the output path (last arg).
    if !new_args.is_empty() {
        let out_idx = new_args.len().saturating_sub(1);
        new_args.insert(out_idx, "-sn".into());
        new_args.insert(out_idx, "-an".into());
    }

    // Rebuild command with filtered args and preserved env vars
    let mut new_cmd = Command::new("ffmpeg");
    new_cmd.args(new_args);
    for (k, v) in env_vars {
        if let Some(val) = v {
            new_cmd.env(k, val);
        }
    }

    new_cmd
}

/// Encode a single window and return the output path
///
/// Runs the window encode command to completion and verifies output exists.
///
/// # Arguments
/// * `job` - The video job
/// * `profile` - Encoding profile
/// * `hw_config` - Optional hardware config
/// * `window` - (start_time, duration) tuple
/// * `quality` - Quality setting to use
/// * `temp_dir` - Directory for output
///
/// # Returns
/// Path to the encoded window file
pub fn encode_window(
    job: &VideoJob,
    profile: &Profile,
    hw_config: Option<&HwEncodingConfig>,
    window: (f64, f64),
    quality: u32,
    temp_dir: &Path,
) -> Result<PathBuf> {
    let (start, duration) = window;

    // Generate output filename
    // Use same container extension as the main output to mirror the actual encode path
    let ext = std::path::Path::new(&job.output_path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("mkv");
    let output_filename = format!("win_{}_{:.1}s_q{}.{}", job.id, start, quality, ext);
    let output_path = temp_dir.join(output_filename);

    // Build and run encode command
    let mut cmd = build_window_encode_cmd(
        job,
        profile,
        hw_config,
        start,
        duration,
        quality,
        &output_path,
    );

    // Debug: log the actual command being executed
    let cmd_str = format!(
        "{} {}",
        cmd.get_program().to_string_lossy(),
        cmd.get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ")
    );
    let _ = write_debug_log(&format!("[Auto-VAMF] Window encode command: {}", cmd_str));

    let output = cmd.output().context("Failed to execute window encode")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = write_debug_log(&format!(
            "[Auto-VAMF] Window encode FAILED. stderr:\n{}",
            stderr
        ));
        anyhow::bail!("Window encode failed: {}", stderr);
    }

    // Verify output exists
    if !output_path.exists() {
        anyhow::bail!("Window encode did not produce output file");
    }

    Ok(output_path)
}

/// Check if profile is compatible with Auto-VAMF
///
/// Auto-VAMF only supports quality-based encoding modes:
/// - Software: CQ mode only
/// - Hardware: CQP or ICQ modes only
///
/// # Arguments
/// * `profile` - The encoding profile to check
///
/// # Returns
/// True if the profile can use Auto-VAMF, false otherwise
pub fn is_vmaf_compatible(profile: &Profile) -> bool {
    if profile.use_hardware_encoding {
        // Hardware: only CQP mode (mode 1)
        // Note: ICQ/VBR/CBR removed due to Intel Arc driver bugs
        profile.hw_rc_mode == 1
    } else {
        // Software: CQ or CQCap mode (quality-driven, max_bitrate is optional ceiling)
        profile.video_target_bitrate == 0
    }
}

/// Get baseline quality from profile
///
/// Extracts the appropriate quality setting based on encoder type.
///
/// # Arguments
/// * `profile` - The encoding profile
///
/// # Returns
/// The baseline quality value (CRF or global_quality)
fn get_baseline_quality(profile: &Profile) -> u32 {
    if profile.use_hardware_encoding {
        match &profile.codec {
            Codec::Av1(av1) => av1.hw_cq,
            Codec::Vp9(_) => profile.hw_global_quality,
        }
    } else {
        profile.crf
    }
}

/// Get quality floor for the encoder type
///
/// Returns the minimum quality setting we'll try.
/// Going lower than this produces diminishing returns.
///
/// # Arguments
/// * `profile` - The encoding profile
///
/// # Returns
/// The quality floor value
fn get_quality_floor(profile: &Profile) -> u32 {
    if profile.use_hardware_encoding {
        HARDWARE_QUALITY_FLOOR
    } else {
        SOFTWARE_QUALITY_FLOOR
    }
}

/// Calibrate encoding quality to meet target VMAF
///
/// This is the main Auto-VAMF algorithm. It:
/// 1. Selects representative windows from the video
/// 2. Iteratively encodes windows at different quality settings
/// 3. Measures VMAF for each window
/// 4. Stops when target VMAF is met or quality floor is reached
///
/// # Algorithm
/// - Start at user's baseline quality
/// - If min_vmaf < target: increase quality (lower CRF/global_quality)
/// - Repeat until target met or floor reached
/// - Maximum of `vmaf_max_attempts` iterations
///
/// # Arguments
/// * `job` - The video job to calibrate (mutable for real-time updates)
/// * `profile` - Encoding profile with VMAF settings
/// * `hw_config` - Optional hardware encoding config
/// * `callback` - Progress callback for real-time UI updates
///
/// # Returns
/// CalibrationResult with final quality and measured VMAF
///
/// # Errors
/// Returns error if:
/// - VMAF filter not available
/// - Profile incompatible with Auto-VAMF
/// - Duration probe fails
/// - Window encoding or VMAF evaluation fails
pub fn calibrate_quality<F>(
    job: &mut VideoJob,
    profile: &Profile,
    hw_config: Option<&HwEncodingConfig>,
    callback: &mut F,
) -> Result<CalibrationResult>
where
    F: FnMut(&VideoJob, &ProgressParser),
{
    // Pre-flight checks
    if !vmaf_filter_available() {
        anyhow::bail!("VMAF filter not available in ffmpeg");
    }

    if !is_vmaf_compatible(profile) {
        anyhow::bail!("Profile rate control mode not compatible with Auto-VAMF (use CQ/CQP/ICQ)");
    }

    // Probe duration
    let duration = probe_duration(&job.input_path)
        .with_context(|| format!("Failed to probe duration for {}", job.input_path.display()))?;

    if duration < 1.0 {
        anyhow::bail!("Video too short for Auto-VAMF calibration (< 1s)");
    }

    // Select windows
    let windows = select_windows(
        duration,
        profile.vmaf_window_duration_sec,
        profile.vmaf_analysis_budget_sec,
    );

    if windows.is_empty() {
        anyhow::bail!("No valid windows selected for calibration");
    }

    let _ = write_debug_log(&format!(
        "[Auto-VAMF] Selected {} windows for calibration ({}s budget)",
        windows.len(),
        profile.vmaf_analysis_budget_sec
    ));

    // Create temp directory
    let temp_dir = create_job_temp_dir(job)?;

    // Get calibration parameters
    let mut quality = get_baseline_quality(profile);
    let quality_floor = get_quality_floor(profile);
    let target_vmaf = profile.vmaf_target;
    let step = profile.vmaf_step as u32;
    let max_attempts = profile.vmaf_max_attempts;

    // Determine output height for VMAF model selection
    let output_height = if profile.scale_height > 0 {
        profile.scale_height as u32
    } else {
        // Use 1080 as default if source resolution
        1080
    };

    let _ = write_debug_log(&format!(
        "[Auto-VAMF] Starting calibration - baseline quality={}, target VMAF={}, floor={}, step={}",
        quality, target_vmaf, quality_floor, step
    ));

    // Track calibration progress (windows * attempts)
    let total_steps = (max_attempts as usize * windows.len()) as u32;
    job.calibrating_total_steps = Some(total_steps);
    job.calibrating_completed_steps = 0;
    job.progress_pct = 0.0;
    job.status = JobStatus::Calibrating;
    callback(job, &ProgressParser::new());

    let mut result = CalibrationResult {
        quality,
        measured_vmaf: 0.0,
        attempts: 0,
        hit_floor: false,
    };

    // Calibration loop
    for attempt in 1..=max_attempts {
        result.attempts = attempt;

        let _ = write_debug_log(&format!(
            "[Auto-VAMF] Attempt {}/{} with quality={}",
            attempt, max_attempts, quality
        ));

        // Clear partial scores for new attempt (keep vmaf_result showing last value)
        job.vmaf_partial_scores.clear();
        // Note: Don't clear vmaf_result here - keep showing last attempt's result
        // until first window of new attempt completes

        let mut window_scores = Vec::with_capacity(windows.len());

        // Encode and evaluate each window
        for (idx, &window) in windows.iter().enumerate() {
            let (start, duration) = window;

            let _ = write_debug_log(&format!(
                "[Auto-VAMF] Encoding window {} at {:.1}s ({}s)",
                idx + 1,
                start,
                duration
            ));

            // Encode window
            let encoded_path = encode_window(job, profile, hw_config, window, quality, &temp_dir)
                .with_context(|| {
                format!("Failed to encode window {} at quality {}", idx + 1, quality)
            })?;

            // Evaluate VMAF
            let vmaf_score = run_vmaf_evaluation(
                &job.input_path,
                &encoded_path,
                start,
                duration,
                profile.fps,
                output_height,
                profile.vmaf_n_subsample,
                &temp_dir,
                hw_config,
            )
            .with_context(|| format!("Failed to evaluate VMAF for window {}", idx + 1))?;

            let _ = write_debug_log(&format!(
                "[Auto-VAMF] Window {} VMAF = {:.2}",
                idx + 1,
                vmaf_score
            ));

            window_scores.push(vmaf_score);

            // Update partial scores and running average for real-time UI display
            job.vmaf_partial_scores.push(vmaf_score);
            let running_avg: f32 =
                job.vmaf_partial_scores.iter().sum::<f32>() / job.vmaf_partial_scores.len() as f32;
            job.vmaf_result = Some(running_avg);

            // Update calibration progress (based on completed windows)
            if let Some(total) = job.calibrating_total_steps {
                job.calibrating_completed_steps = job.calibrating_completed_steps.saturating_add(1);
                let completed = job.calibrating_completed_steps.min(total);
                job.progress_pct = ((completed as f64 / total as f64) * 100.0).min(100.0);
            }

            // Notify UI of progress update
            callback(job, &ProgressParser::new());

            // Clean up encoded window (best effort)
            let _ = std::fs::remove_file(&encoded_path);
        }

        // Aggregate scores (use average of windows)
        let sum_vmaf: f32 = window_scores.iter().copied().sum();
        let avg_vmaf: f32 = if !window_scores.is_empty() {
            sum_vmaf / window_scores.len() as f32
        } else {
            0.0
        };

        result.measured_vmaf = avg_vmaf;

        let _ = write_debug_log(&format!(
            "[Auto-VAMF] Attempt {} complete - avg VMAF = {:.2} (target = {:.2})",
            attempt, avg_vmaf, target_vmaf
        ));

        // Check if target met
        if avg_vmaf >= target_vmaf {
            let _ = write_debug_log(&format!(
                "[Auto-VAMF] Target met! Quality {} achieves VMAF {:.2}",
                quality, avg_vmaf
            ));
            result.quality = quality;
            break;
        }

        // Check if we hit quality floor
        if quality <= quality_floor {
            let _ = write_debug_log(&format!(
                "[Auto-VAMF] WARNING: Quality floor ({}) reached without meeting target. Best VMAF: {:.2}",
                quality_floor, avg_vmaf
            ));
            result.quality = quality;
            result.hit_floor = true;
            break;
        }

        // Check if this is the last attempt
        if attempt == max_attempts {
            let _ = write_debug_log(&format!(
                "[Auto-VAMF] WARNING: Max attempts ({}) reached. Best VMAF: {:.2} at quality {}",
                max_attempts, avg_vmaf, quality
            ));
            result.quality = quality;
            break;
        }

        // Increase quality for next iteration (lower CRF/global_quality = higher quality)
        let new_quality = quality.saturating_sub(step);
        quality = new_quality.max(quality_floor);

        let _ = write_debug_log(&format!(
            "[Auto-VAMF] Increasing quality to {} for next attempt",
            quality
        ));
    }

    // Cleanup temp directory (best effort)
    let _ = cleanup_job_temp_dir(&temp_dir);

    // Mark calibration complete for UI; encoding will reset progress/status
    job.progress_pct = 100.0;
    job.calibrating_total_steps = None;
    job.calibrating_completed_steps = 0;

    let _ = write_debug_log(&format!(
        "[Auto-VAMF] Calibration complete - quality={}, VMAF={:.2}, attempts={}, hit_floor={}",
        result.quality, result.measured_vmaf, result.attempts, result.hit_floor
    ));

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vmaf_model_selection() {
        assert_eq!(select_vmaf_model(1080), "version=vmaf_v0.6.1");
        assert_eq!(select_vmaf_model(1920), "version=vmaf_v0.6.1");
        assert_eq!(select_vmaf_model(2160), "version=vmaf_4k_v0.6.1");
        assert_eq!(select_vmaf_model(3840), "version=vmaf_4k_v0.6.1");
    }

    #[test]
    fn test_vmaf_filter_check() {
        // This will check if libvmaf is available on the system
        // The result depends on the ffmpeg build
        let _ = vmaf_filter_available();
        // Just ensure it doesn't panic
    }

    #[test]
    fn test_window_selection_short_file() {
        // File shorter than window duration
        let windows = select_windows(5.0, 10, 60);
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0], (0.0, 5.0));
    }

    #[test]
    fn test_window_selection_basic() {
        // 10 minute video, 10s windows, 60s budget
        // With dynamic scaling: 10min < 5min threshold → 1 window
        let windows = select_windows(600.0, 10, 60);
        assert_eq!(windows.len(), 1);
        // Should have single window near start
        assert!(windows[0].0 < 10.0); // Start near beginning
    }

    #[test]
    fn test_window_selection_long_movie() {
        // 90 minute movie, 10s windows, 60s budget
        // With dynamic scaling: 90min → 5 windows (max)
        let windows = select_windows(90.0 * 60.0, 10, 60);
        assert_eq!(windows.len(), 5);
        // Should have start, mid, end + extras
        assert!(windows[0].0 < 10.0); // Start near beginning
        assert!(windows.last().unwrap().0 > 5380.0); // End near... end
    }

    #[test]
    fn test_window_selection_short_clip() {
        // 2 minute video, 10s windows, 30s budget
        // With dynamic scaling: 2min < 5min threshold → 1 window
        let windows = select_windows(120.0, 10, 30);
        assert_eq!(windows.len(), 1);
        // Should have single window near start
        assert!(windows[0].0 < 10.0);
    }

    #[test]
    fn test_dynamic_window_count() {
        // < 5 min → 1 window
        assert_eq!(calculate_dynamic_window_count(3.0 * 60.0, 10, 10), 1);
        assert_eq!(calculate_dynamic_window_count(4.5 * 60.0, 10, 10), 1);

        // 5 min → 1 window (at threshold)
        assert_eq!(calculate_dynamic_window_count(5.0 * 60.0, 10, 10), 1);

        // 47.5 min (midpoint between 5 and 90) → 3 windows
        assert_eq!(calculate_dynamic_window_count(47.5 * 60.0, 10, 10), 3);

        // 90 min → 5 windows
        assert_eq!(calculate_dynamic_window_count(90.0 * 60.0, 10, 10), 5);

        // 120 min → 5 windows (capped)
        assert_eq!(calculate_dynamic_window_count(120.0 * 60.0, 10, 10), 5);

        // Budget limit: only 2 windows possible, duration suggests 5
        assert_eq!(calculate_dynamic_window_count(90.0 * 60.0, 2, 10), 2);

        // File length limit: only 3 windows fit, duration suggests 5
        assert_eq!(calculate_dynamic_window_count(90.0 * 60.0, 10, 3), 3);

        // Very short video
        assert_eq!(calculate_dynamic_window_count(30.0, 10, 10), 1);

        // 10 minute video → 2 windows (10min is 1/8 between 5 and 90, so ~1.5 rounds to 2)
        assert_eq!(calculate_dynamic_window_count(10.0 * 60.0, 10, 10), 1);

        // 30 minute video → 2 windows
        assert_eq!(calculate_dynamic_window_count(30.0 * 60.0, 10, 10), 2);

        // 60 minute video → 4 windows
        assert_eq!(calculate_dynamic_window_count(60.0 * 60.0, 10, 10), 4);
    }

    #[test]
    fn test_window_selection_budget_limited() {
        // Large video but small budget
        let windows = select_windows(3600.0, 10, 30);
        assert_eq!(windows.len(), 3); // Budget only allows 3 windows
    }

    #[test]
    fn test_is_vmaf_compatible_software() {
        let mut profile = Profile::get("vp9-good");

        // CQ mode (no bitrate targets) - compatible
        profile.use_hardware_encoding = false;
        profile.video_target_bitrate = 0;
        profile.video_max_bitrate = 0;
        assert!(is_vmaf_compatible(&profile));

        // VBR mode - not compatible
        profile.video_target_bitrate = 5000;
        assert!(!is_vmaf_compatible(&profile));

        // CQCap mode (CQ with max bitrate ceiling) - compatible
        profile.video_target_bitrate = 0;
        profile.video_max_bitrate = 8000;
        assert!(is_vmaf_compatible(&profile));
    }

    #[test]
    fn test_is_vmaf_compatible_hardware() {
        let mut profile = Profile::get("vp9-good");
        profile.use_hardware_encoding = true;

        // CQP mode (rc_mode = 1) - compatible (only supported mode)
        profile.hw_rc_mode = 1;
        assert!(is_vmaf_compatible(&profile));

        // CBR mode (rc_mode = 2) - not compatible
        profile.hw_rc_mode = 2;
        assert!(!is_vmaf_compatible(&profile));

        // VBR mode (rc_mode = 3) - not compatible
        profile.hw_rc_mode = 3;
        assert!(!is_vmaf_compatible(&profile));

        // ICQ mode (rc_mode = 4) - not compatible (removed due to driver bugs)
        profile.hw_rc_mode = 4;
        assert!(!is_vmaf_compatible(&profile));
    }

    #[test]
    fn test_get_baseline_quality() {
        let mut profile = Profile::get("vp9-good");

        // Software
        profile.use_hardware_encoding = false;
        profile.crf = 30;
        assert_eq!(get_baseline_quality(&profile), 30);

        // Hardware
        profile.use_hardware_encoding = true;
        profile.hw_global_quality = 70;
        assert_eq!(get_baseline_quality(&profile), 70);
    }

    #[test]
    fn test_get_quality_floor() {
        let mut profile = Profile::get("vp9-good");

        // Software floor
        profile.use_hardware_encoding = false;
        assert_eq!(get_quality_floor(&profile), SOFTWARE_QUALITY_FLOOR);

        // Hardware floor
        profile.use_hardware_encoding = true;
        assert_eq!(get_quality_floor(&profile), HARDWARE_QUALITY_FLOOR);
    }

    // === Temp Directory Tests ===

    #[test]
    fn test_create_job_temp_dir() {
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("video.mp4");
        std::fs::write(&input, b"fake").unwrap();

        let job = crate::engine::VideoJob::new(
            input.clone(),
            temp.path().join("out.webm"),
            "vp9-good".to_string(),
        );
        let result = create_job_temp_dir(&job);

        assert!(result.is_ok());
        let job_dir = result.unwrap();
        assert!(job_dir.exists());
        assert!(job_dir.to_string_lossy().contains(&job.id.to_string()));
    }

    #[test]
    fn test_create_job_temp_dir_nested_input() {
        let temp = tempfile::tempdir().unwrap();
        let nested = temp.path().join("some").join("nested").join("path");
        std::fs::create_dir_all(&nested).unwrap();
        let input = nested.join("video.mp4");
        std::fs::write(&input, b"fake").unwrap();

        let job = crate::engine::VideoJob::new(
            input.clone(),
            nested.join("out.webm"),
            "vp9-good".to_string(),
        );
        let result = create_job_temp_dir(&job);

        assert!(result.is_ok());
        let job_dir = result.unwrap();
        assert!(job_dir.exists());
        // Should be under the nested path, not temp root
        assert!(job_dir.starts_with(&nested));
    }

    #[test]
    fn test_cleanup_job_temp_dir() {
        let temp = tempfile::tempdir().unwrap();
        let job_dir = temp.path().join("job123");
        std::fs::create_dir(&job_dir).unwrap();

        assert!(job_dir.exists());
        let result = cleanup_job_temp_dir(&job_dir);
        assert!(result.is_ok());
        assert!(!job_dir.exists());
    }

    #[test]
    fn test_cleanup_job_temp_dir_nonexistent() {
        // Cleaning up a nonexistent path should succeed
        let result = cleanup_job_temp_dir(Path::new("/tmp/nonexistent_vmaf_test_dir_12345"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cleanup_job_temp_dir_with_files() {
        let temp = tempfile::tempdir().unwrap();
        let job_dir = temp.path().join("job456");
        std::fs::create_dir(&job_dir).unwrap();
        std::fs::write(job_dir.join("window1.webm"), b"data").unwrap();
        std::fs::write(job_dir.join("vmaf.json"), b"{}").unwrap();

        assert!(job_dir.exists());
        let result = cleanup_job_temp_dir(&job_dir);
        assert!(result.is_ok());
        assert!(!job_dir.exists());
    }

    // === VMAF JSON Parsing Tests ===

    #[test]
    fn test_parse_vmaf_score_valid() {
        let temp = tempfile::tempdir().unwrap();
        let log = temp.path().join("vmaf.json");
        std::fs::write(
            &log,
            r#"{
            "pooled_metrics": {
                "vmaf": { "mean": 93.5, "min": 88.2, "max": 98.1 }
            }
        }"#,
        )
        .unwrap();

        let score = parse_vmaf_score(&log).unwrap();
        assert!((score - 93.5).abs() < 0.01);
    }

    #[test]
    fn test_parse_vmaf_score_missing_file() {
        let result = parse_vmaf_score(Path::new("/tmp/nonexistent_vmaf_test_12345.json"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to read"));
    }

    #[test]
    fn test_parse_vmaf_score_invalid_json() {
        let temp = tempfile::tempdir().unwrap();
        let log = temp.path().join("bad.json");
        std::fs::write(&log, "not valid json at all").unwrap();

        let result = parse_vmaf_score(&log);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to parse"));
    }

    #[test]
    fn test_parse_vmaf_score_missing_fields() {
        let temp = tempfile::tempdir().unwrap();
        let log = temp.path().join("partial.json");
        std::fs::write(&log, r#"{"other_field": 123}"#).unwrap();

        let result = parse_vmaf_score(&log);
        assert!(result.is_err());
    }
}
