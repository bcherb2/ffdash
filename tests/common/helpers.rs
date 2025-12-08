#![allow(dead_code)] // Spare tools for the next regression meteor shower

use ffdash::engine::{HwEncodingConfig, Profile, VideoJob, build_ffmpeg_cmd_with_profile};
use ffdash::ui::state::{ConfigState, RateControlMode};
use std::path::PathBuf;
use std::process::Command;

/// Convert a Command to a string for testing/assertions
pub fn cmd_to_string(cmd: &Command) -> String {
    let program = cmd.get_program().to_string_lossy();
    let args: Vec<String> = cmd
        .get_args()
        .map(|arg| arg.to_string_lossy().to_string())
        .collect();

    format!("{} {}", program, args.join(" "))
}

/// Build an FFmpeg command from a config for testing
pub fn build_test_cmd(config: &ConfigState, profile_name: &str) -> String {
    let profile = Profile::from_config(profile_name.to_string(), config);
    let job = VideoJob::new(
        PathBuf::from("test_input.mp4"),
        PathBuf::from("test_output.webm"),
        profile_name.to_string(),
    );

    // We need to manually build the command here since build_ffmpeg_cmd uses Profile::get
    // which only works with built-in profiles. For testing, we'll create a simplified version
    // that uses our test profile directly.
    build_cmd_from_profile(&profile, &job.input_path, &job.output_path)
}

/// Build an FFmpeg command from a Profile for testing
pub fn build_cmd_from_profile(profile: &Profile, input: &PathBuf, output: &PathBuf) -> String {
    let mut parts = vec!["ffmpeg".to_string()];

    // Input
    parts.push("-i".to_string());
    parts.push(input.to_string_lossy().to_string());

    // Progress output
    parts.push("-progress".to_string());
    parts.push("-".to_string());
    parts.push("-nostats".to_string());

    // Video codec
    parts.push("-c:v".to_string());
    parts.push(profile.video_codec.clone());

    // Rate control
    parts.push("-b:v".to_string());
    parts.push(profile.video_target_bitrate.to_string());
    parts.push("-crf".to_string());
    parts.push(profile.crf.to_string());

    if profile.video_min_bitrate > 0 {
        parts.push("-minrate".to_string());
        parts.push(format!("{}k", profile.video_min_bitrate));
    }
    if profile.video_max_bitrate > 0 {
        parts.push("-maxrate".to_string());
        parts.push(format!("{}k", profile.video_max_bitrate));
    }
    if profile.video_bufsize > 0 {
        parts.push("-bufsize".to_string());
        parts.push(format!("{}k", profile.video_bufsize));
    }
    if profile.undershoot_pct >= 0 {
        parts.push("-undershoot-pct".to_string());
        parts.push(profile.undershoot_pct.to_string());
    }
    if profile.overshoot_pct >= 0 {
        parts.push("-overshoot-pct".to_string());
        parts.push(profile.overshoot_pct.to_string());
    }

    // Quality mode
    parts.push("-quality".to_string());
    parts.push(profile.quality_mode.clone());

    // CPU-used
    if profile.two_pass {
        parts.push("-cpu-used".to_string());
        parts.push(profile.cpu_used_pass1.to_string());
    } else {
        parts.push("-cpu-used".to_string());
        parts.push(profile.cpu_used.to_string());
    }

    // VP9 profile and pixel format
    parts.push("-profile:v".to_string());
    parts.push(profile.vp9_profile.to_string());
    parts.push("-pix_fmt".to_string());
    parts.push(profile.pix_fmt.clone());

    // Parallelism
    if profile.row_mt {
        parts.push("-row-mt".to_string());
        parts.push("1".to_string());
    }
    if profile.tile_columns >= 0 {
        parts.push("-tile-columns".to_string());
        parts.push(profile.tile_columns.to_string());
    }
    if profile.tile_rows >= 0 {
        parts.push("-tile-rows".to_string());
        parts.push(profile.tile_rows.to_string());
    }
    if profile.threads > 0 {
        parts.push("-threads".to_string());
        parts.push(profile.threads.to_string());
    }
    if profile.frame_parallel {
        parts.push("-frame-parallel".to_string());
        parts.push("1".to_string());
    }

    // GOP & keyframes
    parts.push("-g".to_string());
    parts.push(profile.gop_length.to_string());
    if profile.keyint_min > 0 {
        parts.push("-keyint_min".to_string());
        parts.push(profile.keyint_min.to_string());
    }
    if profile.fixed_gop {
        parts.push("-sc_threshold".to_string());
        parts.push("0".to_string());
    }
    parts.push("-lag-in-frames".to_string());
    parts.push(profile.lag_in_frames.to_string());
    if profile.auto_alt_ref {
        parts.push("-auto-alt-ref".to_string());
        parts.push("1".to_string());
    }

    // AQ mode
    if profile.aq_mode >= 0 {
        parts.push("-aq-mode".to_string());
        parts.push(profile.aq_mode.to_string());
    }

    // ARNR
    if profile.arnr_max_frames > 0 {
        parts.push("-arnr-maxframes".to_string());
        parts.push(profile.arnr_max_frames.to_string());
    }
    if profile.arnr_strength > 0 {
        parts.push("-arnr-strength".to_string());
        parts.push(profile.arnr_strength.to_string());
    }
    if profile.arnr_type >= 0 {
        parts.push("-arnr-type".to_string());
        parts.push(profile.arnr_type.to_string());
    }

    // Advanced tuning
    if profile.enable_tpl {
        parts.push("-enable-tpl".to_string());
        parts.push("1".to_string());
    }
    if profile.sharpness >= 0 {
        parts.push("-sharpness".to_string());
        parts.push(profile.sharpness.to_string());
    }
    if profile.noise_sensitivity > 0 {
        parts.push("-noise-sensitivity".to_string());
        parts.push(profile.noise_sensitivity.to_string());
    }
    if profile.static_thresh > 0 {
        parts.push("-static-thresh".to_string());
        parts.push(profile.static_thresh.to_string());
    }
    if profile.max_intra_rate > 0 {
        parts.push("-max-intra-rate".to_string());
        parts.push(profile.max_intra_rate.to_string());
    }
    if profile.tune_content != "default" {
        parts.push("-tune-content".to_string());
        parts.push(profile.tune_content.clone());
    }

    // Color metadata
    if profile.colorspace >= 0 {
        parts.push("-colorspace".to_string());
        parts.push(profile.colorspace.to_string());
    }
    if profile.color_primaries >= 0 {
        parts.push("-color_primaries".to_string());
        parts.push(profile.color_primaries.to_string());
    }
    if profile.color_trc >= 0 {
        parts.push("-color_trc".to_string());
        parts.push(profile.color_trc.to_string());
    }
    if profile.color_range >= 0 {
        parts.push("-color_range".to_string());
        parts.push(profile.color_range.to_string());
    }

    // Audio codec
    parts.push("-c:a".to_string());
    parts.push(profile.audio_codec.clone());
    parts.push("-b:a".to_string());
    parts.push(format!("{}k", profile.audio_bitrate));

    // Output
    parts.push(output.to_string_lossy().to_string());

    parts.join(" ")
}

/// Create a default ConfigState for testing
pub fn default_config() -> ConfigState {
    ConfigState::default()
}

/// Create a ConfigState with CQ rate control mode
pub fn cq_config() -> ConfigState {
    let mut config = ConfigState::default();
    config.rate_control_mode = RateControlMode::CQ;
    config.crf = 30;
    config
}

/// Create a ConfigState with TwoPassVBR rate control mode
pub fn vbr_config() -> ConfigState {
    let mut config = ConfigState::default();
    config.rate_control_mode = RateControlMode::TwoPassVBR;
    config.video_target_bitrate = 2000;
    config.video_min_bitrate = 1000;
    config.video_max_bitrate = 3000;
    config.two_pass = true;
    config
}

/// Create a ConfigState with CBR rate control mode
pub fn cbr_config() -> ConfigState {
    let mut config = ConfigState::default();
    config.rate_control_mode = RateControlMode::CBR;
    config.video_min_bitrate = 2000;
    config.video_max_bitrate = 2000;
    config
}

/// Create a ConfigState with CQCap rate control mode
pub fn cqcap_config() -> ConfigState {
    let mut config = ConfigState::default();
    config.rate_control_mode = RateControlMode::CQCap;
    config.crf = 30;
    config.video_max_bitrate = 5000;
    config
}

/// Create a two-pass encoding config
pub fn two_pass_config() -> ConfigState {
    let mut config = ConfigState::default();
    config.two_pass = true;
    config.cpu_used_pass1 = 4;
    config.cpu_used_pass2 = 1;
    config
}

/// Create a config with all parallelism features enabled
pub fn parallel_config() -> ConfigState {
    let mut config = ConfigState::default();
    config.row_mt = true;
    config.frame_parallel = true;
    config.tile_columns = 2;
    config.tile_rows = 1;
    config.threads = 8;
    config
}

/// Create a config with custom GOP settings
pub fn custom_gop_config() -> ConfigState {
    let mut config = ConfigState::default();
    config.gop_length = 120;
    config.fixed_gop = true;
    config.keyint_min = 60;
    config.lag_in_frames = 16;
    config.auto_alt_ref = true;
    config
}

/// Create a config with tuning options
pub fn tuned_config() -> ConfigState {
    let mut config = ConfigState::default();
    // Set aq_mode via ListState: index 2 = Variance
    config.aq_mode_state.select(Some(2));
    config.arnr_max_frames = 7;
    config.arnr_strength = 4;
    // Set tune_content via ListState: index 1 = screen
    config.tune_content_state.select(Some(1));
    config.enable_tpl = true;
    config.sharpness = 3;
    config
}

/// Convert a ConfigState to a Profile for testing
pub fn config_to_profile(config: &ConfigState, name: &str) -> Profile {
    Profile::from_config(name.to_string(), config)
}

/// Build a software (libvpx-vp9) command using the ACTUAL engine code
/// This ensures tests verify the real implementation, not duplicated logic
pub fn build_software_cmd_for_test(profile: &Profile) -> String {
    let job = VideoJob::new(
        PathBuf::from("test_input.mp4"),
        PathBuf::from("test_output.webm"),
        profile.name.clone(),
    );

    // Call actual build function with hw_config=None for software encoding
    let cmd = build_ffmpeg_cmd_with_profile(&job, None, Some(profile));
    cmd_to_string(&cmd)
}

/// Build a VAAPI hardware command using the ACTUAL engine code
/// This ensures tests verify the real implementation, not duplicated logic
pub fn build_vaapi_cmd_for_test(profile: &Profile, hw_config: &HwEncodingConfig) -> String {
    let job = VideoJob::new(
        PathBuf::from("test_input.mp4"),
        PathBuf::from("test_output.webm"),
        profile.name.clone(),
    );

    // Call actual build function with hw_config=Some for VAAPI encoding
    let cmd = build_ffmpeg_cmd_with_profile(&job, Some(hw_config), Some(profile));
    cmd_to_string(&cmd)
}

/// Create a default HwEncodingConfig for testing
pub fn default_hw_config() -> HwEncodingConfig {
    HwEncodingConfig::default()
}

/// Create a custom HwEncodingConfig for testing
pub fn custom_hw_config(quality: u32, b_frames: u32) -> HwEncodingConfig {
    HwEncodingConfig {
        rc_mode: 4, // ICQ (Intelligent Constant Quality) - default
        global_quality: quality,
        b_frames,
        loop_filter_level: 16,
        loop_filter_sharpness: 4,
        compression_level: 4, // Balanced - default
    }
}
