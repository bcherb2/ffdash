/// Profile validation against the encoder parameter registry.
///
/// This module provides functionality to validate encoding profiles against
/// the parameter registry to ensure all parameters are valid for the target encoder.
use std::collections::HashMap;
use thiserror::Error;

use super::types::{Range, Value};
use super::{get_encoder, get_param, params_for_encoder};
use crate::engine::core::Profile;

/// Validation error types
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Encoder '{0}' not found in registry")]
    EncoderNotFound(String),

    #[error("Parameter '{0}' not found in registry")]
    UnknownParameter(String),

    #[error("Parameter '{param}' value {value:?} out of range")]
    OutOfRange {
        param: String,
        value: Value,
        range: Range,
    },

    #[error("Parameter '{0}' not supported by encoder '{1}'")]
    UnsupportedByEncoder(String, String),

    #[error("Parameter '{param}' has wrong type: expected {expected}, got {actual}")]
    TypeMismatch {
        param: String,
        expected: String,
        actual: String,
    },
}

/// Record of a parameter value that was clamped to valid range
#[derive(Debug, Clone)]
pub struct ParamClamp {
    pub param: String,
    pub original: Value,
    pub clamped: Value,
    pub range: Range,
}

/// Validate a profile against the encoder parameter registry
///
/// # Arguments
/// * `encoder_id` - The encoder ID (e.g., "libvpx-vp9", "vp9_vaapi")
/// * `params` - Map of parameter names to their values
///
/// # Returns
/// * `Ok(())` if validation passes
/// * `Err(ValidationError)` describing the first validation error found
///
/// # Example
/// ```ignore
/// use std::collections::HashMap;
/// use ffdash::engine::params::{validate_profile, Value};
///
/// let mut params = HashMap::new();
/// params.insert("crf".to_string(), Value::I32(31));
/// params.insert("cpu-used".to_string(), Value::I32(1));
///
/// validate_profile("libvpx-vp9", &params)?;
/// ```
pub fn validate_profile(
    encoder_id: &str,
    params: &HashMap<String, Value>,
) -> Result<(), ValidationError> {
    // Check if encoder exists
    let _encoder = get_encoder(encoder_id)
        .ok_or_else(|| ValidationError::EncoderNotFound(encoder_id.to_string()))?;

    // Validate each parameter
    for (param_name, value) in params {
        // Check if parameter exists in registry
        let param_def = get_param(param_name)
            .ok_or_else(|| ValidationError::UnknownParameter(param_name.clone()))?;

        // Check if encoder supports this parameter
        if !param_def.is_supported_by(encoder_id) {
            return Err(ValidationError::UnsupportedByEncoder(
                param_name.clone(),
                encoder_id.to_string(),
            ));
        }

        // Get encoder-specific config (might have range override)
        let encoder_config = param_def
            .get_encoder_config(encoder_id)
            .expect("Should exist since is_supported_by returned true");

        // Use encoder-specific range if available, otherwise use default range
        let range = encoder_config
            .range_override
            .as_ref()
            .unwrap_or(&param_def.range);

        // Validate value is within range
        validate_value_in_range(param_name, value, range)?;
    }

    Ok(())
}

/// Clamp a value to a valid range, returning the clamped value if it was out of range
///
/// # Arguments
/// * `value` - The value to clamp
/// * `range` - The valid range
///
/// # Returns
/// * `Some(Value)` - The clamped value if it was out of range
/// * `None` - If the value was already within range
pub fn clamp_value(value: &Value, range: &Range) -> Option<Value> {
    match (value, range) {
        // Unsigned integers
        (Value::U8(v), Range::Int { min, max }) => {
            let clamped = (*v as i64).clamp(*min, *max) as u8;
            if clamped != *v {
                Some(Value::U8(clamped))
            } else {
                None
            }
        }
        (Value::U16(v), Range::Int { min, max }) => {
            let clamped = (*v as i64).clamp(*min, *max) as u16;
            if clamped != *v {
                Some(Value::U16(clamped))
            } else {
                None
            }
        }
        (Value::U32(v), Range::Int { min, max }) => {
            let clamped = (*v as i64).clamp(*min, *max) as u32;
            if clamped != *v {
                Some(Value::U32(clamped))
            } else {
                None
            }
        }
        (Value::U64(v), Range::Int { min, max }) => {
            let clamped_i64 = (*v as i64).clamp(*min, *max);
            let clamped = clamped_i64 as u64;
            if clamped != *v {
                Some(Value::U64(clamped))
            } else {
                None
            }
        }
        // Signed integers
        (Value::I8(v), Range::Int { min, max }) => {
            let clamped = (*v as i64).clamp(*min, *max) as i8;
            if clamped != *v {
                Some(Value::I8(clamped))
            } else {
                None
            }
        }
        (Value::I16(v), Range::Int { min, max }) => {
            let clamped = (*v as i64).clamp(*min, *max) as i16;
            if clamped != *v {
                Some(Value::I16(clamped))
            } else {
                None
            }
        }
        (Value::I32(v), Range::Int { min, max }) => {
            let clamped = (*v as i64).clamp(*min, *max) as i32;
            if clamped != *v {
                Some(Value::I32(clamped))
            } else {
                None
            }
        }
        (Value::I64(v), Range::Int { min, max }) => {
            let clamped = (*v).clamp(*min, *max);
            if clamped != *v {
                Some(Value::I64(clamped))
            } else {
                None
            }
        }
        // Floats
        (Value::F32(v), Range::Float { min, max }) => {
            let clamped = (*v as f64).clamp(*min, *max) as f32;
            if (clamped - *v).abs() > f32::EPSILON {
                Some(Value::F32(clamped))
            } else {
                None
            }
        }
        (Value::F64(v), Range::Float { min, max }) => {
            let clamped = (*v).clamp(*min, *max);
            if (clamped - *v).abs() > f64::EPSILON {
                Some(Value::F64(clamped))
            } else {
                None
            }
        }
        // Strings and enums - can't clamp, would need validation
        (Value::String(_), Range::Enum { .. }) => None,
        (Value::Str(_), Range::Enum { .. }) => None,
        // Booleans - already valid
        (Value::Bool(_), Range::Bool) => None,
        // Any range - always valid
        (_, Range::Any) => None,
        // Mismatched types - can't clamp
        _ => None,
    }
}

/// Validate and clamp a Profile's parameters against the PARAMS registry
///
/// This is a high-level function that:
/// 1. Extracts relevant parameter values from the Profile
/// 2. Validates them against the encoder's parameter ranges
/// 3. Clamps out-of-range values and updates the Profile in-place
/// 4. Returns a list of all parameters that were clamped
///
/// # Arguments
/// * `profile` - Mutable reference to the Profile to validate and clamp
/// * `encoder_id` - The encoder ID (e.g., "libvpx-vp9", "vp9_qsv", "av1_qsv")
///
/// # Returns
/// * `Vec<ParamClamp>` - List of parameters that were clamped (empty if all were valid)
///
/// # Note
/// This function modifies the Profile in-place, clamping any out-of-range values.
/// It only validates parameters that are relevant to the specified encoder.
pub fn validate_and_clamp_profile(profile: &mut Profile, encoder_id: &str) -> Vec<ParamClamp> {
    let mut clamps = Vec::new();

    // Check if encoder exists
    let Some(_encoder) = get_encoder(encoder_id) else {
        return clamps; // Unknown encoder, skip validation
    };

    let params: Vec<_> = params_for_encoder(encoder_id).collect();

    // Iterate through all parameters for this encoder
    for param_def in params {
        // Extract current value from Profile based on field_path
        let current_value = match extract_profile_field(profile, param_def.field_path) {
            Some(v) => v,
            None => continue, // Field not found or not applicable
        };

        // Get the effective range (considering encoder-specific overrides)
        let range = param_def
            .get_encoder_config(encoder_id)
            .and_then(|cfg| cfg.range_override.as_ref())
            .unwrap_or(&param_def.range);

        // Check if value needs clamping
        if let Some(clamped) = clamp_value(&current_value, range) {
            // Apply clamped value back to Profile
            apply_profile_field(profile, param_def.field_path, &clamped);

            // Record the clamp
            clamps.push(ParamClamp {
                param: param_def.name.to_string(),
                original: current_value,
                clamped,
                range: range.clone(),
            });
        }
    }

    clamps
}

/// Extract a field value from a Profile as a PARAMS Value
///
/// Maps Profile struct fields to Value enum variants based on field name.
/// For codec-specific fields, reads from the codec config (source of truth).
fn extract_profile_field(profile: &Profile, field_name: &str) -> Option<Value> {
    use crate::engine::core::Codec;

    match field_name {
        "crf" => Some(Value::U32(profile.crf)),
        "cpu_used" => {
            // Read from codec config if available
            match &profile.codec {
                Codec::Vp9(vp9) => Some(Value::U32(vp9.cpu_used)),
                _ => Some(Value::U32(profile.cpu_used)),
            }
        }
        "cpu_used_pass1" => match &profile.codec {
            Codec::Vp9(vp9) => Some(Value::U32(vp9.cpu_used_pass1)),
            _ => Some(Value::U32(profile.cpu_used_pass1)),
        },
        "cpu_used_pass2" => match &profile.codec {
            Codec::Vp9(vp9) => Some(Value::U32(vp9.cpu_used_pass2)),
            _ => Some(Value::U32(profile.cpu_used_pass2)),
        },
        "quality_mode" => match &profile.codec {
            Codec::Vp9(vp9) => Some(Value::String(vp9.quality_mode.clone())),
            _ => Some(Value::String(profile.quality_mode.clone())),
        },
        "row_mt" => match &profile.codec {
            Codec::Vp9(vp9) => Some(Value::Bool(vp9.row_mt)),
            _ => Some(Value::Bool(profile.row_mt)),
        },
        "tile_columns" => Some(Value::I32(profile.tile_columns)),
        "tile_rows" => Some(Value::I32(profile.tile_rows)),
        "lag_in_frames" => match &profile.codec {
            Codec::Vp9(vp9) => Some(Value::U32(vp9.lag_in_frames)),
            _ => Some(Value::U32(profile.lag_in_frames)),
        },
        "auto_alt_ref" => match &profile.codec {
            Codec::Vp9(vp9) => Some(Value::U32(vp9.auto_alt_ref)),
            _ => Some(Value::U32(profile.auto_alt_ref)),
        },
        "aq_mode" => match &profile.codec {
            Codec::Vp9(vp9) => Some(Value::I32(vp9.aq_mode)),
            _ => Some(Value::I32(profile.aq_mode)),
        },
        "hw_global_quality" => {
            // Read from codec config (source of truth for hardware encoding)
            match &profile.codec {
                Codec::Vp9(vp9) if profile.use_hardware_encoding => {
                    Some(Value::U32(vp9.hw_global_quality))
                }
                Codec::Av1(av1) if profile.use_hardware_encoding => Some(Value::U32(av1.hw_cq)),
                _ => Some(Value::U32(profile.hw_global_quality)),
            }
        }
        "hw_rc_mode" => Some(Value::U32(profile.hw_rc_mode)),
        "hw_b_frames" => Some(Value::U32(profile.hw_b_frames)),
        "hw_compression_level" => match &profile.codec {
            Codec::Vp9(vp9) => Some(Value::U32(vp9.hw_compression_level)),
            _ => Some(Value::U32(profile.hw_compression_level)),
        },
        "hw_denoise" => match &profile.codec {
            Codec::Vp9(vp9) => Some(Value::U32(vp9.hw_denoise)),
            Codec::Av1(av1) => Some(Value::U32(av1.hw_denoise)),
        },
        "hw_detail" => match &profile.codec {
            Codec::Vp9(vp9) => Some(Value::U32(vp9.hw_detail)),
            Codec::Av1(av1) => Some(Value::U32(av1.hw_detail)),
        },
        "arnr_max_frames" => match &profile.codec {
            Codec::Vp9(vp9) => Some(Value::U32(vp9.arnr_max_frames)),
            _ => Some(Value::U32(profile.arnr_max_frames)),
        },
        "arnr_strength" => match &profile.codec {
            Codec::Vp9(vp9) => Some(Value::U32(vp9.arnr_strength)),
            _ => Some(Value::U32(profile.arnr_strength)),
        },
        "frame_parallel" => match &profile.codec {
            Codec::Vp9(vp9) => Some(Value::Bool(vp9.frame_parallel)),
            _ => Some(Value::Bool(profile.frame_parallel)),
        },
        "film_grain" => match &profile.codec {
            Codec::Av1(av1) => Some(Value::U32(av1.film_grain)),
            _ => None,
        },
        "tune" => match &profile.codec {
            Codec::Av1(av1) => Some(Value::U32(av1.tune)),
            _ => None,
        },
        "hw_loop_filter_level" => match &profile.codec {
            Codec::Vp9(vp9) if profile.use_hardware_encoding => {
                Some(Value::U32(vp9.hw_loop_filter_level))
            }
            _ => Some(Value::U32(profile.hw_loop_filter_level)),
        },
        "hw_loop_filter_sharpness" => match &profile.codec {
            Codec::Vp9(vp9) if profile.use_hardware_encoding => {
                Some(Value::U32(vp9.hw_loop_filter_sharpness))
            }
            _ => Some(Value::U32(profile.hw_loop_filter_sharpness)),
        },
        "qsv_look_ahead" => match &profile.codec {
            Codec::Vp9(vp9) => Some(Value::Bool(vp9.qsv_look_ahead)),
            _ => None,
        },
        "qsv_look_ahead_depth" => match &profile.codec {
            Codec::Vp9(vp9) => Some(Value::U32(vp9.qsv_look_ahead_depth)),
            _ => None,
        },
        "hw_lookahead" => match &profile.codec {
            Codec::Av1(av1) => Some(Value::U32(av1.hw_lookahead)),
            _ => None,
        },
        _ => {
            // Log unmapped field names for debugging
            #[cfg(feature = "dev-tools")]
            {
                use crate::engine::core::write_debug_log;
                let _ = write_debug_log(&format!(
                    "[PARAMS] Warning: extract_profile_field called for unmapped field '{}'\n",
                    field_name
                ));
            }
            None // Field not mapped
        }
    }
}

/// Apply a Value back to a Profile field
///
/// Updates both the Profile root field AND the codec config field (source of truth).
/// This ensures clamping is applied to the actual values used by command builders.
fn apply_profile_field(profile: &mut Profile, field_name: &str, value: &Value) {
    use crate::engine::core::Codec;

    match (field_name, value) {
        ("crf", Value::U32(v)) => profile.crf = *v,
        ("cpu_used", Value::U32(v)) => {
            profile.cpu_used = *v;
            // Also update codec config
            if let Codec::Vp9(vp9) = &mut profile.codec {
                vp9.cpu_used = *v;
            }
        }
        ("cpu_used_pass1", Value::U32(v)) => {
            profile.cpu_used_pass1 = *v;
            if let Codec::Vp9(vp9) = &mut profile.codec {
                vp9.cpu_used_pass1 = *v;
            }
        }
        ("cpu_used_pass2", Value::U32(v)) => {
            profile.cpu_used_pass2 = *v;
            if let Codec::Vp9(vp9) = &mut profile.codec {
                vp9.cpu_used_pass2 = *v;
            }
        }
        ("quality_mode", Value::String(v)) => {
            profile.quality_mode = v.clone();
            if let Codec::Vp9(vp9) = &mut profile.codec {
                vp9.quality_mode = v.clone();
            }
        }
        ("quality_mode", Value::Str(v)) => {
            profile.quality_mode = v.to_string();
            if let Codec::Vp9(vp9) = &mut profile.codec {
                vp9.quality_mode = v.to_string();
            }
        }
        ("row_mt", Value::Bool(v)) => {
            profile.row_mt = *v;
            if let Codec::Vp9(vp9) = &mut profile.codec {
                vp9.row_mt = *v;
            }
        }
        ("tile_columns", Value::I32(v)) => profile.tile_columns = *v,
        ("tile_rows", Value::I32(v)) => profile.tile_rows = *v,
        ("lag_in_frames", Value::U32(v)) => {
            profile.lag_in_frames = *v;
            if let Codec::Vp9(vp9) = &mut profile.codec {
                vp9.lag_in_frames = *v;
            }
        }
        ("auto_alt_ref", Value::U32(v)) => {
            profile.auto_alt_ref = *v;
            if let Codec::Vp9(vp9) = &mut profile.codec {
                vp9.auto_alt_ref = *v;
            }
        }
        ("aq_mode", Value::I32(v)) => {
            profile.aq_mode = *v;
            if let Codec::Vp9(vp9) = &mut profile.codec {
                vp9.aq_mode = *v;
            }
        }
        ("hw_global_quality", Value::U32(v)) => {
            profile.hw_global_quality = *v;
            // Update codec-specific quality field (source of truth)
            match &mut profile.codec {
                Codec::Vp9(vp9) if profile.use_hardware_encoding => vp9.hw_global_quality = *v,
                Codec::Av1(av1) if profile.use_hardware_encoding => av1.hw_cq = *v,
                _ => {}
            }
        }
        ("hw_rc_mode", Value::U32(v)) => profile.hw_rc_mode = *v,
        ("hw_b_frames", Value::U32(v)) => profile.hw_b_frames = *v,
        ("hw_compression_level", Value::U32(v)) => {
            profile.hw_compression_level = *v;
            if let Codec::Vp9(vp9) = &mut profile.codec {
                vp9.hw_compression_level = *v;
            }
        }
        ("hw_denoise", Value::U32(v)) => match &mut profile.codec {
            Codec::Vp9(vp9) => vp9.hw_denoise = *v,
            Codec::Av1(av1) => av1.hw_denoise = *v,
        },
        ("hw_detail", Value::U32(v)) => match &mut profile.codec {
            Codec::Vp9(vp9) => vp9.hw_detail = *v,
            Codec::Av1(av1) => av1.hw_detail = *v,
        },
        ("arnr_max_frames", Value::U32(v)) => {
            profile.arnr_max_frames = *v;
            if let Codec::Vp9(vp9) = &mut profile.codec {
                vp9.arnr_max_frames = *v;
            }
        }
        ("arnr_strength", Value::U32(v)) => {
            profile.arnr_strength = *v;
            if let Codec::Vp9(vp9) = &mut profile.codec {
                vp9.arnr_strength = *v;
            }
        }
        ("frame_parallel", Value::Bool(v)) => {
            profile.frame_parallel = *v;
            if let Codec::Vp9(vp9) = &mut profile.codec {
                vp9.frame_parallel = *v;
            }
        }
        ("film_grain", Value::U32(v)) => {
            if let Codec::Av1(av1) = &mut profile.codec {
                av1.film_grain = *v;
            }
        }
        ("tune", Value::U32(v)) => {
            if let Codec::Av1(av1) = &mut profile.codec {
                av1.tune = *v;
            }
        }
        ("hw_loop_filter_level", Value::U32(v)) => {
            profile.hw_loop_filter_level = *v;
            if let Codec::Vp9(vp9) = &mut profile.codec {
                vp9.hw_loop_filter_level = *v;
            }
        }
        ("hw_loop_filter_sharpness", Value::U32(v)) => {
            profile.hw_loop_filter_sharpness = *v;
            if let Codec::Vp9(vp9) = &mut profile.codec {
                vp9.hw_loop_filter_sharpness = *v;
            }
        }
        ("qsv_look_ahead", Value::Bool(v)) => {
            if let Codec::Vp9(vp9) = &mut profile.codec {
                vp9.qsv_look_ahead = *v;
            }
        }
        ("qsv_look_ahead_depth", Value::U32(v)) => {
            if let Codec::Vp9(vp9) = &mut profile.codec {
                vp9.qsv_look_ahead_depth = *v;
            }
        }
        ("hw_lookahead", Value::U32(v)) => {
            if let Codec::Av1(av1) = &mut profile.codec {
                av1.hw_lookahead = *v;
            }
        }
        _ => {
            // Log when we can't apply a value (unmapped field or type mismatch)
            #[cfg(feature = "dev-tools")]
            {
                use crate::engine::core::write_debug_log;
                let _ = write_debug_log(&format!(
                    "[PARAMS] Warning: apply_profile_field couldn't apply value (field not mapped or type mismatch)\n"
                ));
            }
        }
    }
}

/// Helper function to validate a value is within a given range
fn validate_value_in_range(
    param_name: &str,
    value: &Value,
    range: &Range,
) -> Result<(), ValidationError> {
    let valid = match (value, range) {
        (Value::U8(v), Range::Int { min, max }) => {
            let v = *v as i64;
            v >= *min && v <= *max
        }
        (Value::U16(v), Range::Int { min, max }) => {
            let v = *v as i64;
            v >= *min && v <= *max
        }
        (Value::U32(v), Range::Int { min, max }) => {
            let v = *v as i64;
            v >= *min && v <= *max
        }
        (Value::U64(v), Range::Int { min, max }) => {
            let v_i64 = (*v).try_into().unwrap_or(i64::MAX);
            v_i64 >= *min && v_i64 <= *max
        }
        (Value::I8(v), Range::Int { min, max }) => {
            let v = *v as i64;
            v >= *min && v <= *max
        }
        (Value::I16(v), Range::Int { min, max }) => {
            let v = *v as i64;
            v >= *min && v <= *max
        }
        (Value::I32(v), Range::Int { min, max }) => {
            let v = *v as i64;
            v >= *min && v <= *max
        }
        (Value::I64(v), Range::Int { min, max }) => *v >= *min && *v <= *max,
        (Value::F32(v), Range::Float { min, max }) => {
            let v = *v as f64;
            v >= *min && v <= *max
        }
        (Value::F64(v), Range::Float { min, max }) => *v >= *min && *v <= *max,
        (Value::String(v), Range::Enum { values }) => values.contains(v),
        (Value::Str(v), Range::Enum { values }) => values.iter().any(|s| s == v),
        (Value::Bool(_), Range::Bool) => true,
        (_, Range::Any) => true,
        _ => {
            return Err(ValidationError::TypeMismatch {
                param: param_name.to_string(),
                expected: format!("{:?}", range),
                actual: format!("{:?}", value),
            });
        }
    };

    if !valid {
        return Err(ValidationError::OutOfRange {
            param: param_name.to_string(),
            value: value.clone(),
            range: range.clone(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_in_range_int() {
        let value = Value::I32(50);
        let range = Range::Int { min: 0, max: 63 };
        assert!(validate_value_in_range("test", &value, &range).is_ok());

        let value = Value::I32(100);
        assert!(validate_value_in_range("test", &value, &range).is_err());
    }

    #[test]
    fn test_value_in_range_enum() {
        let value = Value::String("good".to_string());
        let range = Range::Enum {
            values: vec!["good".to_string(), "best".to_string()],
        };
        assert!(validate_value_in_range("test", &value, &range).is_ok());

        let value = Value::String("invalid".to_string());
        assert!(validate_value_in_range("test", &value, &range).is_err());
    }

    #[test]
    fn test_clamp_value_int_in_range() {
        let value = Value::U32(31);
        let range = Range::Int { min: 0, max: 63 };
        assert!(clamp_value(&value, &range).is_none());
    }

    #[test]
    fn test_clamp_value_int_above_max() {
        let value = Value::U32(100);
        let range = Range::Int { min: 0, max: 63 };
        let clamped = clamp_value(&value, &range);
        assert_eq!(clamped, Some(Value::U32(63)));
    }

    #[test]
    fn test_clamp_value_int_below_min() {
        let value = Value::I32(-5);
        let range = Range::Int { min: 0, max: 63 };
        let clamped = clamp_value(&value, &range);
        assert_eq!(clamped, Some(Value::I32(0)));
    }

    #[test]
    fn test_clamp_value_float() {
        let value = Value::F32(150.0);
        let range = Range::Float {
            min: 0.0,
            max: 100.0,
        };
        let clamped = clamp_value(&value, &range);
        assert_eq!(clamped, Some(Value::F32(100.0)));
    }

    #[test]
    fn test_clamp_value_bool_no_clamp() {
        let value = Value::Bool(true);
        let range = Range::Bool;
        assert!(clamp_value(&value, &range).is_none());
    }

    #[test]
    fn test_validate_and_clamp_profile() {
        use crate::engine::core::{Codec, Profile, Vp9Config};

        // Create a profile with out-of-range values
        let mut vp9_config = Vp9Config::default();
        vp9_config.cpu_used = 100; // Out of range (max 8)

        let mut profile = Profile {
            name: "Test".to_string(),
            video_codec: "libvpx-vp9".to_string(),
            crf: 100,      // Out of range (max 63)
            cpu_used: 100, // Out of range (will be clamped to max in registry)
            use_hardware_encoding: false,
            codec: Codec::Vp9(vp9_config),
            ..Profile::get("vp9-hq")
        };

        // Validate and clamp
        let clamps = validate_and_clamp_profile(&mut profile, "libvpx-vp9");

        // Should have clamped at least crf and cpu_used
        assert!(
            !clamps.is_empty(),
            "Should have clamped at least one parameter"
        );

        // Verify crf was clamped to 63
        assert_eq!(profile.crf, 63, "crf should be clamped to 63");

        // Verify cpu_used was clamped to 5 (libvpx-vp9 specific max, both root and codec config)
        assert_eq!(
            profile.cpu_used, 5,
            "profile.cpu_used should be clamped to 5"
        );
        if let Codec::Vp9(vp9) = &profile.codec {
            assert_eq!(vp9.cpu_used, 5, "vp9.cpu_used should be clamped to 5");
        }

        // Check that clamps were recorded
        let crf_clamp = clamps.iter().find(|c| c.param == "crf");
        assert!(crf_clamp.is_some(), "crf clamp should be recorded");

        if let Some(clamp) = crf_clamp {
            assert_eq!(clamp.original, Value::U32(100));
            assert_eq!(clamp.clamped, Value::U32(63));
        }

        let cpu_clamp = clamps.iter().find(|c| c.param == "cpu-used");
        assert!(cpu_clamp.is_some(), "cpu-used clamp should be recorded");
    }
}
