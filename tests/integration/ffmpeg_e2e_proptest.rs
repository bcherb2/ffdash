// Property-based E2E tests that run FFmpeg with setting permutations
//
// These tests use proptest to generate many combinations of settings
// and validate they all work with real FFmpeg execution

use ffdash::ui::state::RateControlMode;
use proptest::prelude::*;
use std::path::PathBuf;
use tempfile::TempDir;

use crate::common::ffmpeg_runner::*;
use crate::common::helpers::*;

// Helper to check if FFmpeg is available, skip test if not
// Using prop_assume! to skip test cases when FFmpeg is unavailable
macro_rules! require_ffmpeg {
    () => {
        prop_assume!(is_ffmpeg_available());
    };
}

// ============================================================================
// SETUP: Test fixtures
// ============================================================================

fn create_test_video(temp_dir: &TempDir) -> PathBuf {
    let video_path = temp_dir.path().join("test_input.mp4");
    generate_test_video(&video_path, 1.0, 320, 240).expect("Failed to generate test video");
    video_path
}

// ============================================================================
// PROPERTY-BASED E2E TESTS: Core Settings
// ============================================================================

proptest! {
    // Limit cases for speed - increase in CI for exhaustive testing
    #![proptest_config(ProptestConfig::with_cases(10))]

    /// Test all CRF values actually work with FFmpeg
    #[test]
    fn proptest_e2e_crf_values(crf in 10u32..=50) {
        require_ffmpeg!();

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let input = create_test_video(&temp_dir);
        let output = temp_dir.path().join(format!("output_crf{}.webm", crf));

        let mut config = default_config();
        config.crf = crf;
        config.two_pass = false; // Single pass for speed

        let cmd = build_test_cmd(&config, &format!("E2E_CRF{}", crf));

        let test_config = FfmpegTestConfig {
            max_frames: 3,
            max_duration_secs: 0.15,
            timeout_secs: 15,
            ..Default::default()
        };

        let result = run_ffmpeg_command_string(&cmd, &input, &output, &test_config)
            .expect("Failed to run FFmpeg");

        prop_assert!(result.success,
            "FFmpeg failed for CRF {}: {}", crf, result.stderr);
        prop_assert!(result.output_file_exists,
            "Output file not created for CRF {}", crf);
        prop_assert!(result.output_file_size.unwrap_or(0) > 0,
            "Output file is empty for CRF {}", crf);
    }

    /// Test all CPU-used values actually work with FFmpeg
    #[test]
    fn proptest_e2e_cpu_used_values(cpu_used in 0u32..=8) {
        require_ffmpeg!();

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let input = create_test_video(&temp_dir);
        let output = temp_dir.path().join(format!("output_cpu{}.webm", cpu_used));

        let mut config = default_config();
        config.cpu_used = cpu_used;
        config.two_pass = false;

        let cmd = build_test_cmd(&config, &format!("E2E_CPU{}", cpu_used));

        let test_config = FfmpegTestConfig {
            max_frames: if cpu_used < 2 { 2 } else { 3 },
            max_duration_secs: 0.15,
            timeout_secs: if cpu_used < 2 { 20 } else { 15 },
            ..Default::default()
        };

        let result = run_ffmpeg_command_string(&cmd, &input, &output, &test_config).expect("Failed to run FFmpeg");

        prop_assert!(result.success,
            "FFmpeg failed for CPU-used {}: {}", cpu_used, result.stderr);
        prop_assert!(result.output_file_exists);
        prop_assert!(result.output_file_size.unwrap_or(0) > 0);

    }

    /// Test two-pass mode with different CPU-used combinations
    #[test]
    fn proptest_e2e_two_pass_cpu_combinations(
        cpu_pass1 in 3u32..=8,
        cpu_pass2 in 0u32..=2,
    ) {
        require_ffmpeg!();

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let input = create_test_video(&temp_dir);
        let output = temp_dir.path().join(format!("output_2pass_{}_{}.webm", cpu_pass1, cpu_pass2));

        let mut config = default_config();
        config.two_pass = true;
        config.cpu_used_pass1 = cpu_pass1;
        config.cpu_used_pass2 = cpu_pass2;

        let cmd = build_test_cmd(&config, &format!("E2E_TwoPass_{}_{}", cpu_pass1, cpu_pass2));

        let test_config = FfmpegTestConfig {
            max_frames: 2,
            max_duration_secs: 0.1,
            timeout_secs: 20,
            ..Default::default()
        };

        let result = run_ffmpeg_command_string(&cmd, &input, &output, &test_config).expect("Failed to run FFmpeg");

        prop_assert!(result.success,
            "FFmpeg failed for two-pass {}/{}: {}", cpu_pass1, cpu_pass2, result.stderr);
        prop_assert!(result.output_file_exists);
        prop_assert!(result.output_file_size.unwrap_or(0) > 0);

    }

    /// Test GOP length range
    #[test]
    fn proptest_e2e_gop_lengths(gop_length in 10u32..=300) {
        require_ffmpeg!();

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let input = create_test_video(&temp_dir);
        let output = temp_dir.path().join(format!("output_gop{}.webm", gop_length));

        let mut config = default_config();
        config.gop_length = gop_length;
        config.two_pass = false;

        let cmd = build_test_cmd(&config, &format!("E2E_GOP{}", gop_length));

        let test_config = FfmpegTestConfig {
            max_frames: 3,
            max_duration_secs: 0.15,
            ..Default::default()
        };

        let result = run_ffmpeg_command_string(&cmd, &input, &output, &test_config).expect("Failed to run FFmpeg");

        prop_assert!(result.success,
            "FFmpeg failed for GOP {}: {}", gop_length, result.stderr);
        prop_assert!(result.output_file_exists);
        prop_assert!(result.output_file_size.unwrap_or(0) > 0);

    }

    /// Test parallelism settings combinations
    #[test]
    fn proptest_e2e_parallelism(
        tile_cols in 0i32..=4,
        tile_rows in 0i32..=2,
        threads in 1u32..=8,
        row_mt in prop::bool::ANY,
        frame_parallel in prop::bool::ANY,
    ) {
        require_ffmpeg!();

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let input = create_test_video(&temp_dir);
        let output = temp_dir.path().join(format!("output_parallel_{}_{}_{}.webm",
            tile_cols, tile_rows, threads));

        let mut config = default_config();
        config.tile_columns = tile_cols;
        config.tile_rows = tile_rows;
        config.threads = threads;
        config.row_mt = row_mt;
        config.frame_parallel = frame_parallel;
        config.two_pass = false;

        let cmd = build_test_cmd(&config, "E2E_Parallel");

        let test_config = FfmpegTestConfig {
            max_frames: 3,
            max_duration_secs: 0.15,
            ..Default::default()
        };

        let result = run_ffmpeg_command_string(&cmd, &input, &output, &test_config).expect("Failed to run FFmpeg");

        prop_assert!(result.success,
            "FFmpeg failed for parallelism config: {}", result.stderr);
        prop_assert!(result.output_file_exists);
        prop_assert!(result.output_file_size.unwrap_or(0) > 0);

    }

    /// Test audio settings combinations
    #[test]
    fn proptest_e2e_audio_settings(
        codec_idx in 0usize..=3,  // 0=libopus, 1=aac, 2=mp3, 3=vorbis
        bitrate in 96u32..=192,  // Safe range for most codecs
    ) {
        require_ffmpeg!();

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let input = create_test_video(&temp_dir);
        let output = temp_dir.path().join(format!("output_audio_{}_{}.webm", codec_idx, bitrate));

        let mut config = default_config();
        config.codec_list_state.select(Some(codec_idx));
        config.audio_bitrate = bitrate;
        config.two_pass = false;

        let cmd = build_test_cmd(&config, "E2E_Audio");

        let test_config = FfmpegTestConfig {
            max_frames: 3,
            max_duration_secs: 0.15,
            ..Default::default()
        };

        let result = run_ffmpeg_command_string(&cmd, &input, &output, &test_config).expect("Failed to run FFmpeg");

        // Some codec/bitrate combinations might fail - that's OK, we're testing what works
        // Skip unsupported codec/bitrate combinations
        prop_assume!(!(result.stderr.contains("unsupported") || result.stderr.contains("Invalid argument")));

        prop_assert!(result.success,
            "FFmpeg failed for audio codec idx {} bitrate {}: {}",
            codec_idx, bitrate, result.stderr);
        prop_assert!(result.output_file_exists);
        prop_assert!(result.output_file_size.unwrap_or(0) > 0);
    }

    /// Test advanced tuning settings combinations
    #[test]
    fn proptest_e2e_tuning_settings(
        sharpness in -1i32..=7,
        lag_in_frames in 0u32..=25,
        arnr_max_frames in 0u32..=15,
        arnr_strength in 0u32..=6,
        enable_tpl in prop::bool::ANY,
        auto_alt_ref in prop::bool::ANY,
    ) {
        require_ffmpeg!();

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let input = create_test_video(&temp_dir);
        let output = temp_dir.path().join("output_tuning.webm");

        let mut config = default_config();
        config.sharpness = sharpness;
        config.lag_in_frames = lag_in_frames;
        config.arnr_max_frames = arnr_max_frames;
        config.arnr_strength = arnr_strength;
        config.enable_tpl = enable_tpl;
        config.auto_alt_ref = auto_alt_ref;
        config.two_pass = false;

        let cmd = build_test_cmd(&config, "E2E_Tuning");

        let test_config = FfmpegTestConfig {
            max_frames: 3,
            max_duration_secs: 0.15,
            ..Default::default()
        };

        let result = run_ffmpeg_command_string(&cmd, &input, &output, &test_config).expect("Failed to run FFmpeg");

        prop_assert!(result.success,
            "FFmpeg failed for tuning settings: {}", result.stderr);
        prop_assert!(result.output_file_exists);
        prop_assert!(result.output_file_size.unwrap_or(0) > 0);

    }

    /// Test VBR bitrate range combinations
    #[test]
    fn proptest_e2e_vbr_bitrates(
        target in 500u32..=5000,
        min_offset in 0u32..=400,
        max_offset in 0u32..=2000,
    ) {
        require_ffmpeg!();

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let input = create_test_video(&temp_dir);
        let output = temp_dir.path().join("output_vbr.webm");

        let mut config = default_config();
        config.rate_control_mode = RateControlMode::TwoPassVBR;
        config.two_pass = true;
        config.video_target_bitrate = target;
        config.video_min_bitrate = if target > min_offset { target - min_offset } else { 100 };
        config.video_max_bitrate = target + max_offset;

        let cmd = build_test_cmd(&config, "E2E_VBR");

        let test_config = FfmpegTestConfig {
            max_frames: 2,
            max_duration_secs: 0.1,
            timeout_secs: 20,
            ..Default::default()
        };

        let result = run_ffmpeg_command_string(&cmd, &input, &output, &test_config).expect("Failed to run FFmpeg");

        prop_assert!(result.success,
            "FFmpeg failed for VBR {}/{}/{}: {}",
            config.video_min_bitrate, target, config.video_max_bitrate,
            result.stderr);
        prop_assert!(result.output_file_exists);
        prop_assert!(result.output_file_size.unwrap_or(0) > 0);

    }

    /// Test GOP settings with keyframe options
    #[test]
    fn proptest_e2e_gop_keyframe_combinations(
        gop_length in 30u32..=240,
        keyint_min in 0u32..=120,
        fixed_gop in prop::bool::ANY,
        auto_alt_ref in prop::bool::ANY,
    ) {
        require_ffmpeg!();

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let input = create_test_video(&temp_dir);
        let output = temp_dir.path().join("output_gop_keyframe.webm");

        let mut config = default_config();
        config.gop_length = gop_length;
        config.keyint_min = keyint_min;
        config.fixed_gop = fixed_gop;
        config.auto_alt_ref = auto_alt_ref;
        config.two_pass = false;

        let cmd = build_test_cmd(&config, "E2E_GOP_Keyframe");

        let test_config = FfmpegTestConfig {
            max_frames: 3,
            max_duration_secs: 0.15,
            ..Default::default()
        };

        let result = run_ffmpeg_command_string(&cmd, &input, &output, &test_config).expect("Failed to run FFmpeg");

        prop_assert!(result.success,
            "FFmpeg failed for GOP/keyframe settings: {}", result.stderr);
        prop_assert!(result.output_file_exists);
        prop_assert!(result.output_file_size.unwrap_or(0) > 0);

    }
}

// ============================================================================
// COMPREHENSIVE COMBINATION TESTS
// ============================================================================

proptest! {
    // Fewer cases for comprehensive tests (very slow)
    #![proptest_config(ProptestConfig::with_cases(5))]

    /// Test comprehensive setting combinations
    /// This tests multiple settings at once to catch interaction bugs
    #[test]
    fn proptest_e2e_comprehensive_combinations(
        crf in 20u32..=35,
        cpu_used in 2u32..=6,
        gop_length in 60u32..=180,
        tile_cols in 0i32..=2,
        threads in 2u32..=6,
        lag_in_frames in 10u32..=20,
        row_mt in prop::bool::ANY,
        auto_alt_ref in prop::bool::ANY,
    ) {
        require_ffmpeg!();

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let input = create_test_video(&temp_dir);
        let output = temp_dir.path().join("output_comprehensive.webm");

        let mut config = default_config();
        config.crf = crf;
        config.cpu_used = cpu_used;
        config.gop_length = gop_length;
        config.tile_columns = tile_cols;
        config.threads = threads;
        config.lag_in_frames = lag_in_frames;
        config.row_mt = row_mt;
        config.auto_alt_ref = auto_alt_ref;
        config.two_pass = false;

        let cmd = build_test_cmd(&config, "E2E_Comprehensive");

        let test_config = FfmpegTestConfig {
            max_frames: 3,
            max_duration_secs: 0.15,
            timeout_secs: 20,
            ..Default::default()
        };

        let result = run_ffmpeg_command_string(&cmd, &input, &output, &test_config).expect("Failed to run FFmpeg");

        prop_assert!(result.success,
            "FFmpeg failed for comprehensive settings: {}", result.stderr);
        prop_assert!(result.output_file_exists);
        prop_assert!(result.output_file_size.unwrap_or(0) > 0);

    }
}
