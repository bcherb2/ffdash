//! hevc_nvenc (NVIDIA NVENC, Linux and Windows).

use std::process::Command;

use super::super::container_from_output;
use super::super::{apply_audio_settings, apply_color_metadata, apply_job_additional_args};
use super::apply_hevc_container_tag;
use crate::engine::core::is_ten_bit_pix_fmt;
use crate::engine::core::profile::Profile;
use crate::engine::core::types::VideoJob;
use crate::engine::probe;

/// Map the shared hardware preset onto NVENC's `p1` (fastest) .. `p7` (best quality).
/// The UI scale is 1 = best quality .. 7 = fastest, so it is inverted.
fn nvenc_preset(hw_preset: &str) -> String {
    if hw_preset.starts_with('p') {
        return hw_preset.to_string();
    }
    let val: u32 = hw_preset.parse().unwrap_or(4);
    format!("p{}", 8u32.saturating_sub(val).clamp(1, 7))
}

pub(super) fn build(job: &VideoJob, profile: &Profile, cpu_decode: bool) -> Command {
    let mut cmd = Command::new("ffmpeg");
    let hevc = profile.codec.hevc_or_default();

    // Linux decodes on the GPU; elsewhere CPU frames go to NVENC.
    if !cpu_decode {
        cmd.arg("-hwaccel").arg("cuda");
    }

    cmd.arg("-i").arg(&job.input_path);
    cmd.arg("-progress").arg("-").arg("-nostats");

    // Video filters (fps/scale)
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
    }
    if !filters.is_empty() {
        cmd.arg("-vf").arg(filters.join(","));
    }

    cmd.arg("-c:v").arg("hevc_nvenc");

    // Constant quality: VBR with `-cq` and no bitrate cap (`-b:v 0`).
    cmd.arg("-rc").arg("vbr");
    cmd.arg("-cq").arg(hevc.nvenc_quality.min(51).to_string());
    cmd.arg("-b:v").arg("0");
    cmd.arg("-preset").arg(nvenc_preset(&hevc.hw_preset));
    if hevc.hw_lookahead > 0 {
        // NVENC accepts at most 32 lookahead frames and 4 B-frames.
        cmd.arg("-rc-lookahead")
            .arg(hevc.hw_lookahead.min(32).to_string());
    }
    cmd.arg("-bf").arg(hevc.hw_b_frames.min(4).to_string());

    // Pixel format: 10-bit gives Main10; "auto" keeps the source's depth.
    if profile.pix_fmt != "auto" {
        if is_ten_bit_pix_fmt(&profile.pix_fmt) {
            cmd.arg("-pix_fmt").arg("p010le");
            cmd.arg("-profile:v").arg("main10");
        } else {
            cmd.arg("-pix_fmt").arg("nv12");
        }
    }

    cmd.arg("-g:v").arg(&profile.gop_length);

    apply_color_metadata(&mut cmd, profile);
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
