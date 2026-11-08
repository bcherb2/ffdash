//! Quality Override: the override lands in the active Encoder's quality flag and the
//! configured value does not.

use ffdash::engine::core::{EncodePlan, Profile, VideoJob, build_encode_cmds};
use ffdash::engine::hardware::VideoEncoder;
use std::path::PathBuf;

fn args_with_override(profile: &Profile, encoder: VideoEncoder, quality: u32) -> Vec<String> {
    let job = VideoJob::new(
        PathBuf::from("/tmp/input.mp4"),
        PathBuf::from("/tmp/output.mkv"),
        profile.name.clone(),
    );
    let plan = EncodePlan::new(encoder).with_quality_override(quality);
    build_encode_cmds(&job, profile, &plan)
        .pop()
        .unwrap()
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect()
}

fn value_of(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1).cloned())
}

#[test]
fn override_replaces_active_encoder_quality() {
    // (profile, encoder, flag carrying quality)
    let cases = [
        ("av1-svt", VideoEncoder::LibsvtAv1, "-crf"),
        ("av1-qsv", VideoEncoder::Av1Qsv, "-q:v"),
        ("av1-nvenc", VideoEncoder::Av1Nvenc, "-cq"),
        ("av1-vaapi", VideoEncoder::Av1Vaapi, "-global_quality:v"),
        ("vp9-good", VideoEncoder::LibvpxVp9, "-crf"),
        ("hevc-x265", VideoEncoder::Libx265, "-crf"),
        ("hevc-qsv", VideoEncoder::HevcQsv, "-q:v"),
        ("hevc-nvenc", VideoEncoder::HevcNvenc, "-cq"),
    ];
    for (name, encoder, flag) in cases {
        let profile = Profile::get(name);
        let baseline = args_with_override(&profile, encoder, 33);
        let got = value_of(&baseline, flag)
            .unwrap_or_else(|| panic!("{name}: no {flag} in {baseline:?}"));
        assert_eq!(got, "33", "{name}: override must reach {flag}");
    }
}

#[test]
fn override_replaces_hardware_vp9_quality() {
    for (name, encoder, flag) in [
        ("vp9-vaapi", VideoEncoder::Vp9Vaapi, "-global_quality:v"),
        ("vp9-qsv", VideoEncoder::Vp9Qsv, "-q:v"),
    ] {
        let profile = Profile::get(name);
        let args = args_with_override(&profile, encoder, 91);
        let got = value_of(&args, flag).unwrap_or_else(|| panic!("{name}: no {flag} in {args:?}"));
        assert_eq!(got, "91", "{name}");
    }
}

#[test]
fn configured_quality_is_absent_when_overridden() {
    // Configured values deliberately differ from the override.
    let mut av1 = Profile::get("av1-vaapi");
    av1.codec.as_av1_mut().unwrap().vaapi_cq = 77;
    av1.codec.as_av1_mut().unwrap().hw_cq = 88;
    let args = args_with_override(&av1, VideoEncoder::Av1Vaapi, 33);
    assert_eq!(value_of(&args, "-global_quality:v").as_deref(), Some("33"));
    assert!(!args.iter().any(|a| a == "77" || a == "88"), "{args:?}");
}

#[test]
fn active_quality_reads_the_active_encoders_field_not_legacy_hw_cq() {
    let mut profile = Profile::get("av1-nvenc");
    let av1 = profile.codec.as_av1_mut().unwrap();
    av1.nvenc_cq = 21;
    av1.qsv_cq = 44;
    av1.vaapi_cq = 55;
    av1.hw_cq = 99;
    assert_eq!(profile.active_quality(VideoEncoder::Av1Nvenc), 21);
    assert_eq!(profile.active_quality(VideoEncoder::Av1Qsv), 44);
    assert_eq!(profile.active_quality(VideoEncoder::Av1Vaapi), 55);
}

#[test]
fn hevc_override_replaces_x265_crf_and_not_the_configured_value() {
    let mut profile = Profile::get("hevc-x265");
    profile.codec.as_hevc_mut().unwrap().x265_crf = 19;
    profile.crf = 77;
    let args = args_with_override(&profile, VideoEncoder::Libx265, 33);
    assert_eq!(value_of(&args, "-crf").as_deref(), Some("33"));
    assert!(!args.iter().any(|a| a == "19" || a == "77"), "{args:?}");
    assert_eq!(profile.active_quality(VideoEncoder::Libx265), 19);
}

#[test]
fn hevc_qsv_override_replaces_qsv_quality_and_not_the_configured_value() {
    let mut profile = Profile::get("hevc-qsv");
    profile.codec.as_hevc_mut().unwrap().qsv_quality = 19;
    profile.crf = 77;
    let args = args_with_override(&profile, VideoEncoder::HevcQsv, 33);
    assert_eq!(value_of(&args, "-q:v").as_deref(), Some("33"));
    assert!(!args.iter().any(|a| a == "19" || a == "77"), "{args:?}");
    assert_eq!(profile.active_quality(VideoEncoder::HevcQsv), 19);
}

#[test]
fn hevc_nvenc_override_replaces_configured_quality() {
    let mut profile = Profile::get("hevc-nvenc");
    profile.codec.as_hevc_mut().unwrap().nvenc_quality = 19;
    profile.crf = 77;
    let args = args_with_override(&profile, VideoEncoder::HevcNvenc, 33);
    assert_eq!(value_of(&args, "-cq").as_deref(), Some("33"));
    assert!(!args.iter().any(|a| a == "19" || a == "77"), "{args:?}");
    assert_eq!(profile.active_quality(VideoEncoder::HevcNvenc), 19);
}

#[test]
fn hevc_vaapi_override_replaces_its_quality_flag_and_not_the_configured_value() {
    let mut profile = Profile::get("hevc-vaapi");
    profile.codec.as_hevc_mut().unwrap().vaapi_quality = 19;
    profile.crf = 77;
    let args = args_with_override(&profile, VideoEncoder::HevcVaapi, 33);
    assert_eq!(value_of(&args, "-global_quality:v").as_deref(), Some("33"));
    assert!(!args.iter().any(|a| a == "19" || a == "77"), "{args:?}");
    assert_eq!(profile.active_quality(VideoEncoder::HevcVaapi), 19);
}
