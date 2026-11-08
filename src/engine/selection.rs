//! Encoder selection as a pure function over Encoder Availability.
//!
//! Nothing here consults the host OS, GPUs or FFmpeg: availability is a plain
//! value built by the caller (today from detection, later from Probe Encodes).

use std::collections::HashMap;

use super::hardware::VideoEncoder;

/// The Codec an output is written in, without any of its settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecKind {
    Vp9,
    Av1,
    Hevc,
}

impl CodecKind {
    /// Every Codec ffdash can encode.
    pub const ALL: [CodecKind; 3] = [Self::Vp9, Self::Av1, Self::Hevc];

    /// Hardware Encoders of this Codec in default priority order:
    /// NVENC, AMF, QSV, VAAPI (VideoToolbox last once it exists).
    pub fn hardware_priority(self) -> &'static [VideoEncoder] {
        use VideoEncoder::*;
        match self {
            Self::Vp9 => &[Vp9Qsv, Vp9Vaapi],
            Self::Av1 => &[Av1Nvenc, Av1Amf, Av1Qsv, Av1Vaapi],
            // Extension point: each HEVC hardware Encoder ticket adds itself here, in
            // priority order (HevcNvenc, HevcAmf, HevcQsv, HevcVaapi, HevcVideotoolbox).
            Self::Hevc => &[HevcNvenc, HevcAmf, HevcQsv, HevcVaapi, HevcVideotoolbox],
        }
    }

    /// The Software Fallback of this Codec.
    pub fn software_fallback(self) -> VideoEncoder {
        match self {
            Self::Vp9 => VideoEncoder::LibvpxVp9,
            Self::Av1 => VideoEncoder::LibsvtAv1,
            Self::Hevc => VideoEncoder::Libx265,
        }
    }
}

/// What one usable Encoder can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncoderCapability {
    /// Whether it can produce 10-bit (Main10) output.
    pub main10: bool,
}

/// Which Encoders can actually be used on this machine, and what each can do.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EncoderAvailability {
    entries: HashMap<VideoEncoder, EncoderCapability>,
}

impl EncoderAvailability {
    /// Record `encoder` as available, with or without Main10 support.
    pub fn with(mut self, encoder: VideoEncoder, main10: bool) -> Self {
        self.entries.insert(encoder, EncoderCapability { main10 });
        self
    }

    pub fn capability(&self, encoder: VideoEncoder) -> Option<EncoderCapability> {
        self.entries.get(&encoder).copied()
    }

    pub fn is_available(&self, encoder: VideoEncoder) -> bool {
        self.entries.contains_key(&encoder)
    }

    /// Whether any hardware Encoder of `codec` is available.
    pub fn has_hardware_for(&self, codec: CodecKind) -> bool {
        codec
            .hardware_priority()
            .iter()
            .any(|e| self.is_available(*e))
    }

    /// Whether any hardware Encoder of any Codec is available.
    pub fn has_any_hardware(&self) -> bool {
        CodecKind::ALL.iter().any(|c| self.has_hardware_for(*c))
    }

    fn usable(&self, encoder: VideoEncoder, bit_depth: u8) -> bool {
        self.capability(encoder)
            .is_some_and(|c| bit_depth <= 8 || c.main10)
    }
}

/// Pick the Encoder for a job.
///
/// - Without `use_hardware`, the Software Fallback.
/// - An available `preferred` Encoder of this Codec wins; otherwise the default
///   priority order applies.
/// - A job needing more than 8 bits never goes to an Encoder without Main10.
/// - If nothing qualifies, the Software Fallback.
pub fn select_encoder(
    codec: CodecKind,
    use_hardware: bool,
    preferred: Option<VideoEncoder>,
    bit_depth: u8,
    availability: &EncoderAvailability,
) -> VideoEncoder {
    if use_hardware {
        let candidates = codec.hardware_priority();
        if let Some(pref) = preferred
            && candidates.contains(&pref)
            && availability.usable(pref, bit_depth)
        {
            return pref;
        }
        if let Some(found) = candidates
            .iter()
            .find(|e| availability.usable(**e, bit_depth))
        {
            return *found;
        }
    }
    codec.software_fallback()
}
