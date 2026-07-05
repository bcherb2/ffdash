use ffdash::engine::core::{
    Profile, VideoJob, build_av1_nvenc_cmd, build_av1_qsv_cmd, build_av1_software_cmd,
    build_av1_vaapi_cmd, build_ffmpeg_cmds_with_profile, build_software_cmd,
};
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
    let cmd = build_software_cmd(&job, &profile);
    assert_snapshot!("vp9_good", to_string(&cmd));

    let profile_best = Profile::get("vp9-best");
    let mut job_best = mk_job("webm");
    job_best.profile = "vp9-best".to_string();
    let cmds = build_ffmpeg_cmds_with_profile(&job_best, None, Some(&profile_best));
    let joined = cmds.iter().map(to_string).collect::<Vec<_>>().join("\n");
    assert_snapshot!("vp9_best_two_pass", joined);
}

#[test]
fn snapshot_av1_svt() {
    let profile = Profile::get("av1-svt");
    let job = mk_job("mkv");
    let cmd = build_av1_software_cmd(&job, &profile);
    assert_snapshot!("av1_svt", to_string(&cmd));
}

#[test]
fn snapshot_av1_qsv() {
    let profile = Profile::get("av1-qsv");
    let job = mk_job("mkv");
    let cmd = build_av1_qsv_cmd(&job, &profile);
    assert_snapshot!("av1_qsv", to_string(&cmd));
}

#[test]
fn snapshot_av1_nvenc() {
    let profile = Profile::get("av1-nvenc");
    let job = mk_job("mkv");
    let cmd = build_av1_nvenc_cmd(&job, &profile);
    assert_snapshot!("av1_nvenc", to_string(&cmd));
}

#[test]
fn snapshot_av1_nvenc_sdr() {
    let mut profile = Profile::get("av1-nvenc");
    profile.colorspace = 1; // bt709
    profile.color_primaries = 1; // bt709
    profile.color_trc = 1; // bt709
    profile.color_range = 0; // tv/limited

    let job = mk_job("mkv");
    let cmd = build_av1_nvenc_cmd(&job, &profile);
    assert_snapshot!("av1_nvenc_sdr", to_string(&cmd));
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
    let cmd = build_av1_nvenc_cmd(&job, &profile);
    assert_snapshot!("av1_nvenc_hdr10", to_string(&cmd));
}

#[test]
fn snapshot_av1_vaapi() {
    let profile = Profile::get("av1-vaapi");
    let job = mk_job("mkv");
    let cmd = build_av1_vaapi_cmd(&job, &profile);
    assert_snapshot!("av1_vaapi", to_string(&cmd));
}

#[test]
fn snapshot_av1_qsv_sdr() {
    let mut profile = Profile::get("av1-qsv");
    profile.colorspace = 1; // bt709
    profile.color_primaries = 1; // bt709
    profile.color_trc = 1; // bt709
    profile.color_range = 0; // tv/limited

    let job = mk_job("mkv");
    let cmd = build_av1_qsv_cmd(&job, &profile);
    assert_snapshot!("av1_qsv_sdr", to_string(&cmd));
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
    let cmd = build_av1_qsv_cmd(&job, &profile);
    assert_snapshot!("av1_qsv_hdr10", to_string(&cmd));
}
