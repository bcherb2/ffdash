#![allow(dead_code)] // Mapping oracles sleep here until parity breaks again

use ffdash::engine::Profile;

/// Defines which encoder(s) support a given parameter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterSupport {
    /// Parameter works in both software (libvpx-vp9) AND VAAPI hardware encoding
    Both,
    /// Parameter only works with software encoding (libvpx-vp9)
    SoftwareOnly,
    /// Parameter only works with VAAPI hardware encoding
    VaapiOnly,
    /// Not a command parameter (metadata, runtime config, etc.)
    NotApplicable,
}

/// Defines when a parameter should be included in the FFmpeg command
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterCondition {
    /// Always include in command
    Always,
    /// Include only if value is non-zero
    NonZero,
    /// Include only if value is non-negative (>= 0)
    NonNegative,
    /// Include only if boolean is true
    BoolTrue,
    /// Include only if string is non-empty
    NonEmpty,
}

/// Maps a Profile struct field to its FFmpeg command-line flag(s)
#[derive(Debug, Clone)]
pub struct ParameterMapping {
    /// The name of the field in the Profile struct
    pub field_name: &'static str,

    /// The FFmpeg flag for software encoding (libvpx-vp9)
    /// None if not applicable to software encoding
    pub software_flag: Option<&'static str>,

    /// The FFmpeg flag for VAAPI hardware encoding
    /// None if not applicable to VAAPI
    pub vaapi_flag: Option<&'static str>,

    /// Which encoder(s) support this parameter
    pub support: ParameterSupport,

    /// When to include this parameter in the command
    /// None means use a custom condition function
    pub condition: Option<ParameterCondition>,
}

impl ParameterMapping {
    /// Check if this parameter should be included in a software command
    pub fn should_be_in_software(&self, profile: &Profile) -> bool {
        if !matches!(
            self.support,
            ParameterSupport::Both | ParameterSupport::SoftwareOnly
        ) {
            return false;
        }

        self.check_condition(profile)
    }

    /// Check if this parameter should be included in a VAAPI command
    pub fn should_be_in_vaapi(&self, profile: &Profile) -> bool {
        if !matches!(
            self.support,
            ParameterSupport::Both | ParameterSupport::VaapiOnly
        ) {
            return false;
        }

        self.check_condition(profile)
    }

    /// Check if the parameter's condition is satisfied
    fn check_condition(&self, profile: &Profile) -> bool {
        match self.condition {
            Some(ParameterCondition::Always) => true,
            Some(ParameterCondition::NonZero) => self.is_non_zero(profile),
            Some(ParameterCondition::NonNegative) => self.is_non_negative(profile),
            Some(ParameterCondition::BoolTrue) => self.is_bool_true(profile),
            Some(ParameterCondition::NonEmpty) => self.is_non_empty(profile),
            None => true, // Custom condition, assume true
        }
    }

    /// Check if numeric field is non-zero (using runtime reflection)
    fn is_non_zero(&self, profile: &Profile) -> bool {
        match self.field_name {
            "video_target_bitrate" => profile.video_target_bitrate > 0,
            "video_min_bitrate" => profile.video_min_bitrate > 0,
            "video_max_bitrate" => profile.video_max_bitrate > 0,
            "video_bufsize" => profile.video_bufsize > 0,
            "fps" => profile.fps > 0,
            "keyint_min" => profile.keyint_min > 0,
            "lag_in_frames" => profile.lag_in_frames > 0,
            "arnr_max_frames" => profile.arnr_max_frames > 0,
            "noise_sensitivity" => profile.noise_sensitivity > 0,
            "static_thresh" => profile.static_thresh > 0,
            "max_intra_rate" => profile.max_intra_rate > 0,
            "hw_b_frames" => profile.hw_b_frames > 0,
            _ => false,
        }
    }

    /// Check if numeric field is non-negative (>= 0)
    fn is_non_negative(&self, profile: &Profile) -> bool {
        match self.field_name {
            "tile_columns" => profile.tile_columns >= 0,
            "tile_rows" => profile.tile_rows >= 0,
            "sharpness" => profile.sharpness >= 0,
            "aq_mode" => profile.aq_mode >= 0,
            "arnr_type" => profile.arnr_type >= 0,
            "colorspace" => profile.colorspace >= 0,
            "color_primaries" => profile.color_primaries >= 0,
            "color_trc" => profile.color_trc >= 0,
            "color_range" => profile.color_range >= 0,
            _ => false,
        }
    }

    /// Check if boolean field is true
    fn is_bool_true(&self, profile: &Profile) -> bool {
        match self.field_name {
            "row_mt" => profile.row_mt,
            "frame_parallel" => profile.frame_parallel,
            "two_pass" => profile.two_pass,
            "fixed_gop" => profile.fixed_gop,
            "auto_alt_ref" => profile.auto_alt_ref,
            "enable_tpl" => profile.enable_tpl,
            _ => false,
        }
    }

    /// Check if string field is non-empty
    fn is_non_empty(&self, profile: &Profile) -> bool {
        match self.field_name {
            "quality_mode" => !profile.quality_mode.is_empty() && profile.quality_mode != "good",
            "tune_content" => !profile.tune_content.is_empty() && profile.tune_content != "default",
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_support_equality() {
        assert_eq!(ParameterSupport::Both, ParameterSupport::Both);
        assert_ne!(ParameterSupport::Both, ParameterSupport::SoftwareOnly);
    }

    #[test]
    fn test_parameter_condition_check() {
        let mapping = ParameterMapping {
            field_name: "row_mt",
            software_flag: Some("-row-mt"),
            vaapi_flag: Some("-row-mt"),
            support: ParameterSupport::Both,
            condition: Some(ParameterCondition::BoolTrue),
        };

        let mut profile = Profile::get("vp9-good");
        profile.row_mt = false;
        assert!(!mapping.check_condition(&profile));

        profile.row_mt = true;
        assert!(mapping.check_condition(&profile));
    }
}
