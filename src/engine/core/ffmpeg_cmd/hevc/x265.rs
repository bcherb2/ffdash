//! libx265 (the HEVC Software Fallback).

use std::process::Command;

use super::super::container_from_output;
use super::super::{
    apply_audio_settings, apply_color_metadata, apply_job_additional_args, fps_scale_filters,
};
use super::apply_hevc_container_tag;
use crate::engine::core::is_ten_bit_pix_fmt;
use crate::engine::core::profile::Profile;
use crate::engine::core::types::VideoJob;
use crate::engine::probe;

/// HDR10 signalling for x265. Colour primaries/transfer/matrix reach the stream through
/// the shared `-color_*` flags; mastering display and content light level are carried
/// over from the source frames by FFmpeg's libx265 wrapper.
fn x265_params(profile: &Profile) -> Option<String> {
    let mut params = Vec::new();
    if profile.color_trc == 16 {
        // PQ transfer: HDR10 signalling.
        params.push("hdr10=1");
        if is_ten_bit_pix_fmt(&profile.pix_fmt) {
            params.push("hdr10-opt=1");
        }
        params.push("repeat-headers=1");
    }
    if params.is_empty() {
        None
    } else {
        Some(params.join(":"))
    }
}

pub(super) fn build(job: &VideoJob, profile: &Profile) -> Command {
    let mut cmd = Command::new("ffmpeg");
    let hevc = profile.codec.hevc_or_default();

    cmd.arg("-i").arg(&job.input_path);
    cmd.arg("-progress").arg("-").arg("-nostats");

    cmd.arg("-c:v").arg("libx265");
    cmd.arg("-crf").arg(hevc.x265_crf.to_string());
    cmd.arg("-preset").arg(&hevc.x265_preset);
    if hevc.x265_tune != "none" {
        cmd.arg("-tune").arg(&hevc.x265_tune);
    }
    if let Some(params) = x265_params(profile) {
        cmd.arg("-x265-params").arg(params);
    }

    // Pixel format: 10-bit gives Main10; "auto" keeps the source's depth.
    if profile.pix_fmt != "auto" {
        cmd.arg("-pix_fmt").arg(&profile.pix_fmt);
    }

    cmd.arg("-g:v").arg(&profile.gop_length);

    if profile.threads > 0 {
        cmd.arg("-threads").arg(profile.threads.to_string());
    }

    apply_color_metadata(&mut cmd, profile);
    apply_hevc_container_tag(&mut cmd, job, profile);

    // Video filters (fps, scale, HDR->SDR tonemap)
    let mut filters = Vec::new();
    if let Ok(input_info) = probe::probe_input_info(&job.input_path) {
        filters.extend(fps_scale_filters(profile, &input_info));

        if profile.colorspace == 1 && profile.color_trc == 1 && input_info.is_hdr {
            filters.push("zscale=t=linear:npl=100".to_string());
            filters.push("tonemap=hable:desat=0".to_string());
            filters.push("zscale=t=bt709:m=bt709:r=tv".to_string());
            filters.push("format=yuv420p".to_string());
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
