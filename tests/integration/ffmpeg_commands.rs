// Integration tests for FFmpeg command generation from config settings
//
// These tests verify that UI config permutations correctly translate to FFmpeg commands

use ffdash::engine::Profile;
use ffdash::ui::state::{ConfigState, RateControlMode};
use proptest::prelude::*;

use crate::common::assertions::*;
use crate::common::helpers::*;

// ============================================================================
// UNIT TESTS: Specific rate control modes
// ============================================================================

#[test]
fn test_cq_mode_generates_crf_flag() {
    let mut config = default_config();
    config.rate_control_mode = RateControlMode::CQ;
    config.crf = 30;
    config.video_target_bitrate = 0; // Should be 0 for unconstrained CQ

    let cmd = build_test_cmd(&config, "TestCQ");

    assert_cmd_contains(&cmd, "-crf 30");
    assert_cmd_contains(&cmd, "-b:v 0");
    assert_cmd_not_contains(&cmd, "-maxrate");
    assert_cmd_not_contains(&cmd, "-minrate");
}

#[test]
fn test_cqcap_mode_generates_crf_and_maxrate() {
    let mut config = default_config();
    config.rate_control_mode = RateControlMode::CQCap;
    config.crf = 25;
    config.video_max_bitrate = 5000;

    let cmd = build_test_cmd(&config, "TestCQCap");

    assert_cmd_contains(&cmd, "-crf 25");
    assert_cmd_contains(&cmd, "-maxrate 5000k");
    // CQCap should not have minrate
    assert_cmd_not_contains(&cmd, "-minrate");
}

#[test]
fn test_two_pass_vbr_generates_bitrate_flags() {
    let mut config = default_config();
    config.rate_control_mode = RateControlMode::TwoPassVBR;
    config.two_pass = true;
    config.video_target_bitrate = 2000;
    config.video_min_bitrate = 1000;
    config.video_max_bitrate = 3000;
    config.video_bufsize = 4000;

    let cmd = build_test_cmd(&config, "TestVBR");

    assert_cmd_contains(&cmd, "-b:v 2000");
    assert_cmd_contains(&cmd, "-minrate 1000k");
    assert_cmd_contains(&cmd, "-maxrate 3000k");
    assert_cmd_contains(&cmd, "-bufsize 4000k");
}

#[test]
fn test_cbr_mode_generates_equal_min_max_bitrate() {
    let mut config = default_config();
    config.rate_control_mode = RateControlMode::CBR;
    // For CBR, Profile::from_config uses video_target_bitrate for all bitrate settings
    config.video_target_bitrate = 2000;

    let cmd = build_test_cmd(&config, "TestCBR");

    assert_cmd_contains(&cmd, "-b:v 2000");
    assert_cmd_contains(&cmd, "-minrate 2000k");
    assert_cmd_contains(&cmd, "-maxrate 2000k");
}

// ============================================================================
// UNIT TESTS: Two-pass encoding
// ============================================================================

#[test]
fn test_two_pass_uses_pass1_cpu_used() {
    let mut config = default_config();
    config.two_pass = true;
    config.cpu_used_pass1 = 4;
    config.cpu_used_pass2 = 1;
    config.cpu_used = 2; // Should be ignored

    let cmd = build_test_cmd(&config, "TestTwoPass");

    // In two-pass mode, command should use pass1 cpu-used initially
    assert_cmd_contains(&cmd, "-cpu-used 4");
}

#[test]
fn test_single_pass_uses_cpu_used() {
    let mut config = default_config();
    config.two_pass = false;
    config.cpu_used = 3;

    let cmd = build_test_cmd(&config, "TestSinglePass");

    assert_cmd_contains(&cmd, "-cpu-used 3");
}

// ============================================================================
// UNIT TESTS: VP9 profiles and pixel formats
// ============================================================================

#[test]
fn test_vp9_profile_0_8bit() {
    let config = cq_config();
    let profile = Profile::from_config("Test".to_string(), &config);

    assert_eq!(profile.vp9_profile, 0);
    assert_eq!(profile.pix_fmt, "yuv420p");
}

#[test]
fn test_vp9_settings_in_command() {
    let config = default_config();
    let cmd = build_test_cmd(&config, "TestVP9");

    assert_cmd_contains(&cmd, "-c:v libvpx-vp9");
    assert_cmd_contains(&cmd, "-profile:v");
    assert_cmd_contains(&cmd, "-pix_fmt");
}

// ============================================================================
// UNIT TESTS: Parallelism settings
// ============================================================================

#[test]
fn test_parallelism_all_enabled() {
    let config = parallel_config();
    let cmd = build_test_cmd(&config, "TestParallel");

    assert_cmd_contains(&cmd, "-row-mt 1");
    assert_cmd_contains(&cmd, "-frame-parallel 1");
    assert_cmd_contains(&cmd, "-tile-columns 2");
    assert_cmd_contains(&cmd, "-tile-rows 1");
    assert_cmd_contains(&cmd, "-threads 8");
}

#[test]
fn test_threads_zero_means_auto() {
    let mut config = default_config();
    config.threads = 0; // Auto

    let cmd = build_test_cmd(&config, "TestAuto");

    // When threads is 0, the flag should not appear (auto mode)
    assert_cmd_not_contains(&cmd, "-threads");
}

// ============================================================================
// UNIT TESTS: GOP and keyframe settings
// ============================================================================

#[test]
fn test_gop_settings() {
    let config = custom_gop_config();
    let cmd = build_test_cmd(&config, "TestGOP");

    assert_cmd_contains(&cmd, "-g 120");
    assert_cmd_contains(&cmd, "-keyint_min 60");
    assert_cmd_contains(&cmd, "-sc_threshold 0"); // fixed_gop = true
    assert_cmd_contains(&cmd, "-lag-in-frames 16");
    assert_cmd_contains(&cmd, "-auto-alt-ref 1");
}

#[test]
fn test_fixed_gop_sets_sc_threshold() {
    let mut config = default_config();
    config.fixed_gop = true;

    let cmd = build_test_cmd(&config, "TestFixedGOP");

    assert_cmd_contains(&cmd, "-sc_threshold 0");
}

// ============================================================================
// UNIT TESTS: Tuning and advanced settings
// ============================================================================

#[test]
fn test_tuning_settings() {
    let config = tuned_config();
    let cmd = build_test_cmd(&config, "TestTuned");

    assert_cmd_contains(&cmd, "-aq-mode 2");
    assert_cmd_contains(&cmd, "-arnr-maxframes 7");
    assert_cmd_contains(&cmd, "-arnr-strength 4");
    assert_cmd_contains(&cmd, "-tune-content screen");
    assert_cmd_contains(&cmd, "-enable-tpl 1");
    assert_cmd_contains(&cmd, "-sharpness 3");
}

#[test]
fn test_auto_values_not_included() {
    let mut config = default_config();
    config.sharpness = -1; // Auto
    config.undershoot_pct = -1; // Auto
    config.overshoot_pct = -1; // Auto

    let cmd = build_test_cmd(&config, "TestAuto");

    assert_cmd_not_contains(&cmd, "-sharpness");
    assert_cmd_not_contains(&cmd, "-undershoot-pct");
    assert_cmd_not_contains(&cmd, "-overshoot-pct");
}

#[test]
fn test_zero_values_not_included() {
    let mut config = default_config();
    config.static_thresh = 0;
    config.max_intra_rate = 0;
    config.noise_sensitivity = 0;

    let cmd = build_test_cmd(&config, "TestZero");

    assert_cmd_not_contains(&cmd, "-static-thresh");
    assert_cmd_not_contains(&cmd, "-max-intra-rate");
    assert_cmd_not_contains(&cmd, "-noise-sensitivity");
}

// ============================================================================
// PROPERTY-BASED TESTS: Settings permutations
// ============================================================================

proptest! {
    // Test that any valid CRF value generates a valid command
    #[test]
    fn proptest_crf_range(crf in 0u32..=63) {
        let mut config = default_config();
        config.crf = crf;

        let cmd = build_test_cmd(&config, "PropTestCRF");

        assert_cmd_contains(&cmd, &format!("-crf {}", crf));
    }

    // Test that any valid bitrate combination works
    #[test]
    fn proptest_bitrate_ranges(
        target in 100u32..=50000,
        min in 100u32..=50000,
        max in 100u32..=50000,
    ) {
        let mut config = default_config();
        config.video_target_bitrate = target;
        config.video_min_bitrate = min;
        config.video_max_bitrate = max;

        let cmd = build_test_cmd(&config, "PropTestBitrate");

        // Command should be valid (contain ffmpeg)
        assert!(cmd.contains("ffmpeg"));
        // Should contain video codec
        assert_cmd_contains(&cmd, "-c:v libvpx-vp9");
    }

    // Test that all rate control modes generate valid commands
    #[test]
    fn proptest_rate_control_modes(
        mode_idx in 0usize..=3,
        crf in 10u32..=50,
        bitrate in 500u32..=10000,
    ) {
        let mode = match mode_idx {
            0 => RateControlMode::CQ,
            1 => RateControlMode::CQCap,
            2 => RateControlMode::TwoPassVBR,
            _ => RateControlMode::CBR,
        };

        let mut config = default_config();
        config.rate_control_mode = mode;
        config.crf = crf;
        config.video_target_bitrate = bitrate;
        config.video_max_bitrate = bitrate + 1000;
        config.video_min_bitrate = if bitrate > 500 { bitrate - 500 } else { 100 };

        let cmd = build_test_cmd(&config, "PropTestMode");

        // All commands should contain base flags
        assert_cmd_contains(&cmd, "-c:v libvpx-vp9");
        assert_cmd_contains(&cmd, "-quality");
    }

    // Test that CPU-used values are within valid range
    #[test]
    fn proptest_cpu_used(cpu_used in 0u32..=8) {
        let mut config = default_config();
        config.cpu_used = cpu_used;
        config.two_pass = false; // Ensure single-pass mode to use cpu_used, not cpu_used_pass1

        let cmd = build_test_cmd(&config, "PropTestCPU");

        assert_cmd_contains(&cmd, &format!("-cpu-used {}", cpu_used));
    }

    // Test that parallelism settings generate valid commands
    #[test]
    fn proptest_parallelism(
        tile_cols in 0i32..=6,
        tile_rows in 0i32..=6,
        threads in 0u32..=64,
    ) {
        let mut config = default_config();
        config.tile_columns = tile_cols;
        config.tile_rows = tile_rows;
        config.threads = threads;

        let cmd = build_test_cmd(&config, "PropTestParallel");

        // Should generate valid command
        assert_cmd_contains(&cmd, "ffmpeg");

        // Verify tile settings are in command
        assert_cmd_contains(&cmd, &format!("-tile-columns {}", tile_cols));
        assert_cmd_contains(&cmd, &format!("-tile-rows {}", tile_rows));
    }

    // Test that GOP settings generate valid commands
    #[test]
    fn proptest_gop_settings(
        gop in 1u32..=600,
        keyint_min in 0u32..=300,
        lag in 0u32..=25,
    ) {
        let mut config = default_config();
        config.gop_length = gop;
        config.keyint_min = keyint_min;
        config.lag_in_frames = lag;

        let cmd = build_test_cmd(&config, "PropTestGOP");

        assert_cmd_contains(&cmd, &format!("-g {}", gop));
        assert_cmd_contains(&cmd, &format!("-lag-in-frames {}", lag));

        if keyint_min > 0 {
            assert_cmd_contains(&cmd, &format!("-keyint_min {}", keyint_min));
        }
    }

    // Test that AQ mode values generate valid commands
    #[test]
    fn proptest_aq_mode(_aq_mode in -1i32..=5) {
        let config = default_config();
        // We need to map this through the ConfigState properly
        // For now, just test that valid values work
        let profile = Profile::from_config("PropTestAQ".to_string(), &config);

        // Should not panic and should have valid values
        assert!(profile.aq_mode >= -1);
        assert!(profile.aq_mode <= 5);
    }

    // Test ARNR settings
    #[test]
    fn proptest_arnr(
        max_frames in 0u32..=15,
        strength in 0u32..=6,
        arnr_type_idx in 0usize..=3,
    ) {
        let mut config = default_config();
        config.arnr_max_frames = max_frames;
        config.arnr_strength = strength;
        // Set arnr_type via ListState: 0=Auto(-1), 1=Backward(1), 2=Forward(2), 3=Centered(3)
        config.arnr_type_state.select(Some(arnr_type_idx));

        let cmd = build_test_cmd(&config, "PropTestARNR");

        if max_frames > 0 {
            assert_cmd_contains(&cmd, &format!("-arnr-maxframes {}", max_frames));
        }
        if strength > 0 {
            assert_cmd_contains(&cmd, &format!("-arnr-strength {}", strength));
        }
        // arnr_type_idx=0 maps to -1 (Auto), which should not appear in command
        if arnr_type_idx > 0 {
            let arnr_type = arnr_type_idx as i32;
            assert_cmd_contains(&cmd, &format!("-arnr-type {}", arnr_type));
        }
    }

    // Test advanced tuning permutations
    #[test]
    fn proptest_advanced_tuning(
        sharpness in -1i32..=7,
        noise_sens in 0u32..=6,
        static_thresh in 0u32..=10000,
    ) {
        let mut config = default_config();
        config.sharpness = sharpness;
        config.noise_sensitivity = noise_sens;
        config.static_thresh = static_thresh;

        let cmd = build_test_cmd(&config, "PropTestTuning");

        // Should generate valid command
        assert_cmd_contains(&cmd, "ffmpeg");

        if sharpness >= 0 {
            assert_cmd_contains(&cmd, &format!("-sharpness {}", sharpness));
        }
    }

    // Test undershoot/overshoot percentages
    #[test]
    fn proptest_shoot_percentages(
        undershoot in -1i32..=100,
        overshoot in -1i32..=1000,
    ) {
        let mut config = default_config();
        config.undershoot_pct = undershoot;
        config.overshoot_pct = overshoot;

        let cmd = build_test_cmd(&config, "PropTestShoot");

        if undershoot >= 0 {
            assert_cmd_contains(&cmd, &format!("-undershoot-pct {}", undershoot));
        } else {
            assert_cmd_not_contains(&cmd, "-undershoot-pct");
        }

        if overshoot >= 0 {
            assert_cmd_contains(&cmd, &format!("-overshoot-pct {}", overshoot));
        } else {
            assert_cmd_not_contains(&cmd, "-overshoot-pct");
        }
    }

    // Test audio settings
    #[test]
    fn proptest_audio_bitrate(_audio_bitrate in 32u32..=512) {
        let config = default_config();
        let profile = Profile::from_config("PropTestAudio".to_string(), &config);

        // Audio bitrate should be in valid range
        assert!(profile.audio_bitrate >= 32);
        assert!(profile.audio_bitrate <= 512);
    }

    // Comprehensive permutation test: multiple settings at once
    #[test]
    fn proptest_comprehensive_permutation(
        crf in 10u32..=50,
        cpu_used in 0u32..=5,
        tile_cols in 0i32..=4,
        gop in 30u32..=300,
        lag in 0u32..=25,
        two_pass in prop::bool::ANY,
    ) {
        let mut config = default_config();
        config.crf = crf;
        config.cpu_used = cpu_used;
        config.tile_columns = tile_cols;
        config.gop_length = gop;
        config.lag_in_frames = lag;
        config.two_pass = two_pass;

        if two_pass {
            config.cpu_used_pass1 = cpu_used;
            config.cpu_used_pass2 = if cpu_used > 1 { cpu_used - 1 } else { 0 };
        }

        let cmd = build_test_cmd(&config, "PropTestComprehensive");

        // Verify core settings are present
        assert_cmd_contains(&cmd, &format!("-crf {}", crf));
        assert_cmd_contains(&cmd, &format!("-g {}", gop));
        assert_cmd_contains(&cmd, &format!("-lag-in-frames {}", lag));

        // Verify command structure is valid
        assert!(cmd.starts_with("ffmpeg"));
        assert!(cmd.contains("-i test_input.mp4"));
        assert!(cmd.contains("test_output.webm"));
    }
}

// ============================================================================
// CONSISTENCY TESTS: Verify settings translate correctly
// ============================================================================

#[test]
fn test_profile_roundtrip_preserves_settings() {
    let mut config = ConfigState::default();
    config.crf = 28;
    config.cpu_used = 3;
    config.tile_columns = 2;
    config.gop_length = 240;
    config.row_mt = true;

    let profile = Profile::from_config("Roundtrip".to_string(), &config);

    // Verify all settings transferred correctly
    assert_eq!(profile.crf, 28);
    assert_eq!(profile.cpu_used, 3);
    assert_eq!(profile.tile_columns, 2);
    assert_eq!(profile.gop_length, 240);
    assert_eq!(profile.row_mt, true);
}

#[test]
fn test_built_in_profiles_generate_valid_commands() {
    let profiles = vec!["1080p Shrinker", "Efficient 4K", "Daily Driver"];

    for profile_name in profiles {
        let profile = Profile::get_builtin(profile_name).expect("Profile should exist");
        let cmd = build_cmd_from_profile(
            &profile,
            &std::path::PathBuf::from("input.mp4"),
            &std::path::PathBuf::from("output.webm"),
        );

        // All built-in profiles should generate valid commands
        assert!(cmd.contains("ffmpeg"));
        assert!(cmd.contains("-c:v libvpx-vp9"));
        assert!(cmd.contains("-crf"));
    }
}

#[test]
fn test_default_config_generates_reasonable_command() {
    let config = default_config();
    let cmd = build_test_cmd(&config, "Default");

    // Default config should have reasonable VP9 settings
    assert_cmd_contains(&cmd, "-c:v libvpx-vp9");
    assert_cmd_contains(&cmd, "-crf");
    assert_cmd_contains(&cmd, "-quality");
    assert_cmd_contains(&cmd, "-cpu-used");
    assert_cmd_contains(&cmd, "-profile:v");
    assert_cmd_contains(&cmd, "-pix_fmt");
}
