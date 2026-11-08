//! HEVC command building: one module per Encoder, one dispatch arm per Encoder.
//!
//! Adding a hardware Encoder: create `hevc/<encoder>.rs` exposing
//! `build(job, profile, cpu_decode) -> Command`, declare it below, and replace that
//! Encoder's arm in [`build_hevc_cmd`]. Nothing else in this file changes.
//! Shared helpers ([`apply_hevc_container_tag`]) live here.

use std::process::Command;

use super::super::encode_plan::EncodePlan;
use super::super::profile::Profile;
use super::super::types::VideoJob;
use super::container_from_output;
use crate::engine::hardware::VideoEncoder;

mod x265;

mod nvenc;

mod qsv;

// mod vaapi; (hevc_vaapi ticket)

// mod amf; (hevc_amf ticket)

// mod videotoolbox; (hevc_videotoolbox ticket)

/// Build the command for an HEVC Encode Plan.
pub(super) fn build_hevc_cmd(job: &VideoJob, profile: &Profile, plan: &EncodePlan) -> Command {
    let cpu_decode = plan.host.decodes_on_cpu();
    match plan.encoder {
        VideoEncoder::Libx265 => x265::build(job, profile),

        // Hardware Encoders: each arm moves to its own module when its ticket lands.
        // Until then they build the Software Fallback command.
        // hevc_nvenc
        VideoEncoder::HevcNvenc => nvenc::build(job, profile, cpu_decode),
        // hevc_qsv
        VideoEncoder::HevcQsv => qsv::build(job, profile, cpu_decode),
        // hevc_vaapi
        VideoEncoder::HevcVaapi => x265::build(job, profile),
        // hevc_amf
        VideoEncoder::HevcAmf => x265::build(job, profile),
        // hevc_videotoolbox
        VideoEncoder::HevcVideotoolbox => x265::build(job, profile),

        // Not HEVC Encoders: the Software Fallback.
        _ => x265::build(job, profile),
    }
}

/// MP4 output always carries `-tag:v hvc1` (QuickTime and Apple players need it).
/// Shared by every HEVC Encoder; call it once per command.
pub(super) fn apply_hevc_container_tag(cmd: &mut Command, job: &VideoJob, profile: &Profile) {
    if matches!(
        container_from_output(job, profile).as_str(),
        "mp4" | "m4v" | "mov"
    ) {
        cmd.arg("-tag:v").arg("hvc1");
    }
}
