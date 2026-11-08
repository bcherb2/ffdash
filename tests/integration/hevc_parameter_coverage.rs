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

/// HevcConfig fields hevc_qsv reads, and the flag each produces. The shared hardware
/// settings (hw_*) are owned here once any hardware Encoder reads them.
const QSV_FIELDS: &[(&str, &str)] = &[
    ("qsv_quality", "-q:v"),
    ("hw_preset", "-preset"),
    ("hw_lookahead", "-look_ahead_depth"),
    ("hw_b_frames", "-bf"),
    ("hw_rc_mode", "-q:v"), // "cq" is the only mode: -q:v selects CQP
];

/// HevcConfig fields hevc_nvenc reads, and the flag each produces.
const NVENC_FIELDS: &[(&str, &str)] = &[
    ("nvenc_quality", "-cq"),
    ("hw_preset", "-preset"),
    ("hw_lookahead", "-rc-lookahead"),
    ("hw_b_frames", "-bf"),
];

/// HevcConfig fields hevc_amf reads, and the flag each produces.
const AMF_FIELDS: &[(&str, &str)] = &[
    ("amf_qp", "-qp_i"),
    ("amf_qp", "-qp_p"),
    ("hw_preset", "-quality"),
];

/// HevcConfig fields hevc_videotoolbox reads, and the flag each produces.
const VIDEOTOOLBOX_FIELDS: &[(&str, &str)] = &[("videotoolbox_q", "-q:v")];

/// Fields no Encoder reads yet, with the Encoder ticket that owns them.
const PENDING_HARDWARE: &[(&str, &str)] = &[];

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
            || QSV_FIELDS.iter().any(|(f, _)| f == field)
            || NVENC_FIELDS.iter().any(|(f, _)| f == field)
            || AMF_FIELDS.iter().any(|(f, _)| f == field)
            || VIDEOTOOLBOX_FIELDS.iter().any(|(f, _)| f == field)
            || VAAPI_FIELDS.iter().any(|(f, _)| f == field)
            || PENDING_HARDWARE.iter().any(|(f, _)| f == field);
        assert!(
            owned,
            "HevcConfig.{field} has no owner in the coverage table"
        );
    }
    // ...and nothing in the table is stale.
    for (field, _) in X265_FIELDS
        .iter()
        .chain(QSV_FIELDS)
        .chain(NVENC_FIELDS)
        .chain(AMF_FIELDS)
        .chain(VIDEOTOOLBOX_FIELDS)
        .chain(VAAPI_FIELDS)
        .chain(PENDING_HARDWARE)
    {
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

fn hardware_args(profile: &Profile, encoder: VideoEncoder) -> Vec<String> {
    let job = VideoJob::new(
        PathBuf::from("does-not-exist.mkv"),
        PathBuf::from("/tmp/out.mkv"),
        profile.name.clone(),
    );
    build_encode_cmds(&job, profile, &EncodePlan::new(encoder))
        .pop()
        .unwrap()
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect()
}

#[test]
fn every_amf_field_reaches_its_flag_with_its_value() {
    let mut p = Profile::get("hevc-amf");
    let h = p.codec.as_hevc_mut().unwrap();
    h.amf_qp = 23;
    h.hw_preset = "7".to_string();
    let args = hardware_args(&p, VideoEncoder::HevcAmf);
    assert_eq!(value_of(&args, "-qp_i").as_deref(), Some("23"));
    assert_eq!(value_of(&args, "-qp_p").as_deref(), Some("23"));
    assert_eq!(value_of(&args, "-quality").as_deref(), Some("speed"));
    for (field, flag) in AMF_FIELDS {
        assert!(args.iter().any(|a| a == flag), "{field}: {flag} missing");
    }
}

#[test]
fn every_videotoolbox_field_reaches_its_flag_with_its_value() {
    let mut p = Profile::get("hevc-videotoolbox");
    p.codec.as_hevc_mut().unwrap().videotoolbox_q = 77;
    let args = hardware_args(&p, VideoEncoder::HevcVideotoolbox);
    assert_eq!(value_of(&args, "-q:v").as_deref(), Some("77"));
}

#[cfg(feature = "dev-tools")]
#[test]
fn amf_and_videotoolbox_registry_params_have_a_flag_the_command_emits() {
    use ffdash::engine::params::params_for_encoder;
    for (id, profile, encoder) in [
        ("hevc_amf", "hevc-amf", VideoEncoder::HevcAmf),
        (
            "hevc_videotoolbox",
            "hevc-videotoolbox",
            VideoEncoder::HevcVideotoolbox,
        ),
    ] {
        let args = hardware_args(&Profile::get(profile), encoder);
        let mut seen = 0;
        for param in params_for_encoder(id) {
            let flag = param
                .get_encoder_config(id)
                .and_then(|c| c.flag)
                .expect("registry flag");
            assert!(
                args.iter().any(|a| a == flag),
                "{id}: registry param {} ({flag}) is not emitted",
                param.name
            );
            seen += 1;
        }
        assert!(seen >= 1, "{id} registry entry looks empty");
    }
}

#[cfg(feature = "dev-tools")]
#[test]
fn registry_clamps_amf_qp_and_videotoolbox_q_to_their_native_ranges() {
    use ffdash::engine::params::validate_and_clamp_profile;
    let mut amf = Profile::get("hevc-amf");
    amf.codec.as_hevc_mut().unwrap().amf_qp = 90;
    assert!(!validate_and_clamp_profile(&mut amf, "hevc_amf").is_empty());
    assert_eq!(amf.codec.as_hevc().unwrap().amf_qp, 51);

    let mut vt = Profile::get("hevc-videotoolbox");
    vt.codec.as_hevc_mut().unwrap().videotoolbox_q = 250;
    assert!(!validate_and_clamp_profile(&mut vt, "hevc_videotoolbox").is_empty());
    assert_eq!(vt.codec.as_hevc().unwrap().videotoolbox_q, 100);
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

/// Fields hevc_vaapi reads, and the flag each produces. (hw_preset and hw_lookahead are
/// not supported by hevc_vaapi and stay unowned by it.)
const VAAPI_FIELDS: &[(&str, &str)] = &[
    ("vaapi_quality", "-global_quality:v"),
    ("hw_b_frames", "-bf:v"),
    ("hw_rc_mode", "-rc_mode:v"),
];

#[test]
fn every_hevc_vaapi_field_reaches_its_flag_with_its_value() {
    let mut p = Profile::get("hevc-vaapi");
    let h = p.codec.as_hevc_mut().unwrap();
    h.vaapi_quality = 23;
    h.hw_b_frames = 3;
    let job = VideoJob::new(
        PathBuf::from("does-not-exist.mkv"),
        PathBuf::from("/tmp/out.mkv"),
        p.name.clone(),
    );
    let plan = EncodePlan::new(VideoEncoder::HevcVaapi).with_host(HostPlatform::Linux);
    let args: Vec<String> = build_encode_cmds(&job, &p, &plan)
        .pop()
        .unwrap()
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect();
    assert_eq!(value_of(&args, "-global_quality:v").as_deref(), Some("23"));
    assert_eq!(value_of(&args, "-bf:v").as_deref(), Some("3"));
    assert_eq!(value_of(&args, "-rc_mode:v").as_deref(), Some("CQP"));
    for (field, flag) in VAAPI_FIELDS {
        assert!(args.iter().any(|a| a == flag), "{field}: {flag} missing");
    }
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

fn qsv_args(host: HostPlatform, profile: &Profile) -> Vec<String> {
    let job = VideoJob::new(
        PathBuf::from("does-not-exist.mkv"),
        PathBuf::from("/tmp/out.mkv"),
        profile.name.clone(),
    );
    let plan = EncodePlan::new(VideoEncoder::HevcQsv).with_host(host);
    build_encode_cmds(&job, profile, &plan)
        .pop()
        .unwrap()
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect()
}

fn nvenc_args(profile: &Profile) -> Vec<String> {
    let job = VideoJob::new(
        PathBuf::from("does-not-exist.mkv"),
        PathBuf::from("/tmp/out.mkv"),
        profile.name.clone(),
    );
    let plan = EncodePlan::new(VideoEncoder::HevcNvenc).with_host(HostPlatform::Linux);
    build_encode_cmds(&job, profile, &plan)
        .pop()
        .unwrap()
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect()
}

#[test]
fn every_hevc_qsv_field_reaches_its_flag_with_its_value() {
    let mut p = Profile::get("hevc-qsv");
    let h = p.codec.as_hevc_mut().unwrap();
    h.qsv_quality = 22;
    h.hw_preset = "2".to_string();
    h.hw_lookahead = 55;
    h.hw_b_frames = 3;
    for host in [HostPlatform::Linux, HostPlatform::Windows] {
        let args = qsv_args(host, &p);
        assert_eq!(value_of(&args, "-q:v").as_deref(), Some("22"), "{host:?}");
        assert_eq!(value_of(&args, "-preset").as_deref(), Some("slower"));
        assert_eq!(value_of(&args, "-look_ahead_depth").as_deref(), Some("55"));
        assert_eq!(value_of(&args, "-bf").as_deref(), Some("3"));
        for (field, flag) in QSV_FIELDS {
            assert!(args.iter().any(|a| a == flag), "{field}: {flag} missing");
        }
    }
}

#[test]
fn hevc_qsv_lookahead_zero_omits_the_flags() {
    let mut p = Profile::get("hevc-qsv");
    p.codec.as_hevc_mut().unwrap().hw_lookahead = 0;
    let args = qsv_args(HostPlatform::Linux, &p);
    assert!(
        !args.iter().any(|a| a.starts_with("-look_ahead")),
        "{args:?}"
    );
}

#[test]
fn hevc_qsv_main10_selects_p010_and_the_main10_profile() {
    let mut p = Profile::get("hevc-qsv");
    p.pix_fmt = "yuv420p10le".to_string();
    let args = qsv_args(HostPlatform::Linux, &p);
    assert_eq!(value_of(&args, "-pix_fmt").as_deref(), Some("p010le"));
    assert_eq!(value_of(&args, "-profile:v").as_deref(), Some("main10"));
    let sdr = qsv_args(HostPlatform::Linux, &Profile::get("hevc-qsv"));
    assert!(!sdr.iter().any(|a| a == "-profile:v"), "{sdr:?}");
}

#[test]
fn every_nvenc_field_reaches_its_flag_with_its_value() {
    let mut p = Profile::get("hevc-nvenc");
    let h = p.codec.as_hevc_mut().unwrap();
    h.nvenc_quality = 21;
    h.hw_preset = "2".to_string(); // UI 2 -> NVENC p6
    h.hw_lookahead = 16;
    h.hw_b_frames = 3;
    let args = nvenc_args(&p);
    assert_eq!(value_of(&args, "-cq").as_deref(), Some("21"));
    assert_eq!(value_of(&args, "-preset").as_deref(), Some("p6"));
    assert_eq!(value_of(&args, "-rc-lookahead").as_deref(), Some("16"));
    assert_eq!(value_of(&args, "-bf").as_deref(), Some("3"));
    assert_eq!(value_of(&args, "-rc").as_deref(), Some("vbr"));
}

#[test]
fn nvenc_lookahead_zero_omits_the_flag() {
    let mut p = Profile::get("hevc-nvenc");
    p.codec.as_hevc_mut().unwrap().hw_lookahead = 0;
    assert!(!nvenc_args(&p).iter().any(|a| a == "-rc-lookahead"));
}

#[test]
fn nvenc_main10_selects_p010_and_main10_profile() {
    let mut p = Profile::get("hevc-nvenc");
    p.pix_fmt = "yuv420p10le".to_string();
    let args = nvenc_args(&p);
    assert_eq!(value_of(&args, "-pix_fmt").as_deref(), Some("p010le"));
    assert_eq!(value_of(&args, "-profile:v").as_deref(), Some("main10"));
}

#[test]
fn nvenc_clamps_lookahead_and_b_frames_to_what_the_hardware_accepts() {
    let mut p = Profile::get("hevc-nvenc");
    let h = p.codec.as_hevc_mut().unwrap();
    h.hw_lookahead = 40;
    h.hw_b_frames = 9;
    let args = nvenc_args(&p);
    assert_eq!(value_of(&args, "-rc-lookahead").as_deref(), Some("32"));
    assert_eq!(value_of(&args, "-bf").as_deref(), Some("4"));
}

#[cfg(feature = "dev-tools")]
#[test]
fn every_hevc_qsv_registry_param_has_a_flag_the_command_emits() {
    use ffdash::engine::params::params_for_encoder;
    let args = qsv_args(HostPlatform::Linux, &Profile::get("hevc-qsv"));
    let mut seen = 0;
    for param in params_for_encoder("hevc_qsv") {
        let flag = param
            .get_encoder_config("hevc_qsv")
            .and_then(|c| c.flag)
            .expect("registry flag");
        assert!(
            args.iter().any(|a| a == flag),
            "registry param {} ({flag}) is not emitted by the hevc_qsv command",
            param.name
        );
        seen += 1;
    }
    assert!(seen >= 5, "hevc_qsv registry entry looks empty ({seen})");
}

#[cfg(feature = "dev-tools")]
#[test]
fn hevc_qsv_registry_clamps_quality_to_its_native_range() {
    use ffdash::engine::params::validate_and_clamp_profile;
    let mut p = Profile::get("hevc-qsv");
    p.codec.as_hevc_mut().unwrap().qsv_quality = 99;
    let clamps = validate_and_clamp_profile(&mut p, "hevc_qsv");
    assert!(!clamps.is_empty());
    assert_eq!(p.codec.as_hevc().unwrap().qsv_quality, 51);
}

#[cfg(feature = "dev-tools")]
#[test]
fn every_hevc_nvenc_registry_param_has_a_flag_the_command_emits() {
    use ffdash::engine::params::params_for_encoder;
    let mut p = Profile::get("hevc-nvenc");
    p.codec.as_hevc_mut().unwrap().hw_lookahead = 16;
    let args = nvenc_args(&p);
    let mut seen = 0;
    for param in params_for_encoder("hevc_nvenc") {
        let flag = param
            .get_encoder_config("hevc_nvenc")
            .and_then(|c| c.flag)
            .expect("registry flag");
        assert!(
            args.iter().any(|a| a == flag),
            "registry param {} ({flag}) is not emitted by the hevc_nvenc command",
            param.name
        );
        seen += 1;
    }
    assert!(seen >= 5, "hevc_nvenc registry entry looks empty ({seen})");
}

#[cfg(feature = "dev-tools")]
#[test]
fn hevc_nvenc_registry_clamps_quality_to_its_native_range() {
    use ffdash::engine::params::validate_and_clamp_profile;
    let mut p = Profile::get("hevc-nvenc");
    p.codec.as_hevc_mut().unwrap().nvenc_quality = 60;
    let clamps = validate_and_clamp_profile(&mut p, "hevc_nvenc");
    assert!(!clamps.is_empty());
    assert_eq!(p.codec.as_hevc().unwrap().nvenc_quality, 51);
}
