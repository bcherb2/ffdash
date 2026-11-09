use ffdash::engine::core::HostPlatform;
use ffdash::engine::core::{EncodePlan, Profile, VideoJob, build_encode_cmds};
use ffdash::engine::hardware::VideoEncoder;
use insta::assert_snapshot;
use std::path::PathBuf;
use uuid::Uuid;

fn to_string(cmd: &std::process::Command) -> String {
    let mut parts = Vec::new();
    parts.push(cmd.get_program().to_string_lossy().to_string());
    parts.extend(
        cmd.get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect::<Vec<_>>(),
    );
    parts.join(" ")
}

fn build(job: &VideoJob, profile: &Profile, encoder: VideoEncoder) -> String {
    let plan = EncodePlan::new(encoder).with_host(HostPlatform::Linux);
    let cmds = build_encode_cmds(job, profile, &plan);
    cmds.iter().map(to_string).collect::<Vec<_>>().join("\n")
}

fn mk_job(output_ext: &str) -> VideoJob {
    let mut job = VideoJob::new(
        PathBuf::from("/tmp/input.mp4"),
        PathBuf::from(format!("/tmp/output.{}", output_ext)),
        "snapshot".to_string(),
    );
    job.id = Uuid::nil(); // stable two-pass log path
    job.overwrite = true;
    job
}

#[test]
fn snapshot_vp9_commands() {
    let profile = Profile::get("vp9-good");
    let job = mk_job("webm");
    let out = build(&job, &profile, VideoEncoder::LibvpxVp9);
    assert_snapshot!("vp9_good", out);

    // Production profiles are synced from their codec config before command building.
    let mut profile_best = Profile::get("vp9-best");
    profile_best.sync_legacy_fields();
    let mut job_best = mk_job("webm");
    job_best.profile = "vp9-best".to_string();
    let joined = build(&job_best, &profile_best, VideoEncoder::LibvpxVp9);
    assert_snapshot!("vp9_best_two_pass", joined);
}

#[test]
fn snapshot_av1_svt() {
    let profile = Profile::get("av1-svt");
    let job = mk_job("mkv");
    let out = build(&job, &profile, VideoEncoder::LibsvtAv1);
    assert_snapshot!("av1_svt", out);
}

#[test]
fn snapshot_av1_qsv() {
    let profile = Profile::get("av1-qsv");
    let job = mk_job("mkv");
    let out = build(&job, &profile, VideoEncoder::Av1Qsv);
    assert_snapshot!("av1_qsv", out);
}

#[test]
fn snapshot_av1_nvenc() {
    let profile = Profile::get("av1-nvenc");
    let job = mk_job("mkv");
    let out = build(&job, &profile, VideoEncoder::Av1Nvenc);
    assert_snapshot!("av1_nvenc", out);
}

#[test]
fn snapshot_av1_nvenc_sdr() {
    let mut profile = Profile::get("av1-nvenc");
    profile.colorspace = 1; // bt709
    profile.color_primaries = 1; // bt709
    profile.color_trc = 1; // bt709
    profile.color_range = 0; // tv/limited

    let job = mk_job("mkv");
    let out = build(&job, &profile, VideoEncoder::Av1Nvenc);
    assert_snapshot!("av1_nvenc_sdr", out);
}

#[test]
fn snapshot_av1_nvenc_hdr10() {
    let mut profile = Profile::get("av1-nvenc");
    profile.colorspace = 9; // bt2020nc
    profile.color_primaries = 9; // bt2020
    profile.color_trc = 16; // smpte2084 (PQ)
    profile.color_range = 0; // tv/limited
    profile.pix_fmt = "yuv420p10le".to_string(); // 10-bit for HDR

    let job = mk_job("mkv");
    let out = build(&job, &profile, VideoEncoder::Av1Nvenc);
    assert_snapshot!("av1_nvenc_hdr10", out);
}

#[test]
fn snapshot_av1_vaapi() {
    let profile = Profile::get("av1-vaapi");
    let job = mk_job("mkv");
    let out = build(&job, &profile, VideoEncoder::Av1Vaapi);
    assert_snapshot!("av1_vaapi", out);
}

#[test]
fn snapshot_av1_qsv_sdr() {
    let mut profile = Profile::get("av1-qsv");
    profile.colorspace = 1; // bt709
    profile.color_primaries = 1; // bt709
    profile.color_trc = 1; // bt709
    profile.color_range = 0; // tv/limited

    let job = mk_job("mkv");
    let out = build(&job, &profile, VideoEncoder::Av1Qsv);
    assert_snapshot!("av1_qsv_sdr", out);
}

#[test]
fn snapshot_av1_qsv_hdr10() {
    let mut profile = Profile::get("av1-qsv");
    profile.colorspace = 9; // bt2020nc
    profile.color_primaries = 9; // bt2020
    profile.color_trc = 16; // smpte2084 (PQ)
    profile.color_range = 0; // tv/limited
    profile.pix_fmt = "yuv420p10le".to_string(); // 10-bit for HDR

    let job = mk_job("mkv");
    let out = build(&job, &profile, VideoEncoder::Av1Qsv);
    assert_snapshot!("av1_qsv_hdr10", out);
}

#[test]
fn snapshot_hevc_x265() {
    let profile = Profile::get("hevc-x265");
    let job = mk_job("mkv");
    let out = build(&job, &profile, VideoEncoder::Libx265);
    assert_snapshot!("hevc_x265", out);
}

#[test]
fn snapshot_hevc_x265_hdr10() {
    let mut profile = Profile::get("hevc-x265");
    profile.colorspace = 9; // bt2020nc
    profile.color_primaries = 9; // bt2020
    profile.color_trc = 16; // smpte2084 (PQ)
    profile.color_range = 0; // tv/limited
    profile.pix_fmt = "yuv420p10le".to_string(); // Main10

    let job = mk_job("mkv");
    let out = build(&job, &profile, VideoEncoder::Libx265);
    assert_snapshot!("hevc_x265_hdr10", out);
}

#[test]
fn snapshot_hevc_x265_mp4_is_tagged_hvc1() {
    let profile = Profile::get("hevc-x265");
    let job = mk_job("mp4");
    let out = build(&job, &profile, VideoEncoder::Libx265);
    assert!(out.contains("-tag:v hvc1"), "{out}");
    assert_snapshot!("hevc_x265_mp4", out);
}

#[test]
fn snapshot_mp4_output_never_maps_attachments_or_copies_subtitles() {
    // Profiles written for MKV map `0:s?`/`0:t?` and copy them; MP4 cannot hold those.
    // The input does not exist, so no text subtitles are probed and subtitles are dropped.
    let av1 = build(
        &mk_job("mp4"),
        &Profile::get("av1-svt"),
        VideoEncoder::LibsvtAv1,
    );
    assert_snapshot!("av1_svt_mp4", av1);

    let vp9 = build(
        &mk_job("mp4"),
        &Profile::get("vp9-good"),
        VideoEncoder::LibvpxVp9,
    );
    assert_snapshot!("vp9_good_mp4", vp9);

    for out in [&av1, &vp9] {
        assert!(
            !out.contains("-map 0:t") && !out.contains("-map 0:s"),
            "{out}"
        );
        assert!(!out.contains("-c:s") && !out.contains("-c:t"), "{out}");
    }
}

#[test]
fn snapshot_hevc_nvenc() {
    let profile = Profile::get("hevc-nvenc");
    let job = mk_job("mkv");
    let out = build(&job, &profile, VideoEncoder::HevcNvenc);
    assert_snapshot!("hevc_nvenc", out);
}

#[test]
fn snapshot_hevc_nvenc_hdr10() {
    let mut profile = Profile::get("hevc-nvenc");
    profile.colorspace = 9;
    profile.color_primaries = 9;
    profile.color_trc = 16;
    profile.color_range = 0;
    profile.pix_fmt = "yuv420p10le".to_string(); // Main10
    let out = build(&mk_job("mkv"), &profile, VideoEncoder::HevcNvenc);
    assert_snapshot!("hevc_nvenc_hdr10", out);
}

#[test]
fn snapshot_hevc_nvenc_mp4_is_tagged_hvc1() {
    let profile = Profile::get("hevc-nvenc");
    let out = build(&mk_job("mp4"), &profile, VideoEncoder::HevcNvenc);
    assert!(out.contains("-tag:v hvc1"), "{out}");
    assert_snapshot!("hevc_nvenc_mp4", out);
}

#[test]
fn snapshot_windows_hevc_nvenc() {
    let out = build_on(
        HostPlatform::Windows,
        &mk_job("mkv"),
        &Profile::get("hevc-nvenc"),
        VideoEncoder::HevcNvenc,
    );
    assert!(!out.contains("-hwaccel"), "{out}");
    assert_snapshot!("windows_hevc_nvenc", out);
}

#[test]
fn snapshot_hevc_vaapi_sdr() {
    let profile = Profile::get("hevc-vaapi");
    let job = mk_job("mkv");
    let out = build(&job, &profile, VideoEncoder::HevcVaapi);
    assert_snapshot!("hevc_vaapi_sdr", out);
}

#[test]
fn snapshot_hevc_vaapi_main10() {
    let mut profile = Profile::get("hevc-vaapi");
    profile.pix_fmt = "yuv420p10le".to_string();
    let job = mk_job("mkv");
    let out = build(&job, &profile, VideoEncoder::HevcVaapi);
    assert!(out.contains("format=p010,hwupload"), "{out}");
    assert!(out.contains("-profile:v main10"), "{out}");
    assert_snapshot!("hevc_vaapi_main10", out);
}

#[test]
fn snapshot_hevc_vaapi_mp4_is_tagged_hvc1() {
    let profile = Profile::get("hevc-vaapi");
    let out = build(&mk_job("mp4"), &profile, VideoEncoder::HevcVaapi);
    assert!(out.contains("-tag:v hvc1"), "{out}");
}

#[test]
fn hevc_in_mkv_is_not_tagged() {
    let profile = Profile::get("hevc-x265");
    let job = mk_job("mkv");
    assert!(!build(&job, &profile, VideoEncoder::Libx265).contains("-tag:v"));
}

fn hdr10(profile: &mut Profile) {
    profile.colorspace = 9; // bt2020nc
    profile.color_primaries = 9; // bt2020
    profile.color_trc = 16; // smpte2084 (PQ)
    profile.color_range = 0; // tv/limited
    profile.pix_fmt = "yuv420p10le".to_string(); // Main10
}

#[test]
fn snapshot_hevc_qsv_sdr() {
    let mut profile = Profile::get("hevc-qsv");
    profile.colorspace = 1;
    profile.color_primaries = 1;
    profile.color_trc = 1;
    profile.color_range = 0;
    let out = build(&mk_job("mkv"), &profile, VideoEncoder::HevcQsv);
    assert_snapshot!("hevc_qsv_sdr", out);
}

#[test]
fn snapshot_hevc_qsv_hdr10() {
    let mut profile = Profile::get("hevc-qsv");
    hdr10(&mut profile);
    let out = build(&mk_job("mkv"), &profile, VideoEncoder::HevcQsv);
    assert_snapshot!("hevc_qsv_hdr10", out);
}

#[test]
fn snapshot_hevc_qsv_mp4_is_tagged_hvc1() {
    let profile = Profile::get("hevc-qsv");
    let out = build(&mk_job("mp4"), &profile, VideoEncoder::HevcQsv);
    assert!(out.contains("-tag:v hvc1"), "{out}");
}

#[test]
fn snapshot_windows_hevc_qsv_sdr() {
    let out = build_on(
        HostPlatform::Windows,
        &mk_job("mkv"),
        &Profile::get("hevc-qsv"),
        VideoEncoder::HevcQsv,
    );
    assert!(
        !out.contains("-hwaccel") && !out.contains("/dev/dri"),
        "{out}"
    );
    assert_snapshot!("windows_hevc_qsv_sdr", out);
}

#[test]
fn snapshot_windows_hevc_qsv_hdr10() {
    let mut profile = Profile::get("hevc-qsv");
    hdr10(&mut profile);
    let out = build_on(
        HostPlatform::Windows,
        &mk_job("mkv"),
        &profile,
        VideoEncoder::HevcQsv,
    );
    assert_snapshot!("windows_hevc_qsv_hdr10", out);
}

#[test]
fn software_vp9_fallback_for_hardware_profile_stays_single_pass() {
    // A profile that requested hardware but was planned onto the software Encoder
    // (hardware unavailable) builds one single-pass command, even with two-pass and a
    // target bitrate set.
    let mut profile = Profile::get("vp9-best");
    profile.use_hardware_encoding = true;
    profile.two_pass = true;
    profile.video_target_bitrate = 2000;
    let job = mk_job("webm");
    let plan = EncodePlan::new(VideoEncoder::LibvpxVp9);
    assert_eq!(build_encode_cmds(&job, &profile, &plan).len(), 1);

    // Control: the same profile without the hardware request is two-pass.
    profile.use_hardware_encoding = false;
    assert_eq!(build_encode_cmds(&job, &profile, &plan).len(), 2);
}

// --- Non-Linux hosts: Hardware Encoders are fed from CPU decoding -------------------
// The host platform is an explicit input, so these run on any machine.

fn build_on(
    host: HostPlatform,
    job: &VideoJob,
    profile: &Profile,
    encoder: VideoEncoder,
) -> String {
    let plan = EncodePlan::new(encoder).with_host(host);
    build_encode_cmds(job, profile, &plan)
        .iter()
        .map(to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

const HW_CASES: [(&str, VideoEncoder); 5] = [
    ("vp9-good", VideoEncoder::Vp9Qsv),
    ("vp9-good", VideoEncoder::Vp9Vaapi),
    ("av1-qsv", VideoEncoder::Av1Qsv),
    ("av1-nvenc", VideoEncoder::Av1Nvenc),
    ("av1-vaapi", VideoEncoder::Av1Vaapi),
];

#[test]
fn non_linux_hardware_commands_decode_on_cpu() {
    for host in [HostPlatform::Windows, HostPlatform::MacOs] {
        for (name, encoder) in HW_CASES {
            let profile = Profile::get(name);
            let out = build_on(host, &mk_job("mkv"), &profile, encoder);
            assert!(!out.contains("-hwaccel"), "{host:?} {encoder:?}: {out}");
            // VAAPI is a Linux-only Backend; its render node is not applicable elsewhere.
            if !matches!(encoder, VideoEncoder::Vp9Vaapi | VideoEncoder::Av1Vaapi) {
                assert!(!out.contains("/dev/dri"), "{host:?} {encoder:?}: {out}");
            }
        }
    }
}

#[test]
fn linux_host_keeps_hardware_decode_paths() {
    for (name, encoder) in HW_CASES {
        let profile = Profile::get(name);
        let job = mk_job("mkv");
        let explicit = build_on(HostPlatform::Linux, &job, &profile, encoder);
        assert!(
            explicit.contains("-init_hw_device") || explicit.contains("-hwaccel"),
            "{encoder:?}: {explicit}"
        );
        {
            assert_eq!(explicit, build(&job, &profile, encoder), "{encoder:?}");
        }
    }
}

#[test]
fn snapshot_windows_av1_nvenc() {
    let out = build_on(
        HostPlatform::Windows,
        &mk_job("mkv"),
        &Profile::get("av1-nvenc"),
        VideoEncoder::Av1Nvenc,
    );
    assert_snapshot!("windows_av1_nvenc", out);
}

#[test]
fn snapshot_windows_av1_qsv() {
    let out = build_on(
        HostPlatform::Windows,
        &mk_job("mkv"),
        &Profile::get("av1-qsv"),
        VideoEncoder::Av1Qsv,
    );
    assert_snapshot!("windows_av1_qsv", out);
}

#[test]
fn snapshot_windows_vp9_qsv() {
    let out = build_on(
        HostPlatform::Windows,
        &mk_job("webm"),
        &Profile::get("vp9-good"),
        VideoEncoder::Vp9Qsv,
    );
    assert_snapshot!("windows_vp9_qsv", out);
}

#[test]
fn snapshot_macos_av1_nvenc_hdr10() {
    let mut profile = Profile::get("av1-nvenc");
    profile.colorspace = 9;
    profile.color_primaries = 9;
    profile.color_trc = 16;
    profile.color_range = 0;
    profile.pix_fmt = "yuv420p10le".to_string();
    let out = build_on(
        HostPlatform::MacOs,
        &mk_job("mkv"),
        &profile,
        VideoEncoder::Av1Nvenc,
    );
    assert_snapshot!("macos_av1_nvenc_hdr10", out);
}

#[test]
fn snapshot_windows_vp9_vaapi() {
    let out = build_on(
        HostPlatform::Windows,
        &mk_job("webm"),
        &Profile::get("vp9-good"),
        VideoEncoder::Vp9Vaapi,
    );
    assert_snapshot!("windows_vp9_vaapi", out);
}
