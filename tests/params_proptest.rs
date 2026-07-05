/// Property-based tests for parameter validation
///
/// Uses proptest to generate random valid and invalid parameter combinations
/// and verify that validation behaves correctly.
///
/// Run with: cargo test --features dev-tools --test params_proptest

#[cfg(feature = "dev-tools")]
mod tests {
    use ffdash::engine::params::{
        ENCODERS, Range, Value, get_param, params_for_encoder, validate_profile,
    };
    use proptest::prelude::*;
    use std::collections::HashMap;

    /// Generate a valid value within a parameter's range
    fn value_in_range(range: &Range) -> BoxedStrategy<Value> {
        match range {
            Range::Int { min, max } => {
                // Clamp to i32 range for simplicity
                let min = (*min).max(i32::MIN as i64) as i32;
                let max = (*max).min(i32::MAX as i64) as i32;
                (min..=max).prop_map(Value::I32).boxed()
            }
            Range::Float { min, max } => {
                let min = (*min) as f32;
                let max = (*max) as f32;
                (min..=max).prop_map(Value::F32).boxed()
            }
            Range::Enum { values } => {
                // For enum, just use the first value as a simple test
                if !values.is_empty() {
                    Just(Value::String(values[0].clone())).boxed()
                } else {
                    Just(Value::String("".to_string())).boxed()
                }
            }
            Range::Bool => prop::bool::ANY.prop_map(Value::Bool).boxed(),
            Range::Any => Just(Value::I32(0)).boxed(), // Default for Any
        }
    }

    #[test]
    fn test_valid_values_for_all_encoders() {
        // Simpler version without proptest - just test mid-range values
        for encoder in ENCODERS.iter() {
            let mut params = HashMap::new();

            for param in params_for_encoder(encoder.id).take(3) {
                let value = match &param.range {
                    Range::Int { min, max } => {
                        let mid = (min + max) / 2;
                        Value::I32(mid as i32)
                    }
                    Range::Float { min, max } => {
                        let mid = (min + max) / 2.0;
                        Value::F32(mid as f32)
                    }
                    Range::Enum { values } => {
                        if !values.is_empty() {
                            Value::String(values[0].clone())
                        } else {
                            continue;
                        }
                    }
                    Range::Bool => Value::Bool(true),
                    Range::Any => Value::I32(0),
                };
                params.insert(param.name.to_string(), value);
            }

            if !params.is_empty() {
                let result = validate_profile(encoder.id, &params);
                assert!(
                    result.is_ok(),
                    "Valid values should pass validation for encoder {}: {:?}",
                    encoder.id,
                    result
                );
            }
        }
    }

    proptest! {
        #[test]
        fn out_of_range_int_values_always_fail(
            value in i32::MIN..i32::MAX,
        ) {
            // Get crf parameter (known to have range 0-63 for most encoders)
            let param = get_param("crf");
            if param.is_none() {
                return Ok(());
            }

            let param = param.unwrap();
            if let Range::Int { min, max } = &param.range {
                let min = *min as i32;
                let max = *max as i32;

                // If value is outside range
                if value < min || value > max {
                    let mut params = HashMap::new();
                    params.insert("crf".to_string(), Value::I32(value));

                    // Find an encoder that supports crf
                    for encoder in ENCODERS.iter() {
                        if param.is_supported_by(encoder.id) {
                            let result = validate_profile(encoder.id, &params);
                            prop_assert!(result.is_err(),
                                "Out-of-range value {} should fail for encoder {} (range: {}-{})",
                                value, encoder.id, min, max);
                            break;
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_unknown_parameter_names_fail() {
        // Test a few known-bad parameter names
        let bad_names = vec!["nonexistent", "invalid_param", "fake_setting"];

        for bad_name in bad_names {
            let mut params = HashMap::new();
            params.insert(bad_name.to_string(), Value::I32(0));

            let result = validate_profile("libvpx-vp9", &params);
            assert!(
                result.is_err(),
                "Unknown parameter '{}' should fail validation",
                bad_name
            );
        }
    }

    #[test]
    fn test_parameter_range_coverage() {
        // Test that all parameter ranges are testable
        for param in ffdash::engine::params::PARAMS.iter() {
            match &param.range {
                Range::Int { min, max } => {
                    assert!(
                        min <= max,
                        "Parameter '{}' has invalid range: {} > {}",
                        param.name,
                        min,
                        max
                    );
                }
                Range::Float { min, max } => {
                    assert!(
                        min <= max,
                        "Parameter '{}' has invalid float range: {} > {}",
                        param.name,
                        min,
                        max
                    );
                }
                Range::Enum { values } => {
                    assert!(
                        !values.is_empty(),
                        "Parameter '{}' has empty enum values",
                        param.name
                    );
                }
                Range::Bool | Range::Any => {}
            }
        }
    }

    #[test]
    fn test_all_encoders_have_testable_params() {
        // Verify that every encoder has at least one parameter we can test
        for encoder in ENCODERS.iter() {
            let supported_params: Vec<_> = params_for_encoder(encoder.id).collect();

            assert!(
                !supported_params.is_empty(),
                "Encoder '{}' should have at least one testable parameter",
                encoder.id
            );

            println!(
                "Encoder '{}' has {} testable parameters",
                encoder.id,
                supported_params.len()
            );
        }
    }

    #[test]
    fn test_enum_validation_strictness() {
        // Test that enum validation is strict (rejects invalid values)
        for param in ffdash::engine::params::PARAMS.iter() {
            if let Range::Enum { values } = &param.range {
                // Find an encoder that supports this param
                for encoder in ENCODERS.iter() {
                    if !param.is_supported_by(encoder.id) {
                        continue;
                    }

                    // Test with invalid enum value
                    let mut params = HashMap::new();
                    params.insert(param.name.to_string(), Value::Str("invalid_enum_value_xyz"));

                    let result = validate_profile(encoder.id, &params);
                    assert!(
                        result.is_err(),
                        "Parameter '{}' should reject invalid enum value for encoder '{}'",
                        param.name,
                        encoder.id
                    );
                    break; // Only test one encoder per param
                }
            }
        }
    }

    #[test]
    fn test_cross_encoder_parameter_isolation() {
        // Test that parameters for one encoder don't leak to another
        let libvpx_params: Vec<_> = params_for_encoder("libvpx-vp9").map(|p| p.name).collect();
        let vaapi_params: Vec<_> = params_for_encoder("vp9_vaapi").map(|p| p.name).collect();

        println!("libvpx-vp9 has {} params", libvpx_params.len());
        println!("vp9_vaapi has {} params", vaapi_params.len());

        // Software encoder should have crf
        assert!(
            libvpx_params.contains(&"crf"),
            "libvpx-vp9 should support crf"
        );

        // Hardware encoder should NOT have crf
        assert!(
            !vaapi_params.contains(&"crf"),
            "vp9_vaapi should NOT support crf (uses global_quality instead)"
        );

        // Hardware encoder should have global_quality
        assert!(
            vaapi_params.contains(&"global_quality"),
            "vp9_vaapi should support global_quality"
        );
    }

    #[test]
    fn test_validation_error_messages() {
        // Test that validation errors provide useful information
        use ffdash::engine::params::ValidationError;

        // Test out of range error
        let mut params = HashMap::new();
        params.insert("crf".to_string(), Value::I32(999));

        match validate_profile("libvpx-vp9", &params) {
            Err(ValidationError::OutOfRange { param, value, .. }) => {
                assert_eq!(param, "crf");
                assert!(matches!(value, Value::I32(999)));
            }
            _ => panic!("Expected OutOfRange error"),
        }

        // Test unknown parameter error
        let mut params = HashMap::new();
        params.insert("nonexistent".to_string(), Value::I32(0));

        match validate_profile("libvpx-vp9", &params) {
            Err(ValidationError::UnknownParameter(param)) => {
                assert_eq!(param, "nonexistent");
            }
            _ => panic!("Expected UnknownParameter error"),
        }

        // Test unsupported by encoder error
        let mut params = HashMap::new();
        params.insert("crf".to_string(), Value::I32(31));

        match validate_profile("vp9_vaapi", &params) {
            Err(ValidationError::UnsupportedByEncoder(param, encoder)) => {
                assert_eq!(param, "crf");
                assert_eq!(encoder, "vp9_vaapi");
            }
            _ => panic!("Expected UnsupportedByEncoder error"),
        }
    }
}
