// Integration tests for FPS limiting and duration preservation
//
// These tests verify that:
// 1. FPS limiting works correctly (50fps -> 30fps)
// 2. Video duration is preserved (not shortened)
// 3. Resolution scaling works correctly
// 4. Videos with same FPS as limit are not affected

use ffdash::engine::{self, Profile, VideoJob, probe};
use ffdash::ui::state::RateControlMode;
use std::process::Command;
use tempfile::TempDir;

use crate::common::ffmpeg_runner::*;
use crate::common::helpers::*;

// Helper to check if FFmpeg is available
macro_rules! require_ffmpeg {
    () => {
        if !is_ffmpeg_available() {
            eprintln!("Skipping test: FFmpeg not available");
            return;
        }
    };
}

/// Generate a test video with specific FPS and duration
fn generate_test_video_with_fps(
    path: &std::path::Path,
    duration_secs: f64,
    width: u32,
    height: u32,
    fps: u32,
) -> anyhow::Result<()> {
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y")
        .arg("-f")
        .arg("lavfi")
        .arg("-i")
        .arg(format!(
            "testsrc=duration={}:size={}x{}:rate={}",
            duration_secs, width, height, fps
        ))
        .arg("-c:v")
        .arg("libx264")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-threads")
        .arg("1")
        .arg("-preset")
        .arg("ultrafast")
        .arg(path);

    let output = cmd.output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to generate test video: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

#[test]
fn test_fps_limiting_preserves_duration() {
    require_ffmpeg!();

    let temp_dir = TempDir::new().unwrap();

    // Generate a 3-second test video at 50fps
    let input = temp_dir.path().join("input_50fps.mp4");
    generate_test_video_with_fps(&input, 3.0, 640, 480, 50)
        .expect("Failed to generate 50fps test video");

    // Probe input to verify it's 50fps and 3 seconds
    let input_info = probe::probe_input_info(&input).expect("Failed to probe input video");

    assert!(
        input_info.fps >= 49.0 && input_info.fps <= 51.0,
        "Input should be ~50fps, got {}",
        input_info.fps
    );
    assert!(input_info.duration.is_some(), "Input should have duration");
    let input_duration = input_info.duration.unwrap();
    assert!(
        input_duration >= 2.9 && input_duration <= 3.1,
        "Input should be ~3 seconds, got {}",
        input_duration
    );

    // Encode with FPS limit of 30fps
    let output = temp_dir.path().join("output_30fps.webm");
    let job = VideoJob::new(input.clone(), output.clone(), "test".to_string());

    // Create a config with 30fps limit
    let mut config = default_config();
    config.fps = 30; // This should limit to 30fps
    config.rate_control_mode = RateControlMode::CQ;
    config.crf = 30;

    let profile = Profile::from_config("test".to_string(), &config);

    // Build and run the actual FFmpeg command using our engine with profile override
    let mut cmd = engine::build_ffmpeg_cmd_with_profile(&job, None, Some(&profile));

    let output_result = cmd.output().expect("Failed to run FFmpeg");

    if !output_result.status.success() {
        let stderr = String::from_utf8_lossy(&output_result.stderr);
        panic!("FFmpeg failed:\n{}", stderr);
    }

    // Probe output to verify FPS is 30 and duration is preserved
    let output_info = probe::probe_input_info(&output).expect("Failed to probe output video");

    println!("Input: {}fps, {}s", input_info.fps, input_duration);
    println!(
        "Output: {}fps, {}s",
        output_info.fps,
        output_info.duration.unwrap_or(0.0)
    );

    // Verify FPS was limited to 30
    assert!(
        output_info.fps >= 29.0 && output_info.fps <= 31.0,
        "Output should be ~30fps, got {}",
        output_info.fps
    );

    // Verify duration was preserved (within 0.5 second tolerance)
    assert!(
        output_info.duration.is_some(),
        "Output should have duration"
    );
    let output_duration = output_info.duration.unwrap();
    let duration_diff = (output_duration - input_duration).abs();
    assert!(
        duration_diff < 0.5,
        "Duration should be preserved: input={}s, output={}s, diff={}s",
        input_duration,
        output_duration,
        duration_diff
    );
}

#[test]
fn test_same_fps_not_affected() {
    require_ffmpeg!();

    let temp_dir = TempDir::new().unwrap();

    // Generate a 2-second test video at 30fps
    let input = temp_dir.path().join("input_30fps.mp4");
    generate_test_video_with_fps(&input, 2.0, 640, 480, 30)
        .expect("Failed to generate 30fps test video");

    // Probe input
    let input_info = probe::probe_input_info(&input).expect("Failed to probe input video");

    assert!(
        input_info.fps >= 29.0 && input_info.fps <= 31.0,
        "Input should be ~30fps"
    );
    let input_duration = input_info.duration.expect("Input should have duration");

    // Encode with FPS limit of 30fps (same as input)
    let output = temp_dir.path().join("output_30fps.webm");
    let job = VideoJob::new(input.clone(), output.clone(), "test".to_string());

    let mut config = default_config();
    config.fps = 30; // Same as input
    config.rate_control_mode = RateControlMode::CQ;
    config.crf = 30;

    let profile = Profile::from_config("test".to_string(), &config);
    let mut cmd = engine::build_ffmpeg_cmd_with_profile(&job, None, Some(&profile));

    let output_result = cmd.output().expect("Failed to run FFmpeg");
    assert!(output_result.status.success(), "FFmpeg should succeed");

    // Probe output
    let output_info = probe::probe_input_info(&output).expect("Failed to probe output video");

    println!(
        "Same FPS test - Input: {}fps, {}s",
        input_info.fps, input_duration
    );
    println!(
        "Same FPS test - Output: {}fps, {}s",
        output_info.fps,
        output_info.duration.unwrap_or(0.0)
    );

    // Verify FPS stayed 30
    assert!(
        output_info.fps >= 29.0 && output_info.fps <= 31.0,
        "Output should be ~30fps, got {}",
        output_info.fps
    );

    // Verify duration was preserved
    let output_duration = output_info.duration.expect("Output should have duration");
    let duration_diff = (output_duration - input_duration).abs();
    assert!(
        duration_diff < 0.5,
        "Duration should be preserved: input={}s, output={}s",
        input_duration,
        output_duration
    );
}

#[test]
fn test_resolution_scaling_preserves_duration() {
    require_ffmpeg!();

    let temp_dir = TempDir::new().unwrap();

    // Generate a 2-second test video at 1280x720
    let input = temp_dir.path().join("input_720p.mp4");
    generate_test_video_with_fps(&input, 2.0, 1280, 720, 30)
        .expect("Failed to generate 720p test video");

    // Probe input
    let input_info = probe::probe_input_info(&input).expect("Failed to probe input video");

    assert_eq!(input_info.width, 1280, "Input should be 1280 width");
    assert_eq!(input_info.height, 720, "Input should be 720 height");
    let input_duration = input_info.duration.expect("Input should have duration");

    // Encode with resolution limit of 640x360
    let output = temp_dir.path().join("output_360p.webm");
    let job = VideoJob::new(input.clone(), output.clone(), "test".to_string());

    let mut config = default_config();
    config.scale_width = 640;
    config.scale_height = 360;
    config.rate_control_mode = RateControlMode::CQ;
    config.crf = 30;

    let profile = Profile::from_config("test".to_string(), &config);
    let mut cmd = engine::build_ffmpeg_cmd_with_profile(&job, None, Some(&profile));

    let output_result = cmd.output().expect("Failed to run FFmpeg");
    assert!(output_result.status.success(), "FFmpeg should succeed");

    // Probe output
    let output_info = probe::probe_input_info(&output).expect("Failed to probe output video");

    println!(
        "Scaling test - Input: {}x{}, {}s",
        input_info.width, input_info.height, input_duration
    );
    println!(
        "Scaling test - Output: {}x{}, {}s",
        output_info.width,
        output_info.height,
        output_info.duration.unwrap_or(0.0)
    );

    // Verify resolution was scaled down (with some tolerance for aspect ratio adjustments)
    assert!(
        output_info.width <= 640,
        "Output width should be <= 640, got {}",
        output_info.width
    );
    assert!(
        output_info.height <= 360,
        "Output height should be <= 360, got {}",
        output_info.height
    );

    // Verify duration was preserved
    let output_duration = output_info.duration.expect("Output should have duration");
    let duration_diff = (output_duration - input_duration).abs();
    assert!(
        duration_diff < 0.5,
        "Duration should be preserved: input={}s, output={}s",
        input_duration,
        output_duration
    );
}
