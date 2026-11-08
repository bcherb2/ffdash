//! HEVC on AMF (`hevc_amf`) and VideoToolbox (`hevc_videotoolbox`): command snapshots,
//! Quality Override, and the settings each Encoder reads. Both are fed from CPU decode
//! on every host.

use ffdash::engine::core::{EncodePlan, HostPlatform, Profile, VideoJob, build_encode_cmds};
use ffdash::engine::hardware::VideoEncoder;
use insta::assert_snapshot;
use std::path::PathBuf;
use uuid::Uuid;

fn mk_job(output_ext: &str) -> VideoJob {
    let mut job = VideoJob::new(
        PathBuf::from("/tmp/input.mp4"),
        PathBuf::from(format!("/tmp/output.{output_ext}")),
        "p".to_string(),
    );
    job.id = Uuid::nil();
    job
}

fn args_of(job: &VideoJob, profile: &Profile, plan: &EncodePlan) -> Vec<String> {
    build_encode_cmds(job, profile, plan)
        .pop()
        .unwrap()
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect()
}

fn build_on(
    host: HostPlatform,
    job: &VideoJob,
    profile: &Profile,
    encoder: VideoEncoder,
) -> String {
    let plan = EncodePlan::new(encoder).with_host(host);
    args_of(job, profile, &plan).join(" ")
}

fn with_override(profile: &Profile, encoder: VideoEncoder, quality: u32) -> Vec<String> {
    let plan = EncodePlan::new(encoder).with_quality_override(quality);
    args_of(&mk_job("mkv"), profile, &plan)
}

fn value_of(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1).cloned())
}

fn hdr10(mut profile: Profile) -> Profile {
    profile.colorspace = 9;
    profile.color_primaries = 9;
    profile.color_trc = 16;
    profile.color_range = 0;
    profile.pix_fmt = "yuv420p10le".to_string();
    profile
}

#[test]
fn snapshot_hevc_amf() {
    let out = build_on(
        HostPlatform::Windows,
        &mk_job("mkv"),
        &Profile::get("hevc-amf"),
        VideoEncoder::HevcAmf,
    );
    assert_snapshot!("hevc_amf", out);
}

#[test]
fn snapshot_hevc_amf_hdr10_mp4() {
    let out = build_on(
        HostPlatform::Windows,
        &mk_job("mp4"),
        &hdr10(Profile::get("hevc-amf")),
        VideoEncoder::HevcAmf,
    );
    assert!(out.contains("-tag:v hvc1"), "{out}");
    assert_snapshot!("hevc_amf_hdr10_mp4", out);
}

#[test]
fn snapshot_hevc_videotoolbox() {
    let out = build_on(
        HostPlatform::MacOs,
        &mk_job("mp4"),
        &Profile::get("hevc-videotoolbox"),
        VideoEncoder::HevcVideotoolbox,
    );
    assert!(out.contains("-tag:v hvc1"), "{out}");
    assert_snapshot!("hevc_videotoolbox", out);
}

#[test]
fn snapshot_hevc_videotoolbox_hdr10() {
    let out = build_on(
        HostPlatform::MacOs,
        &mk_job("mkv"),
        &hdr10(Profile::get("hevc-videotoolbox")),
        VideoEncoder::HevcVideotoolbox,
    );
    assert_snapshot!("hevc_videotoolbox_hdr10", out);
}

#[test]
fn neither_encoder_uses_hardware_decode_on_any_host() {
    for host in [
        HostPlatform::Linux,
        HostPlatform::Windows,
        HostPlatform::MacOs,
    ] {
        for (name, encoder) in [
            ("hevc-amf", VideoEncoder::HevcAmf),
            ("hevc-videotoolbox", VideoEncoder::HevcVideotoolbox),
        ] {
            let out = build_on(host, &mk_job("mkv"), &Profile::get(name), encoder);
            assert!(!out.contains("-hwaccel"), "{host:?} {encoder:?}: {out}");
            assert!(
                !out.contains("-init_hw_device"),
                "{host:?} {encoder:?}: {out}"
            );
        }
    }
}

#[test]
fn amf_override_reaches_both_qp_flags_and_not_the_configured_value() {
    let mut profile = Profile::get("hevc-amf");
    profile.codec.as_hevc_mut().unwrap().amf_qp = 19;
    let args = with_override(&profile, VideoEncoder::HevcAmf, 33);
    assert_eq!(value_of(&args, "-qp_i").as_deref(), Some("33"));
    assert_eq!(value_of(&args, "-qp_p").as_deref(), Some("33"));
    assert!(!args.iter().any(|a| a == "19"), "{args:?}");
    assert_eq!(profile.active_quality(VideoEncoder::HevcAmf), 19);
}

#[test]
fn videotoolbox_override_reaches_q_v_and_not_the_configured_value() {
    let mut profile = Profile::get("hevc-videotoolbox");
    profile.codec.as_hevc_mut().unwrap().videotoolbox_q = 55;
    let args = with_override(&profile, VideoEncoder::HevcVideotoolbox, 83);
    assert_eq!(value_of(&args, "-q:v").as_deref(), Some("83"));
    assert!(!args.iter().any(|a| a == "55"), "{args:?}");
    assert_eq!(profile.active_quality(VideoEncoder::HevcVideotoolbox), 55);
}

#[test]
fn amf_reads_its_qp_and_preset_but_not_lookahead_or_b_frames() {
    let mut profile = Profile::get("hevc-amf");
    let h = profile.codec.as_hevc_mut().unwrap();
    h.amf_qp = 22;
    h.hw_preset = "1".to_string(); // best quality
    h.hw_lookahead = 17;
    h.hw_b_frames = 3;
    let args = args_of(
        &mk_job("mkv"),
        &profile,
        &EncodePlan::new(VideoEncoder::HevcAmf),
    );
    assert_eq!(value_of(&args, "-qp_i").as_deref(), Some("22"));
    assert_eq!(value_of(&args, "-quality").as_deref(), Some("quality"));
    assert!(!args.iter().any(|a| a == "17" || a == "3"), "{args:?}");

    profile.codec.as_hevc_mut().unwrap().hw_preset = "7".to_string();
    let fast = args_of(
        &mk_job("mkv"),
        &profile,
        &EncodePlan::new(VideoEncoder::HevcAmf),
    );
    assert_eq!(value_of(&fast, "-quality").as_deref(), Some("speed"));
}

#[test]
fn videotoolbox_reads_only_its_quality() {
    let mut profile = Profile::get("hevc-videotoolbox");
    profile.codec.as_hevc_mut().unwrap().videotoolbox_q = 71;
    let args = args_of(
        &mk_job("mkv"),
        &profile,
        &EncodePlan::new(VideoEncoder::HevcVideotoolbox),
    );
    assert_eq!(value_of(&args, "-q:v").as_deref(), Some("71"));
    for flag in ["-preset", "-rc", "-crf", "-qp_i", "-b_ratio", "-bf"] {
        assert!(!args.iter().any(|a| a == flag), "{flag}: {args:?}");
    }
}

#[test]
fn ten_bit_output_uses_the_hardware_pixel_format_and_main10() {
    for (name, encoder) in [
        ("hevc-amf", VideoEncoder::HevcAmf),
        ("hevc-videotoolbox", VideoEncoder::HevcVideotoolbox),
    ] {
        let args = args_of(
            &mk_job("mkv"),
            &hdr10(Profile::get(name)),
            &EncodePlan::new(encoder),
        );
        assert_eq!(
            value_of(&args, "-pix_fmt").as_deref(),
            Some("p010le"),
            "{name}"
        );
        assert_eq!(
            value_of(&args, "-profile:v").as_deref(),
            Some("main10"),
            "{name}"
        );
    }
}

#[test]
fn selection_uses_amf_and_videotoolbox_only_where_they_passed_the_probe() {
    use VideoEncoder::*;
    use ffdash::engine::selection::{CodecKind, EncoderAvailability, select_encoder};
    let avail = |encoders: &[VideoEncoder]| {
        encoders
            .iter()
            .fold(EncoderAvailability::default(), |a, e| a.with(*e, true))
    };
    let pick = |a: &EncoderAvailability| select_encoder(CodecKind::Hevc, true, None, 8, a);
    assert_eq!(pick(&avail(&[Libx265, HevcAmf])), HevcAmf);
    assert_eq!(pick(&avail(&[Libx265, HevcVideotoolbox])), HevcVideotoolbox);
    // AMF outranks VideoToolbox
    assert_eq!(pick(&avail(&[HevcVideotoolbox, HevcAmf])), HevcAmf);
    // Intel Macs fail the constant-quality probe: no entry, so libx265
    assert_eq!(pick(&avail(&[Libx265])), Libx265);
    // Without hardware requested, always the Software Fallback
    let both = avail(&[HevcAmf, HevcVideotoolbox]);
    assert_eq!(
        select_encoder(CodecKind::Hevc, false, None, 8, &both),
        Libx265
    );
}
