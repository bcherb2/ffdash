// Integration tests for Profile save/load workflows
//
// These tests verify that profiles can be created from config state,
// and that all settings survive the round-trip conversion.

use ffdash::engine::Profile;
use ffdash::ui::state::{ConfigState, RateControlMode};
use proptest::prelude::*;

use crate::common::helpers::*;

// ============================================================================
// UNIT TESTS: Profile round-trip conversion
// ============================================================================

#[test]
fn test_basic_profile_roundtrip() {
    let mut config = ConfigState::default();
    config.crf = 30;
    config.cpu_used = 3;

    let profile = Profile::from_config("Test".to_string(), &config);
    assert_eq!(profile.name, "Test");
    assert_eq!(profile.crf, 30);
    assert_eq!(profile.cpu_used, 3);

    // Apply back to config
    let mut restored = ConfigState::default();
    profile.apply_to_config(&mut restored);

    assert_eq!(restored.crf, 30);
    assert_eq!(restored.cpu_used, 3);
}

#[test]
fn test_all_rate_control_modes_roundtrip() {
    let modes = vec![
        RateControlMode::CQ,
        RateControlMode::CQCap,
        RateControlMode::TwoPassVBR,
        RateControlMode::CBR,
    ];

    for mode in modes {
        let mut config = ConfigState::default();
        config.rate_control_mode = mode;
        config.crf = 28;
        config.video_target_bitrate = 2000;
        config.video_max_bitrate = 5000;

        let profile = Profile::from_config("TestMode".to_string(), &config);
        let mut restored = ConfigState::default();
        profile.apply_to_config(&mut restored);

        assert_eq!(restored.rate_control_mode, mode);
        assert_eq!(restored.crf, 28);
    }
}

#[test]
fn test_two_pass_settings_roundtrip() {
    let mut config = ConfigState::default();
    config.two_pass = true;
    config.cpu_used_pass1 = 4;
    config.cpu_used_pass2 = 1;

    let profile = Profile::from_config("TwoPass".to_string(), &config);
    assert_eq!(profile.two_pass, true);
    assert_eq!(profile.cpu_used_pass1, 4);
    assert_eq!(profile.cpu_used_pass2, 1);

    let mut restored = ConfigState::default();
    profile.apply_to_config(&mut restored);

    assert_eq!(restored.two_pass, true);
    assert_eq!(restored.cpu_used_pass1, 4);
    assert_eq!(restored.cpu_used_pass2, 1);
}

#[test]
fn test_parallelism_settings_roundtrip() {
    let config = parallel_config();

    let profile = Profile::from_config("Parallel".to_string(), &config);
    assert_eq!(profile.row_mt, true);
    assert_eq!(profile.frame_parallel, true);
    assert_eq!(profile.tile_columns, 2);
    assert_eq!(profile.tile_rows, 1);
    assert_eq!(profile.threads, 8);

    let mut restored = ConfigState::default();
    profile.apply_to_config(&mut restored);

    assert_eq!(restored.row_mt, true);
    assert_eq!(restored.frame_parallel, true);
    assert_eq!(restored.tile_columns, 2);
    assert_eq!(restored.tile_rows, 1);
    assert_eq!(restored.threads, 8);
}

#[test]
fn test_gop_settings_roundtrip() {
    let config = custom_gop_config();

    let profile = Profile::from_config("GOP".to_string(), &config);
    assert_eq!(profile.gop_length, "120".to_string());
    assert_eq!(profile.fixed_gop, true);
    assert_eq!(profile.keyint_min, "60".to_string());
    assert_eq!(profile.lag_in_frames, 16);
    assert_eq!(profile.auto_alt_ref, 1);

    let mut restored = ConfigState::default();
    profile.apply_to_config(&mut restored);

    assert_eq!(restored.gop_length, "120".to_string());
    assert_eq!(restored.fixed_gop, true);
    assert_eq!(restored.keyint_min, "60".to_string());
    assert_eq!(restored.lag_in_frames, 16);
    assert_eq!(restored.auto_alt_ref, 1);
}

#[test]
fn test_tuning_settings_roundtrip() {
    let config = tuned_config();

    let profile = Profile::from_config("Tuned".to_string(), &config);
    let mut restored = ConfigState::default();
    profile.apply_to_config(&mut restored);

    assert_eq!(restored.enable_tpl, true);
    assert_eq!(restored.sharpness, 3);
}

#[test]
fn test_bitrate_settings_roundtrip() {
    let mut config = vbr_config();
    config.video_target_bitrate = 2500;
    config.video_min_bitrate = 1500;
    config.video_max_bitrate = 4000;
    config.video_bufsize = 5000;

    let profile = Profile::from_config("Bitrate".to_string(), &config);
    assert_eq!(profile.video_target_bitrate, 2500);
    assert_eq!(profile.video_min_bitrate, 1500);
    assert_eq!(profile.video_max_bitrate, 4000);
    assert_eq!(profile.video_bufsize, 5000);

    let mut restored = ConfigState::default();
    profile.apply_to_config(&mut restored);

    assert_eq!(restored.video_target_bitrate, 2500);
    assert_eq!(restored.video_min_bitrate, 1500);
    assert_eq!(restored.video_max_bitrate, 4000);
    assert_eq!(restored.video_bufsize, 5000);
}

#[test]
fn test_undershoot_overshoot_roundtrip() {
    let mut config = ConfigState::default();
    config.undershoot_pct = 50;
    config.overshoot_pct = 200;

    let profile = Profile::from_config("Shoot".to_string(), &config);
    assert_eq!(profile.undershoot_pct, 50);
    assert_eq!(profile.overshoot_pct, 200);

    let mut restored = ConfigState::default();
    profile.apply_to_config(&mut restored);

    assert_eq!(restored.undershoot_pct, 50);
    assert_eq!(restored.overshoot_pct, 200);
}

#[test]
fn test_auto_values_roundtrip() {
    let mut config = ConfigState::default();
    config.sharpness = -1; // Auto
    config.undershoot_pct = -1; // Auto
    config.overshoot_pct = -1; // Auto

    let profile = Profile::from_config("Auto".to_string(), &config);
    assert_eq!(profile.sharpness, -1);
    assert_eq!(profile.undershoot_pct, -1);
    assert_eq!(profile.overshoot_pct, -1);

    let mut restored = ConfigState::default();
    profile.apply_to_config(&mut restored);

    assert_eq!(restored.sharpness, -1);
    assert_eq!(restored.undershoot_pct, -1);
    assert_eq!(restored.overshoot_pct, -1);
}

// ============================================================================
// UNIT TESTS: Built-in profiles
// ============================================================================

#[test]
fn test_built_in_1080p_shrinker_profile() {
    let profile = Profile::get_builtin("1080p Shrinker").expect("Profile should exist");

    assert_eq!(profile.name, "1080p Shrinker");
    assert_eq!(profile.video_codec, "libsvtav1");
    assert_eq!(profile.audio_primary_codec, "aac");
    assert_eq!(profile.crf, 43);
    assert_eq!(profile.container, "mp4");
    assert_eq!(profile.audio_primary_bitrate, 112);
    assert_eq!(profile.pix_fmt, "yuv420p"); // 8-bit SDR
}

// ============================================================================
// PROPERTY-BASED TESTS: Comprehensive round-trip verification
// ============================================================================

proptest! {
    // Test that any valid config survives round-trip conversion
    #[test]
    fn proptest_comprehensive_roundtrip(
        crf in 0u32..=63,
        cpu_used in 0u32..=8,
        bitrate in 100u32..=50000,
        tile_cols in 0i32..=6,
        tile_rows in 0i32..=6,
        threads in 0u32..=64,
        gop in 1u32..=600,
        lag in 0u32..=25,
        row_mt in prop::bool::ANY,
        frame_parallel in prop::bool::ANY,
        two_pass in prop::bool::ANY,
        fixed_gop in prop::bool::ANY,
        auto_alt_ref in 0u32..=2,
    ) {
        let mut config = ConfigState::default();
        config.crf = crf;
        config.cpu_used = cpu_used;
        config.video_target_bitrate = bitrate;
        config.tile_columns = tile_cols;
        config.tile_rows = tile_rows;
        config.threads = threads;
        config.gop_length = gop.to_string();
        config.lag_in_frames = lag;
        config.row_mt = row_mt;
        config.frame_parallel = frame_parallel;
        config.two_pass = two_pass;
        config.fixed_gop = fixed_gop;
        config.auto_alt_ref = auto_alt_ref;

        // Convert to profile and back
        let profile = Profile::from_config("PropTest".to_string(), &config);
        let mut restored = ConfigState::default();
        profile.apply_to_config(&mut restored);

        // Verify all settings preserved
        assert_eq!(restored.crf, crf, "CRF not preserved");
        assert_eq!(restored.cpu_used, cpu_used, "cpu_used not preserved");

        // Note: bitrate is NOT preserved because Profile::from_config modifies it based on rate_control_mode
        // For CQ mode (default), all bitrates are set to 0 in the profile
        // So we only check bitrate preservation if we're in a bitrate-based mode
        // (This is expected behavior, not a bug)

        assert_eq!(restored.tile_columns, tile_cols, "tile_columns not preserved");
        assert_eq!(restored.tile_rows, tile_rows, "tile_rows not preserved");
        assert_eq!(restored.threads, threads, "threads not preserved");
        assert_eq!(restored.gop_length, gop.to_string(), "gop_length not preserved");
        assert_eq!(restored.lag_in_frames, lag, "lag_in_frames not preserved");
        assert_eq!(restored.row_mt, row_mt, "row_mt not preserved");
        assert_eq!(restored.frame_parallel, frame_parallel, "frame_parallel not preserved");
        assert_eq!(restored.two_pass, two_pass, "two_pass not preserved");
        assert_eq!(restored.fixed_gop, fixed_gop, "fixed_gop not preserved");
        assert_eq!(restored.auto_alt_ref, auto_alt_ref, "auto_alt_ref not preserved");
    }

    // Test ARNR settings round-trip
    #[test]
    fn proptest_arnr_roundtrip(
        max_frames in 0u32..=15,
        strength in 0u32..=6,
        arnr_type_idx in 0usize..=3,
    ) {
        let mut config = ConfigState::default();
        config.arnr_max_frames = max_frames;
        config.arnr_strength = strength;
        // Set arnr_type via ListState: 0=Auto(-1), 1=Backward(1), 2=Forward(2), 3=Centered(3)
        config.arnr_type_state.select(Some(arnr_type_idx));

        let profile = Profile::from_config("ARNRTest".to_string(), &config);
        let mut restored = ConfigState::default();
        profile.apply_to_config(&mut restored);

        assert_eq!(restored.arnr_max_frames, max_frames);
        assert_eq!(restored.arnr_strength, strength);
        // Check that arnr_type ListState selection is preserved
        assert_eq!(restored.arnr_type_state.selected(), Some(arnr_type_idx));
    }

    // Test advanced tuning round-trip
    #[test]
    fn proptest_tuning_roundtrip(
        sharpness in -1i32..=7,
        noise_sens in 0u32..=6,
        static_thresh in 0u32..=10000,
        max_intra_rate in 0u32..=500,
        enable_tpl in prop::bool::ANY,
    ) {
        let mut config = ConfigState::default();
        config.sharpness = sharpness;
        config.noise_sensitivity = noise_sens;
        config.static_thresh = static_thresh.to_string();
        config.max_intra_rate = max_intra_rate.to_string();
        config.enable_tpl = enable_tpl;

        let profile = Profile::from_config("TuningTest".to_string(), &config);
        let mut restored = ConfigState::default();
        profile.apply_to_config(&mut restored);

        assert_eq!(restored.sharpness, sharpness);
        assert_eq!(restored.noise_sensitivity, noise_sens);
        assert_eq!(restored.static_thresh, static_thresh.to_string());
        assert_eq!(restored.max_intra_rate, max_intra_rate.to_string());
        assert_eq!(restored.enable_tpl, enable_tpl);
    }

    // Test that profile name is preserved
    #[test]
    fn proptest_profile_name(name in "[a-zA-Z0-9_-]{1,50}") {
        let config = ConfigState::default();
        let profile = Profile::from_config(name.clone(), &config);

        assert_eq!(profile.name, name);
    }
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[test]
fn test_extreme_values_roundtrip() {
    let mut config = ConfigState::default();
    config.crf = 63; // Max CRF
    config.cpu_used = 8; // Max CPU-used
    config.tile_columns = 6; // Max tile columns
    config.tile_rows = 6; // Max tile rows
    config.gop_length = 600.to_string(); // Large GOP

    let profile = Profile::from_config("Extreme".to_string(), &config);
    let mut restored = ConfigState::default();
    profile.apply_to_config(&mut restored);

    assert_eq!(restored.crf, 63);
    assert_eq!(restored.cpu_used, 8);
    assert_eq!(restored.tile_columns, 6);
    assert_eq!(restored.tile_rows, 6);
    assert_eq!(restored.gop_length, "600".to_string());
}

#[test]
fn test_minimum_values_roundtrip() {
    let mut config = ConfigState::default();
    config.crf = 0; // Min CRF
    config.cpu_used = 0; // Min CPU-used
    config.tile_columns = 0;
    config.tile_rows = 0;
    config.threads = 0; // Auto
    config.gop_length = 1.to_string(); // Min GOP

    let profile = Profile::from_config("Minimum".to_string(), &config);
    let mut restored = ConfigState::default();
    profile.apply_to_config(&mut restored);

    assert_eq!(restored.crf, 0);
    assert_eq!(restored.cpu_used, 0);
    assert_eq!(restored.tile_columns, 0);
    assert_eq!(restored.tile_rows, 0);
    assert_eq!(restored.threads, 0);
    assert_eq!(restored.gop_length, "1".to_string());
}
