// Input probing using ffprobe

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputInfo {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub duration: Option<f64>,
}

impl Default for InputInfo {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps: 30.0,
            duration: None,
        }
    }
}

/// Probe input file using ffprobe to get video metadata
pub fn probe_input_info(input_path: &Path) -> Result<InputInfo, String> {
    // Run ffprobe to get JSON output with video stream info
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            "-select_streams",
            "v:0", // First video stream only
        ])
        .arg(input_path)
        .output()
        .map_err(|e| format!("Failed to run ffprobe: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "ffprobe failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // Parse JSON output
    let json_str = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse ffprobe JSON: {}", e))?;

    // Extract video stream info
    let streams = json["streams"]
        .as_array()
        .ok_or("No streams found in ffprobe output")?;

    if streams.is_empty() {
        return Err("No video stream found".to_string());
    }

    let video_stream = &streams[0];

    // Get width and height
    let width = video_stream["width"]
        .as_u64()
        .ok_or("Failed to get video width")? as u32;
    let height = video_stream["height"]
        .as_u64()
        .ok_or("Failed to get video height")? as u32;

    // Get FPS (framerate)
    // Try r_frame_rate first (more accurate), fall back to avg_frame_rate
    let fps_str = video_stream["r_frame_rate"]
        .as_str()
        .or_else(|| video_stream["avg_frame_rate"].as_str())
        .ok_or("Failed to get video framerate")?;

    // Parse fraction (e.g., "30000/1001" or "30/1")
    let fps =
        parse_fraction(fps_str).ok_or_else(|| format!("Failed to parse framerate: {}", fps_str))?;

    // Get duration (optional)
    let duration = json["format"]["duration"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok());

    Ok(InputInfo {
        width,
        height,
        fps,
        duration,
    })
}

/// Parse a fraction string like "30000/1001" to f64
fn parse_fraction(s: &str) -> Option<f64> {
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() != 2 {
        return None;
    }

    let numerator: f64 = parts[0].parse().ok()?;
    let denominator: f64 = parts[1].parse().ok()?;

    if denominator == 0.0 {
        return None;
    }

    Some(numerator / denominator)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_fraction() {
        assert_eq!(parse_fraction("30/1"), Some(30.0));

        // Use approximate equality for floating point results
        let result_29_97 = parse_fraction("30000/1001").unwrap();
        assert!(
            (result_29_97 - 29.970029970029973).abs() < 1e-10,
            "Expected ~29.97, got {}",
            result_29_97
        );

        let result_23_976 = parse_fraction("24000/1001").unwrap();
        assert!(
            (result_23_976 - 23.976023976023978).abs() < 1e-10,
            "Expected ~23.976, got {}",
            result_23_976
        );

        assert_eq!(parse_fraction("60/1"), Some(60.0));
        assert_eq!(parse_fraction("invalid"), None);
        assert_eq!(parse_fraction("30/0"), None);
    }
}
