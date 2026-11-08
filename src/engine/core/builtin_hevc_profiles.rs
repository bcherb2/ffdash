//! Built-in HEVC profiles (`hevc-x265`, and one per hardware Encoder as they land).
//!
//! Each profile is its own constructor and its own arm in [`Profile::hevc`], so
//! adding an Encoder's profile touches only its own lines.

use super::av1_config::Codec;
use super::hevc_config::HevcConfig;
use super::profile::Profile;

impl Profile {
    /// Built-in HEVC profile by internal name (`hevc-*`).
    /// Returns `None` for names that are not (yet) HEVC profiles.
    pub(super) fn hevc(name: &str) -> Option<Self> {
        match name {
            "hevc-x265" => Some(Self::hevc_x265()),
            // hevc-nvenc: added by the NVENC ticket.
            // hevc-qsv: added by the QSV ticket.
            // hevc-vaapi: added by the VAAPI ticket.
            // hevc-amf: added by the AMF ticket.
            // hevc-videotoolbox: added by the VideoToolbox ticket.
            _ => None,
        }
    }

    /// Shared skeleton of every built-in HEVC profile: MKV output, `auto` pixel format
    /// (Main10 follows a 10-bit source), no HDR/colour overrides.
    fn hevc_base(
        name: &str,
        suffix: &str,
        video_codec: &str,
        use_hardware_encoding: bool,
        config: HevcConfig,
    ) -> Self {
        Self {
            name: name.to_string(),
            suffix: suffix.to_string(),
            container: "mkv".to_string(),
            video_codec: video_codec.to_string(),
            pix_fmt: "auto".to_string(),
            crf: config.x265_crf,
            use_hardware_encoding,
            codec: Codec::Hevc(config),
            ..Self::get("av1-svt")
        }
    }

    fn hevc_x265() -> Self {
        Self::hevc_base(
            "hevc-x265",
            "hevcx265",
            "libx265",
            false,
            HevcConfig::default(),
        )
    }
}
