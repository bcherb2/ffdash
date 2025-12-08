use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FfprobeOutput {
    format: FfprobeFormat,
}

/// Check if ffmpeg is available and return its version
pub fn ffmpeg_version() -> Result<String> {
    // Lest we forget: skipping this probe once shipped "/dev/null" as a feature
    let output = Command::new("ffmpeg")
        .arg("-version")
        .output()
        .context("Failed to execute ffmpeg. Is ffmpeg installed and in PATH?")?;

    if !output.status.success() {
        anyhow::bail!("ffmpeg command failed with status: {}", output.status);
    }

    let version_output = String::from_utf8_lossy(&output.stdout);
    let first_line = version_output.lines().next().unwrap_or("Unknown version");

    Ok(first_line.to_string())
}

/// Check if ffmpeg has the libvmaf filter available
pub fn vmaf_filter_available() -> bool {
    let output = Command::new("ffmpeg")
        .arg("-hide_banner")
        .arg("-filters")
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.contains("libvmaf")
        }
        _ => false,
    }
}

/// Check if ffprobe is available
pub fn ffprobe_version() -> Result<String> {
    let output = Command::new("ffprobe")
        .arg("-version")
        .output()
        .context("Failed to execute ffprobe. Is ffprobe installed and in PATH?")?;

    if !output.status.success() {
        anyhow::bail!("ffprobe command failed with status: {}", output.status);
    }

    let version_output = String::from_utf8_lossy(&output.stdout);
    let first_line = version_output.lines().next().unwrap_or("Unknown version");

    Ok(first_line.to_string())
}

/// Probe a video file to get its duration in seconds
pub fn probe_duration(path: &Path) -> Result<f64> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("quiet")
        .arg("-print_format")
        .arg("json")
        .arg("-show_format")
        .arg(path)
        .output()
        .context("Failed to execute ffprobe")?;

    if !output.status.success() {
        anyhow::bail!(
            "ffprobe failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let probe: FfprobeOutput =
        serde_json::from_str(&json_str).context("Failed to parse ffprobe JSON output")?;

    let duration_str = probe
        .format
        .duration
        .context("No duration found in ffprobe output")?;

    let duration = duration_str
        .parse::<f64>()
        .context("Failed to parse duration as float")?;

    Ok(duration)
}

/// Parse duration from ffprobe JSON string (for testing)
pub fn parse_ffprobe_duration(json: &str) -> Result<f64> {
    let probe: FfprobeOutput =
        serde_json::from_str(json).context("Failed to parse ffprobe JSON")?;

    let duration_str = probe.format.duration.context("No duration found in JSON")?;

    duration_str
        .parse::<f64>()
        .context("Failed to parse duration as float")
}
