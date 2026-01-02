/// Parameter Coverage Tests
///
/// These tests verify that FFmpeg parameters are correctly included in commands
/// based on the parameter registry. This catches bugs where parameters are added
/// to one encoder but forgotten in another.
///
/// The parallelism bug (row_mt, tile_columns, tile_rows, frame_parallel missing
/// from VAAPI) would have been caught by test_vaapi_command_covers_shared_parameters().
use ffdash::engine::{HwEncodingConfig, Profile};

use crate::common::assertions::assert_cmd_contains;
use crate::common::helpers::{
    build_software_cmd_for_test, build_vaapi_cmd_for_test, default_hw_config,
};
use crate::common::parameter_mapping::ParameterSupport;
use crate::common::parameter_registry::{get_parameter_mappings, get_parameter_statistics};

#[test]
fn test_parameter_registry_is_complete() {
    let stats = get_parameter_statistics();

    println!("Parameter Registry Statistics:");
    println!("  Total parameters: {}", stats.total);
    println!("  Both encoders: {}", stats.both);
    println!("  Software-only: {}", stats.software_only);
    println!("  VAAPI-only: {}", stats.vaapi_only);
    println!("  Not applicable: {}", stats.not_applicable);

    // Regression test: We know there are at least these many parameters
    assert!(
        stats.total >= 45,
        "Parameter registry has only {} entries, expected at least 45",
        stats.total
    );

    // We know there are at least these shared parameters
    // (Bitrate settings are software-only because VAAPI is forced to CQP mode.)
    assert!(
        stats.both >= 6,
        "Expected at least 6 shared parameters (audio_codec, audio_bitrate, fps, scale, gop_length), found {}",
        stats.both
    );
}

#[test]
fn test_critical_shared_parameters_in_registry() {
    let mappings = get_parameter_mappings();

    // These parameters MUST be marked as Both (shared between encoders)
    // If any are missing or marked differently, it's a bug
    let critical_both = vec![
        "audio_codec",   // Audio settings
        "audio_bitrate", // Audio settings
        "gop_length",    // GOP settings
    ];

    for field_name in critical_both {
        let mapping = mappings
            .iter()
            .find(|m| m.field_name == field_name)
            .unwrap_or_else(|| panic!("Critical parameter '{}' missing from registry", field_name));

        assert_eq!(
            mapping.support,
            ParameterSupport::Both,
            "Parameter '{}' should be ParameterSupport::Both (works in software AND VAAPI)",
            field_name
        );
    }

    // Verify parallelism parameters are correctly marked as SoftwareOnly
    let critical_software_only = vec![
        "row_mt",         // Parallelism - libvpx-vp9 only
        "tile_columns",   // Parallelism - libvpx-vp9 only
        "tile_rows",      // Parallelism - libvpx-vp9 only
        "frame_parallel", // Parallelism - libvpx-vp9 only
    ];

    for field_name in critical_software_only {
        let mapping = mappings
            .iter()
            .find(|m| m.field_name == field_name)
            .unwrap_or_else(|| panic!("Critical parameter '{}' missing from registry", field_name));

        assert_eq!(
            mapping.support,
            ParameterSupport::SoftwareOnly,
            "Parameter '{}' should be ParameterSupport::SoftwareOnly (VAAPI doesn't support tile parallelism)",
            field_name
        );
    }
}

#[test]
fn test_vaapi_command_covers_shared_parameters() {
    // THIS IS THE CRITICAL TEST THAT CATCHES THE BUG!
    //
    // This test verifies that ALL parameters marked as ParameterSupport::Both
    // appear in VAAPI commands when they should.
    //
    // The parallelism bug would have FAILED this test because row_mt,
    // tile_columns, tile_rows, and frame_parallel were marked as Both
    // but were MISSING from the VAAPI command.

    let mappings = get_parameter_mappings();
    let mut profile = Profile::get("vp9-good");

    let hw_config = default_hw_config();

    // Set test values that trigger parameter inclusion
    profile.row_mt = true;
    profile.tile_columns = 2;
    profile.tile_rows = 1;
    profile.frame_parallel = true;

    let cmd = build_vaapi_cmd_for_test(&profile, &hw_config);

    println!("VAAPI command:\n{}\n", cmd);

    let mut missing_params = Vec::new();

    // Check each ParameterSupport::Both parameter
    for mapping in mappings.iter() {
        if matches!(mapping.support, ParameterSupport::Both) {
            if mapping.should_be_in_vaapi(&profile) {
                if let Some(flag) = mapping.vaapi_flag {
                    // Skip filter-based parameters (they're complex to assert)
                    if flag.contains("scale") || flag.contains("fps") {
                        continue;
                    }

                    if !cmd.contains(flag) {
                        missing_params.push(format!(
                            "  - Flag '{}' for parameter '{}' (marked as Both but missing from VAAPI)",
                            flag, mapping.field_name
                        ));
                    }
                }
            }
        }
    }

    if !missing_params.is_empty() {
        panic!(
            "VAAPI PARAMETER BUG DETECTED!\n\n\
            The following parameters are marked as ParameterSupport::Both (should work in\n\
            software AND VAAPI) but are MISSING from the VAAPI command:\n\n{}\n\n\
            This is exactly the type of bug that affected parallelism settings!\n\
            FIX: Add these parameters to build_vaapi_cmd() in src/engine/mod.rs\n",
            missing_params.join("\n")
        );
    }
}

#[test]
fn test_vaapi_only_parameters_in_vaapi_command() {
    let profile = Profile::get("vp9-good");
    let hw_config = HwEncodingConfig {
        rc_mode: 4, // ICQ
        global_quality: 70,
        b_frames: 1, // Enable B-frames to test
        loop_filter_level: 20,
        loop_filter_sharpness: 5,
        compression_level: 4,
    };

    let cmd = build_vaapi_cmd_for_test(&profile, &hw_config);

    println!("VAAPI command (with B-frames):\n{}\n", cmd);

    // Specifically check hardware parameters
    assert_cmd_contains(&cmd, "-global_quality");
    assert_cmd_contains(&cmd, "-loop_filter_level");
    assert_cmd_contains(&cmd, "-loop_filter_sharpness");
    assert_cmd_contains(&cmd, "-bf");
}

#[test]
fn test_software_only_parameters_not_in_vaapi() {
    let mappings = get_parameter_mappings();
    let mut profile = Profile::get("vp9-good");
    let hw_config = default_hw_config();

    // Enable software-only parameters
    profile.arnr_max_frames = 7;
    profile.enable_tpl = true;
    profile.tune_content = "screen".to_string();

    let cmd = build_vaapi_cmd_for_test(&profile, &hw_config);

    // Software-only parameters should NOT appear in VAAPI command
    for mapping in mappings.iter() {
        if matches!(mapping.support, ParameterSupport::SoftwareOnly) {
            if let Some(flag) = mapping.software_flag {
                assert!(
                    !cmd.contains(flag),
                    "VAAPI command should NOT contain software-only flag '{}' (parameter: '{}')",
                    flag,
                    mapping.field_name
                );
            }
        }
    }
}

#[test]
fn test_parallelism_is_software_only() {
    // Parallelism (tile-based) is libvpx-vp9 specific
    // VAAPI VP9 encoder does NOT support these parameters

    let mut profile = Profile::get("vp9-good");
    profile.row_mt = true;
    profile.tile_columns = 2;
    profile.tile_rows = 1;
    profile.frame_parallel = true;

    // Software command MUST have parallelism parameters
    let sw_cmd = build_software_cmd_for_test(&profile);
    println!("Software parallelism test:\n{}\n", sw_cmd);
    assert_cmd_contains(&sw_cmd, "-row-mt");
    assert_cmd_contains(&sw_cmd, "-tile-columns");
    assert_cmd_contains(&sw_cmd, "-tile-rows");
    assert_cmd_contains(&sw_cmd, "-frame-parallel");

    // VAAPI command MUST NOT have parallelism parameters
    let hw_config = default_hw_config();
    let vaapi_cmd = build_vaapi_cmd_for_test(&profile, &hw_config);
    println!("VAAPI parallelism test:\n{}\n", vaapi_cmd);
    assert!(!vaapi_cmd.contains("-row-mt"));
    assert!(!vaapi_cmd.contains("-tile_columns"));
    assert!(!vaapi_cmd.contains("-tile-columns"));
    assert!(!vaapi_cmd.contains("-tile_rows"));
    assert!(!vaapi_cmd.contains("-tile-rows"));
    assert!(!vaapi_cmd.contains("-frame-parallel"));
}

#[test]
fn test_software_uses_hyphens_for_tile_parameters() {
    // libvpx-vp9 uses hyphens for tile parameters
    // (VAAPI doesn't support tile parameters at all)

    let mut profile = Profile::get("vp9-good");
    profile.tile_columns = 3;
    profile.tile_rows = 2;

    let sw_cmd = build_software_cmd_for_test(&profile);
    assert_cmd_contains(&sw_cmd, "-tile-columns");
    assert_cmd_contains(&sw_cmd, "-tile-rows");
}

#[test]
fn test_audio_settings_in_both_commands() {
    let mut profile = Profile::get("vp9-good");
    profile.audio_primary_codec = "libopus".to_string();
    profile.audio_primary_bitrate = 128;

    // Test software command
    let sw_cmd = build_software_cmd_for_test(&profile);
    assert_cmd_contains(&sw_cmd, "-c:a");
    assert_cmd_contains(&sw_cmd, "-b:a");
    assert_cmd_contains(&sw_cmd, "libopus");

    // Test VAAPI command (VBR mode to allow libopus)
    profile.video_target_bitrate = 2000; // Enable VBR mode
    let hw_config = default_hw_config();
    let vaapi_cmd = build_vaapi_cmd_for_test(&profile, &hw_config);
    assert_cmd_contains(&vaapi_cmd, "-c:a");
    assert_cmd_contains(&vaapi_cmd, "-b:a");
}

#[test]
fn test_gop_settings_in_both_commands() {
    let mut profile = Profile::get("vp9-good");
    profile.gop_length = 120.to_string();

    // Test software command
    let sw_cmd = build_software_cmd_for_test(&profile);
    assert_cmd_contains(&sw_cmd, "-g");
    assert_cmd_contains(&sw_cmd, "120");

    // Test VAAPI command
    let hw_config = default_hw_config();
    let vaapi_cmd = build_vaapi_cmd_for_test(&profile, &hw_config);
    assert_cmd_contains(&vaapi_cmd, "-g");
    // VAAPI caps GOP at 240, so 120 should pass through
    assert_cmd_contains(&vaapi_cmd, "120");
}
