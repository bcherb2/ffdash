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
            "hevc-nvenc" => Some(Self::hevc_nvenc()),
            "hevc-qsv" => Some(Self::hevc_qsv()),
            "hevc-vaapi" => Some(Self::hevc_vaapi()),
            "hevc-amf" => Some(Self::hevc_amf()),
            "hevc-videotoolbox" => Some(Self::hevc_videotoolbox()),
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

    fn hevc_nvenc() -> Self {
        Self::hevc_base(
            "hevc-nvenc",
            "hevcnvenc",
            "hevc_nvenc",
            true,
            HevcConfig::default(),
        )
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

    fn hevc_qsv() -> Self {
        Self::hevc_base(
            "hevc-qsv",
            "hevcqsv",
            "hevc_qsv",
            true,
            HevcConfig::default(),
        )
    }

    fn hevc_amf() -> Self {
        Self::hevc_base(
            "hevc-amf",
            "hevcamf",
            "hevc_amf",
            true,
            HevcConfig::default(),
        )
    }

    fn hevc_videotoolbox() -> Self {
        Self::hevc_base(
            "hevc-videotoolbox",
            "hevcvt",
            "hevc_videotoolbox",
            true,
            HevcConfig::default(),
        )
    }

    fn hevc_vaapi() -> Self {
        Self::hevc_base(
            "hevc-vaapi",
            "hevcvaapi",
            "hevc_vaapi",
            true,
            HevcConfig::default(),
        )
    }
}
