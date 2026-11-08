//! hevc_amf: constant-QP rate control, CPU-decoded frames.

use std::process::Command;

use super::cpu_fed;
use crate::engine::core::profile::Profile;
use crate::engine::core::types::VideoJob;

/// Map the shared hardware preset (1 = best quality .. 7 = fastest) to AMF's `-quality`.
fn amf_quality(hw_preset: &str) -> &'static str {
    match hw_preset.parse::<u32>().unwrap_or(4) {
        0..=2 => "quality",
        3..=5 => "balanced",
        _ => "speed",
    }
}

/// `_cpu_decode` is accepted for symmetry with the other Encoders: AMF is fed from CPU
/// decode on every host, so it does not change the command.
pub(super) fn build(job: &VideoJob, profile: &Profile, _cpu_decode: bool) -> Command {
    let hevc = profile.codec.as_hevc().cloned().unwrap_or_default();
    cpu_fed::build(job, profile, "hevc_amf", |cmd| {
        cmd.arg("-rc").arg("cqp");
        cmd.arg("-qp_i").arg(hevc.amf_qp.min(51).to_string());
        cmd.arg("-qp_p").arg(hevc.amf_qp.min(51).to_string());
        cmd.arg("-quality").arg(amf_quality(&hevc.hw_preset));
    })
}
