//! Command skeleton shared by the HEVC Encoders that are always fed from CPU decode
//! (AMF and VideoToolbox): no `-hwaccel`, no device init, system-memory frames in.
//! Each Encoder supplies only its own codec flags.

use std::process::Command;

use super::super::container_from_output;
use super::super::{apply_audio_settings, apply_color_metadata, apply_job_additional_args};
use super::apply_hevc_container_tag;
use crate::engine::core::profile::Profile;
use crate::engine::core::types::VideoJob;
use crate::engine::probe;

/// Hardware surface format for a requested pixel format: 10-bit input becomes `p010le`.
fn hardware_pix_fmt(pix_fmt: &str) -> Option<&'static str> {
    if pix_fmt == "auto" {
        None
    } else if pix_fmt.contains("10") {
        Some("p010le")
    } else {
        Some("nv12")
    }
}

/// Build the whole command; `encoder_flags` appends the Encoder's own flags right after
/// `-c:v <encoder>`.
pub(super) fn build(
    job: &VideoJob,
    profile: &Profile,
    encoder: &str,
    encoder_flags: impl FnOnce(&mut Command),
) -> Command {
    let mut cmd = Command::new("ffmpeg");

    cmd.arg("-i").arg(&job.input_path);
    cmd.arg("-progress").arg("-").arg("-nostats");

    cmd.arg("-c:v").arg(encoder);
    encoder_flags(&mut cmd);

    if let Some(fmt) = hardware_pix_fmt(&profile.pix_fmt) {
        cmd.arg("-pix_fmt").arg(fmt);
        if fmt == "p010le" {
            cmd.arg("-profile:v").arg("main10");
        }
    }

    cmd.arg("-g:v").arg(&profile.gop_length);

    apply_color_metadata(&mut cmd, profile);
    apply_hevc_container_tag(&mut cmd, job, profile);

    // Video filters (fps, scale)
    let mut filters = Vec::new();
    if let Ok(input_info) = probe::probe_input_info(&job.input_path) {
        if profile.fps > 0 && input_info.fps > profile.fps as f64 {
            filters.push(format!("fps={}", profile.fps));
        }
        let needs_scale = (profile.scale_width > 0
            && input_info.width > profile.scale_width as u32)
            || (profile.scale_height > 0 && input_info.height > profile.scale_height as u32);
        if needs_scale {
            let w = if profile.scale_width > 0 {
                format!("min(iw\\,{})", profile.scale_width)
            } else {
                "-2".to_string()
            };
            let h = if profile.scale_height > 0 {
                format!("min(ih\\,{})", profile.scale_height)
            } else {
                "-2".to_string()
            };
            filters.push(format!("scale={}:{}", w, h));
        }
    }
    if !filters.is_empty() {
        cmd.arg("-vf").arg(filters.join(","));
    }

    let container = container_from_output(job, profile);
    apply_audio_settings(&mut cmd, profile, &container);
    apply_job_additional_args(&mut cmd, job, profile);

    if job.overwrite {
        cmd.arg("-y");
    }
    cmd.arg(&job.output_path);
    cmd
}
