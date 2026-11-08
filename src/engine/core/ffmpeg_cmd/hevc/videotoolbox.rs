//! hevc_videotoolbox: constant quality via `-q:v` (1-100, higher is better), CPU-decoded
//! frames. Intel Macs reject `-q:v`, fail the Probe Encode, and fall back to libx265.

use std::process::Command;

use super::cpu_fed;
use crate::engine::core::profile::Profile;
use crate::engine::core::types::VideoJob;

/// `_cpu_decode` is accepted for symmetry with the other Encoders: VideoToolbox is fed
/// from CPU decode on every host, so it does not change the command.
pub(super) fn build(job: &VideoJob, profile: &Profile, _cpu_decode: bool) -> Command {
    let hevc = profile.codec.as_hevc().cloned().unwrap_or_default();
    cpu_fed::build(job, profile, "hevc_videotoolbox", |cmd| {
        cmd.arg("-q:v")
            .arg(hevc.videotoolbox_q.clamp(1, 100).to_string());
    })
}
