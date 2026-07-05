use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FfprobeStreamDisposition {
    #[serde(default)]
    attached_pic: u8,
}

#[derive(Debug, Deserialize)]
struct FfprobeStream {
    codec_name: Option<String>,
    codec_type: Option<String>,
    #[serde(default)]
    disposition: Option<FfprobeStreamDisposition>,
}

impl FfprobeStream {
    /// Returns true if this stream is an attached picture (e.g., album art / thumbnail)
    fn is_attached_pic(&self) -> bool {
        self.disposition
            .as_ref()
            .map_or(false, |d| d.attached_pic != 0)
    }
}

#[derive(Debug, Deserialize)]
struct FfprobeOutput {
    format: FfprobeFormat,
    #[serde(default)]
    streams: Vec<FfprobeStream>,
}

/// Input file info from ffprobe (duration and video codec)
#[derive(Debug, Clone)]
pub struct InputInfo {
    pub duration_s: Option<f64>,
    pub video_codec: Option<String>,
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

/// Probe a video file to get duration and video codec in one call
pub fn probe_input_info(path: &Path) -> Result<InputInfo> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("quiet")
        .arg("-print_format")
        .arg("json")
        .arg("-show_format")
        .arg("-show_streams")
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

    // Extract duration
    let duration_s = probe.format.duration.and_then(|s| s.parse::<f64>().ok());

    // Extract video codec from the first non-thumbnail video stream.
    // Files with embedded cover art (e.g., MJPEG or PNG thumbnails) may have the
    // thumbnail as stream 0, which would report the wrong codec and bypass skip logic.
    let video_codec = probe
        .streams
        .iter()
        .find(|s| s.codec_type.as_deref() == Some("video") && !s.is_attached_pic())
        .and_then(|s| s.codec_name.clone());

    Ok(InputInfo {
        duration_s,
        video_codec,
    })
}

/// Check whether two paths refer to the same file.
///
/// Handles the case where the output doesn't exist yet by canonicalizing the
/// parent directory and comparing with the canonical input path.
/// Falls back to raw path comparison if canonicalization fails entirely.
pub fn paths_are_same_file(a: &Path, b: &Path) -> bool {
    // Fast path: both exist and canonicalize succeeds
    if let (Ok(ca), Ok(cb)) = (a.canonicalize(), b.canonicalize()) {
        return ca == cb;
    }
    // Output may not exist yet — canonicalize parent + filename
    if let (Ok(ca), Some(b_parent), Some(b_name)) = (a.canonicalize(), b.parent(), b.file_name()) {
        if let Ok(cb_parent) = b_parent.canonicalize() {
            return ca == cb_parent.join(b_name);
        }
    }
    // Symmetric: also try the other direction
    if let (Ok(cb), Some(a_parent), Some(a_name)) = (b.canonicalize(), a.parent(), a.file_name()) {
        if let Ok(ca_parent) = a_parent.canonicalize() {
            return cb == ca_parent.join(a_name);
        }
    }
    // Last resort: raw comparison
    a == b
}

/// Parse ffprobe JSON to extract input info (for testing)
pub fn parse_ffprobe_input_info(json: &str) -> Result<InputInfo> {
    let probe: FfprobeOutput =
        serde_json::from_str(json).context("Failed to parse ffprobe JSON")?;

    let duration_s = probe.format.duration.and_then(|s| s.parse::<f64>().ok());

    let video_codec = probe
        .streams
        .iter()
        .find(|s| s.codec_type.as_deref() == Some("video") && !s.is_attached_pic())
        .and_then(|s| s.codec_name.clone());

    Ok(InputInfo {
        duration_s,
        video_codec,
    })
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
