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
    let cmds = build_encode_cmds(job, profile, &EncodePlan::new(encoder));
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
