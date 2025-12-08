use super::parameter_mapping::{ParameterCondition, ParameterMapping, ParameterSupport};

/// Returns the complete registry of all Profile parameters and their FFmpeg mappings
///
/// This is the SINGLE SOURCE OF TRUTH for parameter behavior across encoders.
/// When adding a new parameter:
/// 1. Add field to Profile struct (src/engine/mod.rs)
/// 2. Add ONE entry here to declare support level and flags
/// 3. Tests automatically verify it appears in commands
pub fn get_parameter_mappings() -> Vec<ParameterMapping> {
    vec![
        // ========== METADATA / CONFIGURATION ==========
        // These fields don't map to FFmpeg parameters
        ParameterMapping {
            field_name: "name",
            software_flag: None,
            vaapi_flag: None,
            support: ParameterSupport::NotApplicable,
            condition: None,
        },
        ParameterMapping {
            field_name: "suffix",
            software_flag: None,
            vaapi_flag: None,
            support: ParameterSupport::NotApplicable,
            condition: None,
        },
        ParameterMapping {
            field_name: "use_hardware_encoding",
            software_flag: None,
            vaapi_flag: None,
            support: ParameterSupport::NotApplicable,
            condition: None,
        },
        ParameterMapping {
            field_name: "max_workers",
            software_flag: None,
            vaapi_flag: None,
            support: ParameterSupport::NotApplicable,
            condition: None,
        },
        // ========== CONTAINER & CODECS ==========
        ParameterMapping {
            field_name: "container",
            software_flag: None, // Container determined by output filename
            vaapi_flag: None,
            support: ParameterSupport::NotApplicable,
            condition: None,
        },
        ParameterMapping {
            field_name: "video_codec",
            software_flag: None,
            vaapi_flag: None, // Both use -c:v but with different values (libvpx-vp9 vs vp9_vaapi)
            support: ParameterSupport::NotApplicable,
            condition: None,
        },
        ParameterMapping {
            field_name: "audio_codec",
            software_flag: Some("-c:a"),
            vaapi_flag: Some("-c:a"),
            support: ParameterSupport::Both,
            condition: Some(ParameterCondition::Always),
        },
        ParameterMapping {
            field_name: "audio_bitrate",
            software_flag: Some("-b:a"),
            vaapi_flag: Some("-b:a"),
            support: ParameterSupport::Both,
            condition: Some(ParameterCondition::Always),
        },
        // ========== VIDEO OUTPUT CONSTRAINTS ==========
        ParameterMapping {
            field_name: "fps",
            software_flag: Some("fps"), // Filter: fps=fps=N
            vaapi_flag: Some("fps"),    // Filter: fps=N
            support: ParameterSupport::Both,
            condition: Some(ParameterCondition::NonZero),
        },
        ParameterMapping {
            field_name: "scale_width",
            software_flag: Some("scale"),
            vaapi_flag: Some("scale"),
            support: ParameterSupport::Both,
            condition: None, // Custom condition based on input size
        },
        ParameterMapping {
            field_name: "scale_height",
            software_flag: Some("scale"),
            vaapi_flag: Some("scale"),
            support: ParameterSupport::Both,
            condition: None, // Custom condition based on input size
        },
        // ========== RATE CONTROL ==========
        ParameterMapping {
            field_name: "crf",
            software_flag: Some("-crf"),
            vaapi_flag: None, // VAAPI uses global_quality instead
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::Always),
        },
        ParameterMapping {
            field_name: "video_target_bitrate",
            software_flag: Some("-b:v"),
            vaapi_flag: None, // VAAPI forced to CQP mode; bitrates not supported
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::NonZero),
        },
        ParameterMapping {
            field_name: "video_min_bitrate",
            software_flag: Some("-minrate"),
            vaapi_flag: None, // Not supported in VAAPI
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::NonZero),
        },
        ParameterMapping {
            field_name: "video_max_bitrate",
            software_flag: Some("-maxrate"),
            vaapi_flag: None,
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::NonZero),
        },
        ParameterMapping {
            field_name: "video_bufsize",
            software_flag: Some("-bufsize"),
            vaapi_flag: None,
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::NonZero),
        },
        ParameterMapping {
            field_name: "undershoot_pct",
            software_flag: Some("-undershoot-pct"),
            vaapi_flag: None, // Not supported in VAAPI
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::NonNegative),
        },
        ParameterMapping {
            field_name: "overshoot_pct",
            software_flag: Some("-overshoot-pct"),
            vaapi_flag: None, // Not supported in VAAPI
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::NonNegative),
        },
        // ========== SPEED & QUALITY ==========
        ParameterMapping {
            field_name: "cpu_used",
            software_flag: Some("-cpu-used"),
            vaapi_flag: None, // GPU encoding, not CPU-based
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::Always),
        },
        ParameterMapping {
            field_name: "cpu_used_pass1",
            software_flag: Some("-cpu-used"), // Used in pass 1 of 2-pass
            vaapi_flag: None,
            support: ParameterSupport::SoftwareOnly,
            condition: None, // Custom condition for 2-pass
        },
        ParameterMapping {
            field_name: "cpu_used_pass2",
            software_flag: Some("-cpu-used"), // Used in pass 2 of 2-pass
            vaapi_flag: None,
            support: ParameterSupport::SoftwareOnly,
            condition: None, // Custom condition for 2-pass
        },
        ParameterMapping {
            field_name: "two_pass",
            software_flag: None, // Affects command flow, not a direct flag
            vaapi_flag: None,
            support: ParameterSupport::NotApplicable,
            condition: None,
        },
        ParameterMapping {
            field_name: "quality_mode",
            software_flag: Some("-quality"),
            vaapi_flag: None, // Not supported in VAAPI
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::Always),
        },
        // ========== VP9 SETTINGS ==========
        ParameterMapping {
            field_name: "vp9_profile",
            software_flag: Some("-profile:v"),
            vaapi_flag: None, // VAAPI determines profile automatically
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::Always),
        },
        ParameterMapping {
            field_name: "pix_fmt",
            software_flag: Some("-pix_fmt"),
            vaapi_flag: None, // VAAPI uses format=nv12 in filter chain
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::Always),
        },
        // ========== PARALLELISM ==========
        // Tile-based parallelism is libvpx-vp9 specific, NOT supported in VAAPI
        ParameterMapping {
            field_name: "row_mt",
            software_flag: Some("-row-mt"),
            vaapi_flag: None, // Not supported in VAAPI VP9
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::BoolTrue),
        },
        ParameterMapping {
            field_name: "tile_columns",
            software_flag: Some("-tile-columns"),
            vaapi_flag: None, // Not supported in VAAPI VP9
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::NonNegative),
        },
        ParameterMapping {
            field_name: "tile_rows",
            software_flag: Some("-tile-rows"),
            vaapi_flag: None, // Not supported in VAAPI VP9
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::NonNegative),
        },
        ParameterMapping {
            field_name: "threads",
            software_flag: Some("-threads"),
            vaapi_flag: None, // GPU parallelism, not CPU threads
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::NonZero),
        },
        ParameterMapping {
            field_name: "frame_parallel",
            software_flag: Some("-frame-parallel"),
            vaapi_flag: None, // Not supported in VAAPI VP9
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::BoolTrue),
        },
        // ========== GOP & KEYFRAMES ==========
        ParameterMapping {
            field_name: "gop_length",
            software_flag: Some("-g"),
            vaapi_flag: Some("-g"),
            support: ParameterSupport::Both,
            condition: Some(ParameterCondition::Always),
        },
        ParameterMapping {
            field_name: "keyint_min",
            software_flag: Some("-keyint_min"),
            vaapi_flag: None, // Not supported in VAAPI
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::NonZero),
        },
        ParameterMapping {
            field_name: "fixed_gop",
            software_flag: Some("-sc_threshold"), // Set to 0 when fixed_gop=true
            vaapi_flag: None,                     // Not supported in VAAPI
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::BoolTrue),
        },
        ParameterMapping {
            field_name: "lag_in_frames",
            software_flag: Some("-lag-in-frames"),
            vaapi_flag: None, // Not supported in VAAPI
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::Always),
        },
        ParameterMapping {
            field_name: "auto_alt_ref",
            software_flag: Some("-auto-alt-ref"),
            vaapi_flag: None, // Not supported in VAAPI
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::BoolTrue),
        },
        // ========== ALT-REF DENOISING (ARNR) ==========
        // ARNR is a software-only feature
        ParameterMapping {
            field_name: "arnr_max_frames",
            software_flag: Some("-arnr-maxframes"),
            vaapi_flag: None, // Not supported in VAAPI
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::NonZero),
        },
        ParameterMapping {
            field_name: "arnr_strength",
            software_flag: Some("-arnr-strength"),
            vaapi_flag: None, // Not supported in VAAPI
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::NonZero),
        },
        ParameterMapping {
            field_name: "arnr_type",
            software_flag: Some("-arnr-type"),
            vaapi_flag: None, // Not supported in VAAPI
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::NonNegative),
        },
        // ========== ADVANCED TUNING ==========
        // These are software encoder optimizations not available in VAAPI
        ParameterMapping {
            field_name: "enable_tpl",
            software_flag: Some("-enable-tpl"),
            vaapi_flag: None, // Not supported in VAAPI
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::BoolTrue),
        },
        ParameterMapping {
            field_name: "sharpness",
            software_flag: Some("-sharpness"),
            vaapi_flag: None, // Not supported in VAAPI
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::NonNegative),
        },
        ParameterMapping {
            field_name: "noise_sensitivity",
            software_flag: Some("-noise-sensitivity"),
            vaapi_flag: None, // Not supported in VAAPI
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::NonZero),
        },
        ParameterMapping {
            field_name: "static_thresh",
            software_flag: Some("-static-thresh"),
            vaapi_flag: None, // Not supported in VAAPI
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::NonZero),
        },
        ParameterMapping {
            field_name: "max_intra_rate",
            software_flag: Some("-max-intra-rate"),
            vaapi_flag: None, // Not supported in VAAPI
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::NonZero),
        },
        ParameterMapping {
            field_name: "aq_mode",
            software_flag: Some("-aq-mode"),
            vaapi_flag: None, // Not supported in VAAPI
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::NonNegative),
        },
        ParameterMapping {
            field_name: "tune_content",
            software_flag: Some("-tune-content"),
            vaapi_flag: None, // Not supported in VAAPI
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::NonEmpty),
        },
        // ========== COLOR / HDR METADATA ==========
        // Color metadata is software-only
        ParameterMapping {
            field_name: "colorspace",
            software_flag: Some("-colorspace"),
            vaapi_flag: None, // Not supported in VAAPI
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::NonNegative),
        },
        ParameterMapping {
            field_name: "color_primaries",
            software_flag: Some("-color_primaries"),
            vaapi_flag: None, // Not supported in VAAPI
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::NonNegative),
        },
        ParameterMapping {
            field_name: "color_trc",
            software_flag: Some("-color_trc"),
            vaapi_flag: None, // Not supported in VAAPI
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::NonNegative),
        },
        ParameterMapping {
            field_name: "color_range",
            software_flag: Some("-color_range"),
            vaapi_flag: None, // Not supported in VAAPI
            support: ParameterSupport::SoftwareOnly,
            condition: Some(ParameterCondition::NonNegative),
        },
        // ========== HARDWARE ENCODING SETTINGS (VAAPI-ONLY) ==========
        ParameterMapping {
            field_name: "hw_global_quality",
            software_flag: None,
            vaapi_flag: Some("-global_quality"),
            support: ParameterSupport::VaapiOnly,
            condition: Some(ParameterCondition::Always),
        },
        ParameterMapping {
            field_name: "hw_b_frames",
            software_flag: None,
            vaapi_flag: Some("-bf"),
            support: ParameterSupport::VaapiOnly,
            condition: Some(ParameterCondition::NonZero),
        },
        ParameterMapping {
            field_name: "hw_loop_filter_level",
            software_flag: None,
            vaapi_flag: Some("-loop_filter_level"),
            support: ParameterSupport::VaapiOnly,
            condition: Some(ParameterCondition::Always),
        },
        ParameterMapping {
            field_name: "hw_loop_filter_sharpness",
            software_flag: None,
            vaapi_flag: Some("-loop_filter_sharpness"),
            support: ParameterSupport::VaapiOnly,
            condition: Some(ParameterCondition::Always),
        },
    ]
}

/// Get statistics about parameter coverage
pub fn get_parameter_statistics() -> ParameterStatistics {
    let mappings = get_parameter_mappings();

    let both = mappings
        .iter()
        .filter(|m| m.support == ParameterSupport::Both)
        .count();
    let software_only = mappings
        .iter()
        .filter(|m| m.support == ParameterSupport::SoftwareOnly)
        .count();
    let vaapi_only = mappings
        .iter()
        .filter(|m| m.support == ParameterSupport::VaapiOnly)
        .count();
    let not_applicable = mappings
        .iter()
        .filter(|m| m.support == ParameterSupport::NotApplicable)
        .count();

    ParameterStatistics {
        total: mappings.len(),
        both,
        software_only,
        vaapi_only,
        not_applicable,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterStatistics {
    pub total: usize,
    pub both: usize,
    pub software_only: usize,
    pub vaapi_only: usize,
    pub not_applicable: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_not_empty() {
        let mappings = get_parameter_mappings();
        assert!(
            !mappings.is_empty(),
            "Parameter registry should not be empty"
        );
    }

    #[test]
    fn test_registry_has_minimum_fields() {
        let mappings = get_parameter_mappings();
        // Profile struct has 46+ fields, registry should have at least 45
        assert!(
            mappings.len() >= 45,
            "Parameter registry has only {} entries, expected at least 45",
            mappings.len()
        );
    }

    #[test]
    fn test_parallelism_parameters_marked_as_software_only() {
        let mappings = get_parameter_mappings();

        // These parameters are NOT supported in VAAPI - must be marked as SoftwareOnly
        // The bug was that they were being treated as if they worked on both encoders
        let critical_params = vec!["row_mt", "tile_columns", "tile_rows", "frame_parallel"];

        for param in critical_params {
            let mapping = mappings
                .iter()
                .find(|m| m.field_name == param)
                .unwrap_or_else(|| panic!("Critical parameter '{}' missing from registry", param));

            assert_eq!(
                mapping.support,
                ParameterSupport::SoftwareOnly,
                "Parameter '{}' should be ParameterSupport::SoftwareOnly (not supported in VAAPI)",
                param
            );
        }
    }

    #[test]
    fn test_statistics_sum_equals_total() {
        let stats = get_parameter_statistics();
        assert_eq!(
            stats.both + stats.software_only + stats.vaapi_only + stats.not_applicable,
            stats.total,
            "Sum of categories should equal total"
        );
    }
}
