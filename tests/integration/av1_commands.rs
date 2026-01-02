use ffdash::engine::{Codec, Profile};

use crate::common::assertions::assert_cmd_contains;
use crate::common::helpers::{
    build_av1_nvenc_cmd_for_test, build_av1_qsv_cmd_for_test,
    build_av1_software_cmd_for_test, build_av1_vaapi_cmd_for_test,
};

#[test]
fn test_av1_svt_command_contains_core_flags() {
    let profile = Profile::get("av1-svt");
    let cmd = build_av1_software_cmd_for_test(&profile);

    assert_cmd_contains(&cmd, "-c:v libsvtav1");
    assert_cmd_contains(&cmd, "-crf");
    assert_cmd_contains(&cmd, "-preset");
    assert_cmd_contains(&cmd, "-g:v");
    assert_cmd_contains(&cmd, "-pix_fmt");
}

#[test]
fn test_av1_qsv_command_contains_hw_flags() {
    let profile = Profile::get("av1-qsv");
    let cmd = build_av1_qsv_cmd_for_test(&profile);

    assert_cmd_contains(&cmd, "-c:v av1_qsv");
    assert_cmd_contains(&cmd, "-rc_mode");
    assert_cmd_contains(&cmd, "-q:v");
    assert_cmd_contains(&cmd, "-init_hw_device");
    assert_cmd_contains(&cmd, "-g:v");
}

#[test]
fn test_av1_nvenc_command_contains_hw_flags() {
    let profile = Profile::get("av1-nvenc");
    let cmd = build_av1_nvenc_cmd_for_test(&profile);

    assert_cmd_contains(&cmd, "-c:v av1_nvenc");
    assert_cmd_contains(&cmd, "-rc vbr");
    assert_cmd_contains(&cmd, "-cq");
    assert_cmd_contains(&cmd, "-preset");
}

#[test]
fn test_av1_vaapi_command_contains_hw_flags() {
    let profile = Profile::get("av1-vaapi");
    let cmd = build_av1_vaapi_cmd_for_test(&profile);

    assert_cmd_contains(&cmd, "-c:v av1_vaapi");
    assert_cmd_contains(&cmd, "-global_quality:v");
    assert_cmd_contains(&cmd, "-init_hw_device");
    assert_cmd_contains(&cmd, "-vf");
}

#[test]
fn test_av1_svt_film_grain_denoise() {
    let mut profile = Profile::get("av1-svt");

    // Enable film grain with denoise
    if let Codec::Av1(ref mut cfg) = profile.codec {
        cfg.film_grain = 10;
        cfg.film_grain_denoise = true;
    }

    let cmd = build_av1_software_cmd_for_test(&profile);

    // Should contain film-grain and film-grain-denoise in svtav1-params
    assert_cmd_contains(&cmd, "film-grain=10");
    assert_cmd_contains(&cmd, "film-grain-denoise=1");
}

#[test]
fn test_av1_svt_film_grain_without_denoise() {
    let mut profile = Profile::get("av1-svt");

    // Enable film grain without denoise
    if let Codec::Av1(ref mut cfg) = profile.codec {
        cfg.film_grain = 15;
        cfg.film_grain_denoise = false;
    }

    let cmd = build_av1_software_cmd_for_test(&profile);

    // Should contain film-grain but NOT film-grain-denoise
    assert_cmd_contains(&cmd, "film-grain=15");
    assert!(!cmd.contains("film-grain-denoise"), "Should not contain film-grain-denoise when disabled");
}

#[test]
fn test_av1_svt_no_film_grain_denoise_when_grain_zero() {
    let mut profile = Profile::get("av1-svt");

    // film_grain=0 but denoise=true - denoise should NOT appear
    if let Codec::Av1(ref mut cfg) = profile.codec {
        cfg.film_grain = 0;
        cfg.film_grain_denoise = true;
    }

    let cmd = build_av1_software_cmd_for_test(&profile);

    // Should NOT contain film-grain-denoise when film_grain is 0
    assert!(!cmd.contains("film-grain-denoise"), "Should not contain film-grain-denoise when film_grain=0");
}
