/// VAAPI vs Software Parity Tests
///
/// Property-based tests that verify shared parameters appear in both encoder commands.
/// Tests thousands of parameter combinations automatically using proptest.
///
/// These tests catch bugs where a parameter is implemented in one encoder but
/// forgotten in the other.
use ffdash::engine::{HwEncodingConfig, Profile};
use proptest::prelude::*;

use crate::common::helpers::{
    build_software_cmd_for_test, build_vaapi_cmd_for_test, default_hw_config,
};
use crate::common::parameter_mapping::ParameterSupport;
use crate::common::parameter_registry::get_parameter_mappings;

proptest! {
    #[test]
    fn test_parallelism_is_software_only(
        row_mt in prop::bool::ANY,
        tile_cols in 0i32..=4,
        tile_rows in 0i32..=2,
        frame_parallel in prop::bool::ANY,
    ) {
        let mut profile = Profile::get("vp9-good");
        profile.row_mt = row_mt;
        profile.tile_columns = tile_cols;
        profile.tile_rows = tile_rows;
        profile.frame_parallel = frame_parallel;

        let sw_cmd = build_software_cmd_for_test(&profile);
        let hw_cmd = build_vaapi_cmd_for_test(&profile, &default_hw_config());

        // Parallelism should be in software command when enabled
        if row_mt {
            prop_assert!(sw_cmd.contains("-row-mt"), "row_mt missing from software command");
        }
        if tile_cols >= 0 {
            prop_assert!(sw_cmd.contains("-tile-columns"), "tile_columns missing from software command");
        }
        if tile_rows >= 0 {
            prop_assert!(sw_cmd.contains("-tile-rows"), "tile_rows missing from software command");
        }
        if frame_parallel {
            prop_assert!(sw_cmd.contains("-frame-parallel"), "frame_parallel missing from software command");
        }

        // Parallelism should NEVER be in VAAPI command (not supported)
        prop_assert!(!hw_cmd.contains("-row-mt"), "row-mt should NOT be in VAAPI command");
        prop_assert!(!hw_cmd.contains("-tile_columns"), "tile_columns should NOT be in VAAPI command");
        prop_assert!(!hw_cmd.contains("-tile-columns"), "tile-columns should NOT be in VAAPI command");
        prop_assert!(!hw_cmd.contains("-tile_rows"), "tile_rows should NOT be in VAAPI command");
        prop_assert!(!hw_cmd.contains("-tile-rows"), "tile-rows should NOT be in VAAPI command");
        prop_assert!(!hw_cmd.contains("-frame-parallel"), "frame-parallel should NOT be in VAAPI command");
    }

    #[test]
    fn test_shared_audio_settings_in_both_commands(
        audio_bitrate in 64u32..=256,
    ) {
        let mut profile = Profile::get("vp9-good");
        profile.audio_codec = "libopus".to_string();
        profile.audio_bitrate = audio_bitrate;
        profile.video_target_bitrate = 2000;  // VBR mode for VAAPI to allow libopus

        let sw_cmd = build_software_cmd_for_test(&profile);
        let hw_cmd = build_vaapi_cmd_for_test(&profile, &default_hw_config());

        // Audio codec and bitrate should be in both
        prop_assert!(sw_cmd.contains("-c:a"), "audio codec flag missing from software command");
        prop_assert!(hw_cmd.contains("-c:a"), "audio codec flag missing from VAAPI command");

        prop_assert!(sw_cmd.contains("-b:a"), "audio bitrate flag missing from software command");
        prop_assert!(hw_cmd.contains("-b:a"), "audio bitrate flag missing from VAAPI command");
    }

    #[test]
    fn test_shared_gop_settings_in_both_commands(
        gop_length in 30u32..=240,
    ) {
        let mut profile = Profile::get("vp9-good");
        profile.gop_length = gop_length;

        let sw_cmd = build_software_cmd_for_test(&profile);
        let hw_cmd = build_vaapi_cmd_for_test(&profile, &default_hw_config());

        // GOP length should be in both
        prop_assert!(sw_cmd.contains("-g"), "GOP flag missing from software command");
        prop_assert!(hw_cmd.contains("-g"), "GOP flag missing from VAAPI command");

        // The value should match (VAAPI caps at 240, which is our max test value)
        let gop_str = gop_length.to_string();
        prop_assert!(sw_cmd.contains(&gop_str), "GOP value {} missing from software command", gop_length);
        prop_assert!(hw_cmd.contains(&gop_str), "GOP value {} missing from VAAPI command", gop_length);
    }

    #[test]
    fn test_shared_bitrate_settings_in_software_only(
        target_bitrate in 1000u32..=5000,
        max_bitrate in 2000u32..=8000,
        bufsize in 2000u32..=10000,
    ) {
        let mut profile = Profile::get("vp9-good");
        profile.video_target_bitrate = target_bitrate;
        profile.video_max_bitrate = max_bitrate;
        profile.video_bufsize = bufsize;

        let sw_cmd = build_software_cmd_for_test(&profile);
        let hw_cmd = build_vaapi_cmd_for_test(&profile, &default_hw_config());

        // Target bitrate should be in software only (VAAPI forced to CQP)
        if target_bitrate > 0 {
            prop_assert!(sw_cmd.contains("-b:v"), "target bitrate missing from software command");
            prop_assert!(!hw_cmd.contains("-b:v"), "target bitrate should NOT be in VAAPI command (CQP mode)");
        }

        // Maxrate should be in software only
        if max_bitrate > 0 {
            prop_assert!(sw_cmd.contains("-maxrate"), "maxrate missing from software command");
            prop_assert!(!hw_cmd.contains("-maxrate"), "maxrate should NOT be in VAAPI command (CQP mode)");
        }

        // Bufsize should be in software only
        if bufsize > 0 {
            prop_assert!(sw_cmd.contains("-bufsize"), "bufsize missing from software command");
            prop_assert!(!hw_cmd.contains("-bufsize"), "bufsize should NOT be in VAAPI command (CQP mode)");
        }
    }

    #[test]
    fn test_vaapi_hardware_specific_parameters(
        quality in 40u32..=120,
        b_frames in 0u32..=2,
        loop_filter_level in 10u32..=30,
        loop_filter_sharpness in 2u32..=8,
    ) {
        let profile = Profile::get("vp9-good");
        let hw_config = HwEncodingConfig {
            rc_mode: 4,  // ICQ
            global_quality: quality,
            b_frames,
            loop_filter_level,
            loop_filter_sharpness,
            compression_level: 4,
        };

        let vaapi_cmd = build_vaapi_cmd_for_test(&profile, &hw_config);
        let sw_cmd = build_software_cmd_for_test(&profile);

        // VAAPI-specific parameters should only be in VAAPI command
        prop_assert!(vaapi_cmd.contains("-global_quality"),
            "global_quality missing from VAAPI command");
        prop_assert!(!sw_cmd.contains("-global_quality"),
            "global_quality should NOT be in software command");

        prop_assert!(vaapi_cmd.contains("-loop_filter_level"),
            "loop_filter_level missing from VAAPI command");
        prop_assert!(!sw_cmd.contains("-loop_filter_level"),
            "loop_filter_level should NOT be in software command");

        prop_assert!(vaapi_cmd.contains("-loop_filter_sharpness"),
            "loop_filter_sharpness missing from VAAPI command");
        prop_assert!(!sw_cmd.contains("-loop_filter_sharpness"),
            "loop_filter_sharpness should NOT be in software command");

        // B-frames only when > 0
        if b_frames > 0 {
            prop_assert!(vaapi_cmd.contains("-bf"),
                "b_frames missing from VAAPI command when b_frames > 0");
        }
    }

    #[test]
    fn test_software_only_parameters_not_in_vaapi(
        crf in 20u32..=50,
        cpu_used in 0u32..=5,
        threads in 1u32..=16,
    ) {
        let mut profile = Profile::get("vp9-good");
        profile.crf = crf;
        profile.cpu_used = cpu_used;
        profile.threads = threads;

        let sw_cmd = build_software_cmd_for_test(&profile);
        let vaapi_cmd = build_vaapi_cmd_for_test(&profile, &default_hw_config());

        // These parameters should only be in software command
        prop_assert!(sw_cmd.contains("-crf"),
            "CRF missing from software command");
        prop_assert!(!vaapi_cmd.contains("-crf"),
            "CRF should NOT be in VAAPI command (uses global_quality instead)");

        prop_assert!(sw_cmd.contains("-cpu-used"),
            "cpu-used missing from software command");
        prop_assert!(!vaapi_cmd.contains("-cpu-used"),
            "cpu-used should NOT be in VAAPI command (GPU encoding)");

        if threads > 0 {
            prop_assert!(sw_cmd.contains("-threads"),
                "threads missing from software command");
            prop_assert!(!vaapi_cmd.contains("-threads"),
                "threads should NOT be in VAAPI command (GPU parallelism)");
        }
    }
}

/// Comprehensive parity test using the parameter registry
/// This test uses the registry to automatically verify all shared parameters
#[test]
fn test_comprehensive_parameter_parity() {
    let mappings = get_parameter_mappings();

    // Create a profile with many settings enabled
    let mut profile = Profile::get("vp9-good");
    profile.row_mt = true;
    profile.tile_columns = 2;
    profile.tile_rows = 1;
    profile.frame_parallel = true;
    profile.gop_length = 120;
    profile.audio_codec = "libopus".to_string();
    profile.audio_bitrate = 128;
    profile.video_target_bitrate = 2000;
    profile.video_max_bitrate = 3000;
    profile.video_bufsize = 4000;

    // Use VBR mode for VAAPI (rc_mode=3) so bitrate params are included
    let mut hw_config = default_hw_config();
    hw_config.rc_mode = 3; // VBR mode

    let sw_cmd = build_software_cmd_for_test(&profile);
    let vaapi_cmd = build_vaapi_cmd_for_test(&profile, &hw_config);

    println!("Software command:\n{}\n", sw_cmd);
    println!("VAAPI command:\n{}\n", vaapi_cmd);

    let mut violations = Vec::new();

    // Check all shared parameters
    for mapping in mappings.iter() {
        if matches!(mapping.support, ParameterSupport::Both) {
            // Skip filter-based parameters (complex to verify)
            if let Some(flag) = mapping.vaapi_flag {
                if flag.contains("scale") || flag.contains("fps") {
                    continue;
                }

                // If parameter should be included and is in software, it MUST be in VAAPI
                if mapping.should_be_in_vaapi(&profile) {
                    let sw_has_flag = if let Some(sw_flag) = mapping.software_flag {
                        sw_cmd.contains(sw_flag)
                    } else {
                        false
                    };

                    let vaapi_has_flag = vaapi_cmd.contains(flag);

                    if sw_has_flag && !vaapi_has_flag {
                        violations.push(format!(
                            "Parameter '{}': Present in software (flag: '{}') but MISSING from VAAPI (expected flag: '{}')",
                            mapping.field_name,
                            mapping.software_flag.unwrap_or("N/A"),
                            flag
                        ));
                    }
                }
            }
        }
    }

    if !violations.is_empty() {
        panic!(
            "\nPARITY VIOLATIONS DETECTED:\n\n{}\n\n\
            These parameters are marked as shared (Both) but are missing from VAAPI!\n",
            violations.join("\n")
        );
    }
}

/// Test that verifies parallelism is software-only
#[test]
fn test_parallelism_is_not_shared_between_encoders() {
    // Parallelism (tile-based) is libvpx-vp9 specific
    // VAAPI VP9 encoder does NOT support these parameters
    // This test ensures we don't incorrectly add them to VAAPI

    let mut profile = Profile::get("vp9-good");
    profile.row_mt = true;
    profile.tile_columns = 3;
    profile.tile_rows = 2;
    profile.frame_parallel = true;

    let sw_cmd = build_software_cmd_for_test(&profile);
    let vaapi_cmd = build_vaapi_cmd_for_test(&profile, &default_hw_config());

    // Software command MUST have parallelism settings
    assert!(sw_cmd.contains("-row-mt"), "Software should have row-mt");
    assert!(
        sw_cmd.contains("-tile-columns"),
        "Software should have tile-columns"
    );
    assert!(
        sw_cmd.contains("-tile-rows"),
        "Software should have tile-rows"
    );
    assert!(
        sw_cmd.contains("-frame-parallel"),
        "Software should have frame-parallel"
    );

    // VAAPI command MUST NOT have them (not supported by VAAPI VP9)
    assert!(
        !vaapi_cmd.contains("-row-mt"),
        "VAAPI should NOT have row-mt (not supported)"
    );
    assert!(
        !vaapi_cmd.contains("-tile_columns"),
        "VAAPI should NOT have tile_columns (not supported)"
    );
    assert!(
        !vaapi_cmd.contains("-tile-columns"),
        "VAAPI should NOT have tile-columns (not supported)"
    );
    assert!(
        !vaapi_cmd.contains("-tile_rows"),
        "VAAPI should NOT have tile_rows (not supported)"
    );
    assert!(
        !vaapi_cmd.contains("-tile-rows"),
        "VAAPI should NOT have tile-rows (not supported)"
    );
    assert!(
        !vaapi_cmd.contains("-frame-parallel"),
        "VAAPI should NOT have frame-parallel (not supported)"
    );

    println!("\n✅ Parallelism separation test PASSED");
    println!("   Verified that tile-based parallelism is libvpx-vp9 specific");
    println!("   VAAPI VP9 doesn't support these parameters\n");
}
