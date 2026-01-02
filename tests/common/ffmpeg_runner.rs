#![allow(dead_code)] // Kept for when FFmpeg decides to misbehave off the happy path

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
/// FFmpeg command execution utilities for integration tests
use std::process::{Command, Stdio};

/// Configuration for FFmpeg test runs
pub struct FfmpegTestConfig {
    /// Maximum number of frames to encode (default: 5)
    pub max_frames: u32,
    /// Maximum duration in seconds (default: 0.2)
    pub max_duration_secs: f32,
    /// Timeout in seconds for the entire command (default: 10)
    pub timeout_secs: u64,
    /// Whether to keep output files after test (default: false)
    pub keep_output: bool,
}

impl Default for FfmpegTestConfig {
    fn default() -> Self {
        Self {
            max_frames: 5,
            max_duration_secs: 0.2,
            timeout_secs: 10,
            keep_output: false,
        }
    }
}

/// Result of an FFmpeg test run
#[derive(Debug)]
pub struct FfmpegTestResult {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub output_file_exists: bool,
    pub output_file_size: Option<u64>,
}

/// Run an FFmpeg command with test constraints
pub fn run_ffmpeg_test(
    input_file: &Path,
    output_file: &Path,
    extra_args: &[String],
    config: &FfmpegTestConfig,
) -> Result<FfmpegTestResult> {
    // Ensure input file exists
    if !input_file.exists() {
        anyhow::bail!("Input file does not exist: {}", input_file.display());
    }

    // Build command
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y") // Overwrite output
        .arg("-i")
        .arg(input_file)
        .arg("-t")
        .arg(config.max_duration_secs.to_string()) // Limit duration
        .arg("-frames:v")
        .arg(config.max_frames.to_string()); // Limit frames

    // Add extra arguments (codec, quality settings, etc.)
    for arg in extra_args {
        cmd.arg(arg);
    }

    // Output file
    cmd.arg(output_file);

    // Capture output
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    // Run command with timeout
    let output = cmd.output().context("Failed to execute ffmpeg command")?;

    // Check if output file was created
    let output_file_exists = output_file.exists();
    let output_file_size = if output_file_exists {
        fs::metadata(output_file).ok().map(|m| m.len())
    } else {
        None
    };

    // Collect results
    let result = FfmpegTestResult {
        success: output.status.success(),
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        output_file_exists,
        output_file_size,
    };

    // Clean up output file unless keeping
    if !config.keep_output && output_file_exists {
        let _ = fs::remove_file(output_file);
    }

    Ok(result)
}

/// Run an FFmpeg command from a full command string
pub fn run_ffmpeg_command_string(
    cmd_string: &str,
    input_file: &Path,
    output_file: &Path,
    config: &FfmpegTestConfig,
) -> Result<FfmpegTestResult> {
    // Parse the command string to extract arguments
    let parts: Vec<&str> = cmd_string.split_whitespace().collect();

    if parts.is_empty() || parts[0] != "ffmpeg" {
        anyhow::bail!("Invalid ffmpeg command string");
    }

    // Build command from scratch with our constraints
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y"); // Overwrite

    // Find the -i flag and replace input path
    let mut i = 1;
    let mut found_input = false;
    while i < parts.len() {
        if parts[i] == "-i" && i + 1 < parts.len() {
            cmd.arg("-i").arg(input_file);
            found_input = true;
            i += 2;
            continue;
        }
        i += 1;
    }

    if !found_input {
        anyhow::bail!("No -i flag found in command");
    }

    // Add duration and frame limits
    cmd.arg("-t")
        .arg(config.max_duration_secs.to_string())
        .arg("-frames:v")
        .arg(config.max_frames.to_string());

    // Add all arguments except ffmpeg, -i, input path, and output path
    i = 1;
    while i < parts.len() {
        let arg = parts[i];

        // Skip -i and its argument (already handled)
        if arg == "-i" {
            i += 2;
            continue;
        }

        // Skip original output file (add ours at the end)
        if i == parts.len() - 1 {
            break;
        }

        cmd.arg(arg);
        i += 1;
    }

    // Add our output file
    cmd.arg(output_file);

    // Capture output
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    // Run command
    let output = cmd.output().context("Failed to execute ffmpeg command")?;

    // Check if output file was created
    let output_file_exists = output_file.exists();
    let output_file_size = if output_file_exists {
        fs::metadata(output_file).ok().map(|m| m.len())
    } else {
        None
    };

    // Collect results
    let result = FfmpegTestResult {
        success: output.status.success(),
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        output_file_exists,
        output_file_size,
    };

    // Clean up output file unless keeping
    if !config.keep_output && output_file_exists {
        let _ = fs::remove_file(output_file);
    }

    Ok(result)
}

/// Generate a test input video file using FFmpeg testsrc
pub fn generate_test_video(
    output_path: &Path,
    duration_secs: f32,
    width: u32,
    height: u32,
) -> Result<()> {
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y")
        .arg("-f")
        .arg("lavfi")
        .arg("-i")
        .arg(format!(
            "testsrc=duration={}:size={}x{}:rate=30",
            duration_secs, width, height
        ))
        .arg("-c:v")
        .arg("libx264")
        .arg("-preset")
        .arg("ultrafast")
        .arg("-threads")
        .arg("1")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-an")
        .arg(output_path);

    let output = cmd.output().context("Failed to generate test video")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to generate test video: {}", stderr);
    }

    Ok(())
}

/// Check if FFmpeg is available
pub fn is_ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Assert that an FFmpeg test result is successful
pub fn assert_ffmpeg_success(result: &FfmpegTestResult, test_name: &str) {
    if !result.success {
        eprintln!("FFmpeg command failed for test: {}", test_name);
        eprintln!("Exit code: {}", result.exit_code);
        eprintln!("Stderr:\n{}", result.stderr);
        eprintln!("Stdout:\n{}", result.stdout);
        panic!("FFmpeg command failed");
    }

    assert!(result.output_file_exists, "Output file was not created");
    assert!(
        result.output_file_size.unwrap_or(0) > 0,
        "Output file is empty"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffmpeg_available() {
        // This test documents the requirement for FFmpeg
        if !is_ffmpeg_available() {
            eprintln!("Warning: FFmpeg is not available. E2E tests will be skipped.");
        }
    }
}
