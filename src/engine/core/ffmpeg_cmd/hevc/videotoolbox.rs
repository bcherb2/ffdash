//! hevc_videotoolbox: constant quality via `-q:v` (1-100, higher is better), CPU-decoded
//! frames. Intel Macs reject `-q:v`, fail the Probe Encode, and fall back to libx265.

use std::process::Command;

use super::cpu_decoded_hardware;
use crate::engine::core::profile::Profile;
use crate::engine::core::types::VideoJob;

pub(super) fn build(job: &VideoJob, profile: &Profile) -> Command {
    let hevc = profile.codec.hevc_or_default();
    cpu_decoded_hardware::build(job, profile, "hevc_videotoolbox", |cmd| {
        cmd.arg("-q:v")
            .arg(hevc.videotoolbox_q.clamp(1, 100).to_string());
    })
}
