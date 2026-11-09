//! `hevc_qsv` (Intel Quick Sync, Linux and Windows).
//!
//! Linux reuses the VAAPI-derived QSV device and decodes on the GPU; elsewhere frames
//! are decoded on the CPU and uploaded to QSV surfaces. Quality is constant-quality
//! CQP through `-q:v` (`HevcConfig::qsv_quality`, 1-51, lower is better).

use std::process::Command;

use super::super::{
    apply_audio_settings, apply_job_additional_args, build_qsv_color_options,
    container_from_output, init_qsv_from_vaapi, qsv_preset_name,
};
use super::apply_hevc_container_tag;
use crate::engine::core::profile::Profile;
use crate::engine::core::types::VideoJob;
use crate::engine::probe;

pub(super) fn build(job: &VideoJob, profile: &Profile, cpu_decode: bool) -> Command {
    let mut cmd = Command::new("ffmpeg");
    let hevc = profile.codec.hevc_or_default();

    // Heavier probing helps with large/multi-stream inputs
    cmd.arg("-analyzeduration").arg("200M");
    cmd.arg("-probesize").arg("200M");

    init_qsv_from_vaapi(&mut cmd, cpu_decode);
    if !cpu_decode {
        cmd.arg("-hwaccel").arg("qsv");
        cmd.arg("-hwaccel_output_format").arg("qsv");
        cmd.arg("-filter_hw_device").arg("qs");
    }

    cmd.arg("-i").arg(&job.input_path);
    cmd.arg("-progress").arg("-").arg("-nostats");

    let input_info = probe::probe_input_info(&job.input_path).ok();

    let mut filters = Vec::new();
    if let Some(input_info) = &input_info {
        if profile.fps > 0 && input_info.fps > profile.fps as f64 {
            filters.push(format!("fps=fps={}", profile.fps));
        }

        let needs_scale = (profile.scale_width > 0
            && input_info.width > profile.scale_width as u32)
            || (profile.scale_height > 0 && input_info.height > profile.scale_height as u32);
        if needs_scale {
            let max_w = if profile.scale_width > 0 {
                profile.scale_width
            } else {
                i32::MAX
            };
            let max_h = if profile.scale_height > 0 {
                profile.scale_height
            } else {
                i32::MAX
            };
            filters.push(format!(
                "scale='min({},iw)':'min({},ih)':force_original_aspect_ratio=decrease",
                max_w, max_h
            ));
        }
    }

    // QSV surface format. "auto" keeps the source depth: on Linux the GPU decoder's
    // frames pass straight through; on CPU decode the depth is read from the source.
    let source_is_10bit = input_info
        .as_ref()
        .and_then(|i| i.bit_depth)
        .is_some_and(|d| d > 8);
    let (qsv_format, pix_fmt_arg) = match profile.pix_fmt.as_str() {
        "yuv420p10le" => (Some("p010"), Some("p010le")),
        "yuv420p" => (Some("nv12"), Some("nv12")),
        "auto" if cpu_decode && source_is_10bit => (Some("p010"), Some("p010le")),
        "auto" if cpu_decode => (Some("nv12"), Some("nv12")),
        "auto" => (None, None),
        _ => (Some("nv12"), Some("nv12")),
    };

    // CPU-decoded frames must be uploaded to QSV surfaces before vpp_qsv.
    if cpu_decode {
        filters.push(format!("format={}", qsv_format.unwrap_or("nv12")));
        filters.push("hwupload=extra_hw_frames=64".to_string());
    }

    // QSV encoders ignore -colorspace/-color_*: colour metadata goes through vpp_qsv.
    let mut vpp_opts = Vec::new();
    if let Some(format) = qsv_format {
        vpp_opts.push(format!("format={}", format));
    }
    vpp_opts.extend(build_qsv_color_options(profile));
    if !vpp_opts.is_empty() {
        filters.push(format!("vpp_qsv={}", vpp_opts.join(":")));
    }
    if !filters.is_empty() {
        cmd.arg("-vf").arg(filters.join(","));
    }

    cmd.arg("-c:v").arg("hevc_qsv");

    // Constant quality (CQP): `-q:v` selects it.
    cmd.arg("-q:v").arg(hevc.qsv_quality.to_string());

    let preset = hevc
        .hw_preset
        .parse::<u32>()
        .ok()
        .map(qsv_preset_name)
        .unwrap_or(hevc.hw_preset.as_str());
    cmd.arg("-preset").arg(preset);

    cmd.arg("-bf").arg(hevc.hw_b_frames.to_string());
    if hevc.hw_lookahead > 0 {
        cmd.arg("-look_ahead").arg("1");
        cmd.arg("-look_ahead_depth")
            .arg(hevc.hw_lookahead.min(100).to_string());
    }

    if matches!(pix_fmt_arg, Some("p010le")) {
        cmd.arg("-profile:v").arg("main10");
    }

    cmd.arg("-g:v").arg(&profile.gop_length);

    if let Some(pix_fmt) = pix_fmt_arg {
        cmd.arg("-pix_fmt").arg(pix_fmt);
    }

    apply_hevc_container_tag(&mut cmd, job, profile);

    let container = container_from_output(job, profile);
    apply_audio_settings(&mut cmd, profile, &container);
    apply_job_additional_args(&mut cmd, job, profile);

    if job.overwrite {
        cmd.arg("-y");
    }
    cmd.arg(&job.output_path);
    cmd
}
