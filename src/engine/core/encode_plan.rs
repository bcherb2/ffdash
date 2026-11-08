//! The Encode Plan: what a single encode runs with.

use super::profile::Profile;
use crate::engine::hardware::{self, VideoEncoder};

/// What a single encode will run with: the selected Encoder and, optionally,
/// a Quality Override (as set by Auto-VMAF).
///
/// Whether hardware is used is derived from the Encoder's Backend, never from
/// whether some config object is present.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncodePlan {
    pub encoder: VideoEncoder,
    pub quality_override: Option<u32>,
}

impl EncodePlan {
    pub fn new(encoder: VideoEncoder) -> Self {
        Self {
            encoder,
            quality_override: None,
        }
    }

    pub fn with_quality_override(mut self, quality: u32) -> Self {
        self.quality_override = Some(quality);
        self
    }

    /// Select the Encoder for `profile` on this machine.
    /// `hardware_requested` is the caller's explicit request on top of the profile setting.
    pub fn for_profile(profile: &Profile, hardware_requested: bool) -> Self {
        let mut synced = profile.clone();
        synced.sync_legacy_fields();
        let use_hardware = hardware_requested || synced.use_hardware_encoding;
        Self::new(hardware::select_encoder(
            &synced.codec,
            use_hardware,
            Some(&synced.video_codec),
        ))
    }

    /// Whether this plan encodes on a GPU.
    pub fn uses_hardware(&self) -> bool {
        self.encoder.backend().is_hardware()
    }
}
