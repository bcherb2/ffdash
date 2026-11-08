// Every HEVC setting is accounted for: each HevcConfig field is either exercised by the
// libx265 command or explicitly owned by a hardware Encoder that has not landed yet.
// Hardware Encoder tickets: implement your Encoder's flags, then delete your entries from
// PENDING_HARDWARE (and, for the shared hw_* fields, the entry once any Encoder uses it).

use ffdash::engine::core::{EncodePlan, HostPlatform, Profile, VideoJob, build_encode_cmds};
use ffdash::engine::hardware::VideoEncoder;
use std::path::PathBuf;

/// HevcConfig fields libx265 reads, and the flag each produces.
const X265_FIELDS: &[(&str, &str)] = &[
    ("x265_crf", "-crf"),
    ("x265_preset", "-preset"),
    ("x265_tune", "-tune"),
];

/// Fields no Encoder reads yet, with the Encoder ticket that owns them.
const PENDING_HARDWARE: &[(&str, &str)] = &[
    // shared hardware settings: the first hardware Encoder to use one removes it
    ("hw_preset", "shared hardware"),
    ("hw_lookahead", "shared hardware"),
    ("hw_b_frames", "shared hardware"),
    ("hw_rc_mode", "shared hardware"),
    // per-Encoder quality
    ("nvenc_quality", "hevc_nvenc"),
    ("qsv_quality", "hevc_qsv"),
    ("vaapi_quality", "hevc_vaapi"),
    ("amf_qp", "hevc_amf"),
    ("videotoolbox_q", "hevc_videotoolbox"),
];

fn args(profile: &Profile) -> Vec<String> {
    let job = VideoJob::new(
        PathBuf::from("does-not-exist.mkv"),
        PathBuf::from("/tmp/out.mkv"),
        profile.name.clone(),
    );
    let plan = EncodePlan::new(VideoEncoder::Libx265).with_host(HostPlatform::Linux);
    build_encode_cmds(&job, profile, &plan)
        .pop()
        .unwrap()
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect()
}

/// An x265 profile with every setting off its default.
fn exercised() -> Profile {
    let mut p = Profile::get("hevc-x265");
    let h = p.codec.as_hevc_mut().unwrap();
    h.x265_crf = 21;
    h.x265_preset = "slow".to_string();
    h.x265_tune = "grain".to_string();
    p.color_trc = 16; // PQ: adds -x265-params
    p.pix_fmt = "yuv420p10le".to_string();
    p
}

fn value_of(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1).cloned())
}

#[test]
fn every_hevc_field_is_owned_by_x265_or_a_pending_hardware_encoder() {
    let json = serde_json::to_value(Profile::get("hevc-x265").codec.as_hevc().unwrap()).unwrap();
    for field in json.as_object().unwrap().keys() {
        let owned = X265_FIELDS.iter().any(|(f, _)| f == field)
            || PENDING_HARDWARE.iter().any(|(f, _)| f == field);
        assert!(
            owned,
            "HevcConfig.{field} has no owner in the coverage table"
        );
    }
    // ...and nothing in the table is stale.
    for (field, _) in X265_FIELDS.iter().chain(PENDING_HARDWARE) {
        assert!(json.get(*field).is_some(), "stale coverage entry {field}");
    }
}

#[test]
fn every_x265_field_reaches_its_flag_with_its_value() {
    let args = args(&exercised());
    assert_eq!(value_of(&args, "-crf").as_deref(), Some("21"));
    assert_eq!(value_of(&args, "-preset").as_deref(), Some("slow"));
    assert_eq!(value_of(&args, "-tune").as_deref(), Some("grain"));
    for (field, flag) in X265_FIELDS {
        assert!(args.iter().any(|a| a == flag), "{field}: {flag} missing");
    }
}

#[test]
fn x265_tune_none_omits_the_flag() {
    let args = args(&Profile::get("hevc-x265"));
    assert!(!args.iter().any(|a| a == "-tune"), "{args:?}");
}

#[test]
fn hdr10_signalling_reaches_x265_params() {
    let args = args(&exercised());
    let params = value_of(&args, "-x265-params").expect("-x265-params");
    assert!(params.contains("hdr10=1"), "{params}");
    assert!(params.contains("repeat-headers=1"), "{params}");
    // SDR output carries none of it
    let sdr = self::args(&Profile::get("hevc-x265"));
    assert!(!sdr.iter().any(|a| a == "-x265-params"));
}

#[cfg(feature = "dev-tools")]
#[test]
fn every_libx265_registry_param_has_a_flag_the_command_emits() {
    use ffdash::engine::params::params_for_encoder;
    let args = args(&exercised());
    let mut seen = 0;
    for param in params_for_encoder("libx265") {
        let flag = param
            .get_encoder_config("libx265")
            .and_then(|c| c.flag)
            .expect("registry flag");
        assert!(
            args.iter().any(|a| a == flag),
            "registry param {} ({flag}) is not emitted by the libx265 command",
            param.name
        );
        seen += 1;
    }
    assert!(seen >= 4, "libx265 registry entry looks empty ({seen})");
}

#[cfg(feature = "dev-tools")]
#[test]
fn hevc_registry_clamps_the_x265_crf_to_its_native_range() {
    use ffdash::engine::params::validate_and_clamp_profile;
    let mut p = Profile::get("hevc-x265");
    p.codec.as_hevc_mut().unwrap().x265_crf = 60; // valid for VP9/AV1 (0-63), not for x265
    let clamps = validate_and_clamp_profile(&mut p, "libx265");
    assert!(!clamps.is_empty());
    assert_eq!(p.codec.as_hevc().unwrap().x265_crf, 51);
}
