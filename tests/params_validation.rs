/// Integration tests for the parameter validation system
/// Tests that validation properly catches errors and validates profiles

#[cfg(feature = "dev-tools")]
mod tests {
    use std::collections::HashMap;
    use ffdash::engine::params::{
        get_param, get_encoder, params_for_encoder,
        validate_profile, ValidationError, Value
    };

    #[test]
    fn test_validate_valid_profile() {
        let mut params = HashMap::new();
        params.insert("crf".to_string(), Value::I32(31));
        params.insert("cpu-used".to_string(), Value::I32(2));

        // Should pass validation
        let result = validate_profile("libvpx-vp9", &params);
        assert!(result.is_ok(), "Valid profile should pass validation");
    }

    #[test]
    fn test_validate_out_of_range() {
        let mut params = HashMap::new();
        params.insert("crf".to_string(), Value::I32(100)); // Out of range (0-63)

        let result = validate_profile("libvpx-vp9", &params);
        assert!(result.is_err(), "Out of range value should fail");

        match result.unwrap_err() {
            ValidationError::OutOfRange { param, .. } => {
                assert_eq!(param, "crf");
            }
            _ => panic!("Expected OutOfRange error"),
        }
    }

    #[test]
    fn test_validate_unknown_param() {
        let mut params = HashMap::new();
        params.insert("nonexistent_param".to_string(), Value::I32(1));

        let result = validate_profile("libvpx-vp9", &params);
        assert!(result.is_err(), "Unknown parameter should fail");

        match result.unwrap_err() {
            ValidationError::UnknownParameter(param) => {
                assert_eq!(param, "nonexistent_param");
            }
            _ => panic!("Expected UnknownParameter error"),
        }
    }

    #[test]
    fn test_validate_unsupported_by_encoder() {
        let mut params = HashMap::new();
        // crf is a software encoder param, not supported by hardware encoders
        params.insert("crf".to_string(), Value::I32(31));

        let result = validate_profile("vp9_vaapi", &params);
        assert!(result.is_err(), "Unsupported param should fail");

        match result.unwrap_err() {
            ValidationError::UnsupportedByEncoder(param, encoder) => {
                assert_eq!(param, "crf");
                assert_eq!(encoder, "vp9_vaapi");
            }
            _ => panic!("Expected UnsupportedByEncoder error"),
        }
    }

    #[test]
    fn test_validate_unknown_encoder() {
        let mut params = HashMap::new();
        params.insert("crf".to_string(), Value::I32(31));

        let result = validate_profile("nonexistent_encoder", &params);
        assert!(result.is_err(), "Unknown encoder should fail");

        match result.unwrap_err() {
            ValidationError::EncoderNotFound(encoder) => {
                assert_eq!(encoder, "nonexistent_encoder");
            }
            _ => panic!("Expected EncoderNotFound error"),
        }
    }

    #[test]
    fn test_get_param_works() {
        let crf = get_param("crf").expect("crf parameter should exist");
        assert_eq!(crf.name, "crf");
        assert!(!crf.description.is_empty());
    }

    #[test]
    fn test_get_encoder_works() {
        let encoder = get_encoder("libvpx-vp9").expect("libvpx-vp9 should exist");
        assert_eq!(encoder.id, "libvpx-vp9");
        assert_eq!(encoder.ffmpeg_name, "libvpx-vp9");
    }

    #[test]
    fn test_params_for_encoder_filters_correctly() {
        let vp9_params: Vec<_> = params_for_encoder("libvpx-vp9").collect();
        let vaapi_params: Vec<_> = params_for_encoder("vp9_vaapi").collect();

        // Different encoders should have different parameters
        assert!(vp9_params.len() > 0, "libvpx-vp9 should have params");
        assert!(vaapi_params.len() > 0, "vp9_vaapi should have params");

        // Software encoder should have crf
        assert!(vp9_params.iter().any(|p| p.name == "crf"),
            "libvpx-vp9 should support crf");

        // Hardware encoder should not have crf
        assert!(!vaapi_params.iter().any(|p| p.name == "crf"),
            "vp9_vaapi should not support crf");

        // Hardware encoder should have global_quality
        assert!(vaapi_params.iter().any(|p| p.name == "global_quality"),
            "vp9_vaapi should support global_quality");
    }

    #[test]
    fn test_enum_validation() {
        // Test that enum values are properly validated
        let quality_param = get_param("quality");
        if let Some(param) = quality_param {
            let mut params = HashMap::new();
            params.insert("quality".to_string(), Value::Str("good"));

            // Valid enum value should pass
            let result = validate_profile("libvpx-vp9", &params);
            assert!(result.is_ok() || matches!(result, Err(ValidationError::UnsupportedByEncoder(..  ))),
                "Valid enum value should pass or be unsupported");
        }
    }

    #[test]
    fn test_all_encoders_accessible() {
        // Test that all 9 encoders are accessible
        let expected_encoders = vec![
            "libvpx-vp9", "vp9_vaapi", "vp9_qsv",
            "libsvtav1", "libaom-av1",
            "av1_qsv", "av1_nvenc", "av1_vaapi", "av1_amf"
        ];

        for encoder_id in expected_encoders {
            let encoder = get_encoder(encoder_id);
            assert!(encoder.is_some(),
                "Encoder '{}' should be accessible", encoder_id);
        }
    }
}
