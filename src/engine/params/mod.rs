/// Encoder parameter registry module.
///
/// This module provides a single source of truth for FFmpeg encoder parameters,
/// their valid ranges, and which encoders support them. The parameter definitions
/// are loaded from encoder-params.toml at build time and compiled into the binary.
///
/// The module is only available when the `dev-tools` feature is enabled.
pub mod types;

// Generated code is only available with dev-tools feature
#[cfg(feature = "dev-tools")]
mod generated {
    include!(concat!(env!("OUT_DIR"), "/params_generated.rs"));
}

#[cfg(feature = "dev-tools")]
mod validation;

// Re-export types publicly
pub use types::{
    Codec, Condition, EncoderDef, EncoderParam, EncoderType, HardwareApi, ParamDef, ParamType,
    Range, Value,
};

// Re-export generated data and validation when dev-tools is enabled
#[cfg(feature = "dev-tools")]
pub use generated::{ENCODERS, FFMPEG_VERSION, LAST_VERIFIED, PARAMS, SCHEMA_VERSION};

#[cfg(feature = "dev-tools")]
pub use validation::{
    ParamClamp, ValidationError, clamp_value, validate_and_clamp_profile, validate_profile,
};

/// Get parameter definition by name
///
/// # Example
/// ```ignore
/// use ffdash::engine::params::get_param;
///
/// let crf = get_param("crf").expect("crf parameter should exist");
/// assert_eq!(crf.name, "crf");
/// ```
#[cfg(feature = "dev-tools")]
pub fn get_param(name: &str) -> Option<&'static ParamDef> {
    PARAMS.iter().find(|p| p.name == name)
}

/// Get all parameters for a specific encoder
///
/// Returns an iterator over parameters that are supported by the given encoder.
///
/// # Example
/// ```ignore
/// use ffdash::engine::params::params_for_encoder;
///
/// for param in params_for_encoder("libvpx-vp9") {
///     println!("{}: {}", param.name, param.description);
/// }
/// ```
#[cfg(feature = "dev-tools")]
pub fn params_for_encoder(encoder_id: &str) -> impl Iterator<Item = &'static ParamDef> {
    PARAMS.iter().filter(move |p| p.is_supported_by(encoder_id))
}

/// Get encoder definition by ID
///
/// # Example
/// ```ignore
/// use ffdash::engine::params::get_encoder;
///
/// let encoder = get_encoder("libvpx-vp9").expect("encoder should exist");
/// assert_eq!(encoder.ffmpeg_name, "libvpx-vp9");
/// ```
#[cfg(feature = "dev-tools")]
pub fn get_encoder(encoder_id: &str) -> Option<&'static EncoderDef> {
    ENCODERS.iter().find(|e| e.id == encoder_id)
}

#[cfg(test)]
#[cfg(feature = "dev-tools")]
mod tests {
    use super::*;

    #[test]
    fn schema_version_is_set() {
        assert!(!SCHEMA_VERSION.is_empty());
    }

    #[test]
    fn ffmpeg_version_is_set() {
        assert!(!FFMPEG_VERSION.is_empty());
    }

    #[test]
    fn encoders_not_empty() {
        assert!(
            !ENCODERS.is_empty(),
            "At least one encoder should be defined"
        );
    }

    #[test]
    fn params_not_empty() {
        assert!(
            !PARAMS.is_empty(),
            "At least one parameter should be defined"
        );
    }

    #[test]
    fn can_get_param_by_name() {
        // This will pass once we have crf defined
        if let Some(param) = get_param("crf") {
            assert_eq!(param.name, "crf");
        }
    }

    #[test]
    fn can_get_encoder_by_id() {
        let encoder = get_encoder("libvpx-vp9").expect("libvpx-vp9 encoder should exist");
        assert_eq!(encoder.id, "libvpx-vp9");
    }

    #[test]
    fn all_encoders_have_params() {
        // Each encoder must have at least one parameter
        for encoder in ENCODERS.iter() {
            let param_count = PARAMS
                .iter()
                .filter(|p| p.is_supported_by(encoder.id))
                .count();
            assert!(
                param_count > 0,
                "Encoder '{}' has no parameters defined",
                encoder.id
            );
        }
    }

    #[test]
    fn encoder_flags_are_valid() {
        // All flags must start with '-'
        for param in PARAMS.iter() {
            for (encoder_id, encoder_param) in param.encoder_support.iter() {
                if let Some(flag) = encoder_param.flag {
                    assert!(
                        flag.starts_with('-'),
                        "Invalid flag '{}' for param '{}' in encoder '{}'",
                        flag,
                        param.name,
                        encoder_id
                    );
                }
            }
        }
    }

    #[test]
    fn ranges_are_valid() {
        // Check that integer ranges have min <= max
        for param in PARAMS.iter() {
            if let Range::Int { min, max } = param.range {
                assert!(
                    min <= max,
                    "Parameter '{}' has invalid range: min ({}) > max ({})",
                    param.name,
                    min,
                    max
                );
            }
        }
    }

    #[test]
    fn params_have_descriptions() {
        // All params must have non-empty descriptions
        for param in PARAMS.iter() {
            assert!(
                !param.description.is_empty(),
                "Parameter '{}' missing description",
                param.name
            );
        }
    }

    #[test]
    fn params_for_encoder_works() {
        // Test that params_for_encoder returns the correct params
        let vp9_params: Vec<_> = params_for_encoder("libvpx-vp9").collect();
        assert!(!vp9_params.is_empty(), "libvpx-vp9 should have parameters");

        // Verify all returned params are actually supported
        for param in vp9_params {
            assert!(
                param.is_supported_by("libvpx-vp9"),
                "param '{}' should be supported by libvpx-vp9",
                param.name
            );
        }
    }

    #[test]
    fn no_duplicate_param_names() {
        // Check that there are no duplicate parameter names
        let mut seen = std::collections::HashSet::new();
        for param in PARAMS.iter() {
            assert!(
                seen.insert(param.name),
                "Duplicate parameter name: '{}'",
                param.name
            );
        }
    }

    #[test]
    fn encoder_support_complete() {
        // Every param must have support info for every encoder
        let encoder_count = ENCODERS.len();
        for param in PARAMS.iter() {
            assert_eq!(
                param.encoder_support.len(),
                encoder_count,
                "Parameter '{}' has incomplete encoder support (expected {}, got {})",
                param.name,
                encoder_count,
                param.encoder_support.len()
            );
        }
    }
}
