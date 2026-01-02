use ffdash::engine::Profile;
use ffdash::engine::validate::{HardwareAvailability, ValidationError, validate_profile};

fn assert_err_contains(errs: &[ValidationError], field: &str) {
    assert!(
        errs.iter().any(|e| e.field == field),
        "expected error for field {} but got {:?}",
        field,
        errs
    );
}

#[test]
fn fails_when_av1_qsv_crf_set() {
    let mut profile = Profile::get("av1-qsv");
    profile.crf = 30;

    let result = validate_profile(
        &profile,
        HardwareAvailability {
            av1_qsv: true,
            ..HardwareAvailability::default()
        },
    );

    assert!(result.is_err());
    let errs = result.unwrap_err();
    assert_err_contains(&errs, "crf");
}

#[test]
fn fails_when_hw_not_available() {
    let profile = Profile::get("av1-nvenc");
    let result = validate_profile(
        &profile,
        HardwareAvailability {
            av1_nvenc: false,
            ..HardwareAvailability::default()
        },
    );

    assert!(result.is_err());
    let errs = result.unwrap_err();
    assert_err_contains(&errs, "use_hardware_encoding");
}

#[test]
fn passes_when_sane_defaults() {
    let profile = Profile::get("vp9-good");
    let result = validate_profile(&profile, HardwareAvailability::default());
    assert!(result.is_ok());
}
