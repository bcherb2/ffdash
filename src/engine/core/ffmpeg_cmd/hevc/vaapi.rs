//! hevc_vaapi (VAAPI, Linux).
//!
//! Frames are decoded on the CPU and uploaded (`format=...,hwupload`). Rate control is
//! always CQP with `-global_quality:v` carrying the QP (1-51, lower is better).
//! hevc_vaapi has no preset or lookahead control, so `hw_preset` and `hw_lookahead`
//! are not used here.

use std::process::Command;

use super::super::container_from_output;
use super::super::{apply_audio_settings, apply_color_metadata, apply_job_additional_args};
use super::apply_hevc_container_tag;
use crate::engine::core::profile::Profile;
use crate::engine::core::types::VideoJob;
use crate::engine::hardware;
use crate::engine::probe;

/// Whether the output is Main10: an explicit 10-bit pixel format, or `auto` on a source
/// that is 10-bit or deeper.
fn is_main10(job: &VideoJob, profile: &Profile) -> bool {
    match profile.pix_fmt.as_str() {
        "yuv420p10le" => true,
        "auto" => probe::probe_input_info(&job.input_path)
            .ok()
            .and_then(|info| info.bit_depth)
            .is_some_and(|depth| depth >= 10),
        _ => false,
    }
}

pub(super) fn build(job: &VideoJob, profile: &Profile) -> Command {
    let mut cmd = Command::new("ffmpeg");
    let hevc = profile.codec.hevc_or_default();
    let container = container_from_output(job, profile);

    // CPU-side filters: fps, scale, HDR->SDR tonemap (all before the upload).
    let mut filters = Vec::new();
    if let Ok(input_info) = probe::probe_input_info(&job.input_path) {
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

        if profile.colorspace == 1 && profile.color_trc == 1 && input_info.is_hdr {
            filters.push("zscale=t=linear:npl=100".to_string());
            filters.push("tonemap=hable:desat=0".to_string());
            filters.push("zscale=t=bt709:m=bt709:r=tv".to_string());
            filters.push("format=yuv420p".to_string());
        }
    }

    let main10 = is_main10(job, profile);
    filters.push(format!("format={}", if main10 { "p010" } else { "nv12" }));
    filters.push("hwupload".to_string());

    // VAAPI device
    if let Some(config) = hardware::detect_vaapi_config() {
        cmd.env("LIBVA_DRIVERS_PATH", &config.driver.path);
        cmd.env("LIBVA_DRIVER_NAME", &config.driver.name);
        cmd.arg("-init_hw_device")
            .arg(format!("vaapi=va:{}", config.render_device));
    } else {
        cmd.arg("-init_hw_device")
            .arg("vaapi=va:/dev/dri/renderD128");
    }
    cmd.arg("-filter_hw_device").arg("va");

    cmd.arg("-i").arg(&job.input_path);
    cmd.arg("-progress").arg("-").arg("-nostats");
    cmd.arg("-vf").arg(filters.join(","));

    cmd.arg("-c:v").arg("hevc_vaapi");
    cmd.arg("-profile:v")
        .arg(if main10 { "main10" } else { "main" });

    // Constant quality is the only mode (ADR 0001): CQP with the QP in global_quality.
    cmd.arg("-rc_mode:v").arg("CQP");
    cmd.arg("-global_quality:v")
        .arg(hevc.vaapi_quality.clamp(1, 51).to_string());
    cmd.arg("-bf:v").arg(hevc.hw_b_frames.to_string());

    cmd.arg("-g:v").arg(&profile.gop_length);

    apply_color_metadata(&mut cmd, profile);
    apply_hevc_container_tag(&mut cmd, job, profile);

    apply_audio_settings(&mut cmd, profile, &container);
    apply_job_additional_args(&mut cmd, job, profile);

    if job.overwrite {
        cmd.arg("-y");
    }
    cmd.arg(&job.output_path);
    cmd
}
