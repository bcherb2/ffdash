/// Smoke tests for encoder parameter registry
///
/// These tests verify that the TOML parameter ranges match actual FFmpeg behavior
/// by attempting to encode with boundary values and invalid values.
///
/// Run with: cargo test --features dev-tools --test params_smoke_tests -- --test-threads=1
///
/// Note: These tests are SLOW (they actually encode video) and require FFmpeg installed.

#[cfg(feature = "dev-tools")]
mod tests {
    use ffdash::engine::params::{Range, params_for_encoder};
    use std::path::Path;
    use std::process::Command;
    use tempfile::TempDir;

    /// Generate a minimal test video file (1 second, 64x64, solid color)
    fn create_test_video(output_path: &Path) -> Result<(), String> {
        let status = Command::new("ffmpeg")
            .args(&[
                "-f",
                "lavfi",
                "-i",
                "color=c=black:s=64x64:d=1",
                "-pix_fmt",
                "yuv420p",
                "-y",
                output_path.to_str().unwrap(),
            ])
            .output()
            .map_err(|e| format!("Failed to run ffmpeg: {}", e))?;

        if !status.status.success() {
            return Err(format!(
                "Failed to create test video: {}",
                String::from_utf8_lossy(&status.stderr)
            ));
        }

        Ok(())
    }

    /// Test encoding with specific parameters
    fn test_encode(
        encoder: &str,
        params: &[(&str, &str)],
        input: &Path,
        output: &Path,
    ) -> Result<(), String> {
        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-i").arg(input);
        cmd.arg("-c:v").arg(encoder);

        // Add test parameters
        for (flag, value) in params {
            cmd.arg(flag).arg(value);
        }

        cmd.arg("-frames:v").arg("10"); // Only encode 10 frames for speed
        cmd.arg("-y");
        cmd.arg(output);

        let output = cmd
            .output()
            .map_err(|e| format!("Failed to execute ffmpeg: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("FFmpeg failed: {}", stderr));
        }

        Ok(())
    }

    #[test]
    #[ignore] // Run explicitly: cargo test --features dev-tools boundary_value_tests -- --ignored
    fn boundary_value_tests_software_encoders() {
        // Test software encoders with boundary values
        let software_encoders = vec!["libvpx-vp9", "libsvtav1", "libaom-av1"];

        for encoder_id in software_encoders {
            println!("\n=== Testing {} ===", encoder_id);

            // Skip if encoder not available
            let check = Command::new("ffmpeg")
                .args(&["-encoders"])
                .output()
                .unwrap();
            let encoders_list = String::from_utf8_lossy(&check.stdout);
            if !encoders_list.contains(encoder_id) {
                println!("⚠️  Encoder {} not available, skipping", encoder_id);
                continue;
            }

            let temp_dir = TempDir::new().unwrap();
            let input_path = temp_dir.path().join("input.mp4");
            let output_path = temp_dir.path().join("output.mkv");

            // Create test input
            if let Err(e) = create_test_video(&input_path) {
                panic!("Failed to create test video: {}", e);
            }

            // Test each parameter's boundary values
            for param in params_for_encoder(encoder_id) {
                match &param.range {
                    Range::Int { min, max } => {
                        let config = param
                            .get_encoder_config(encoder_id)
                            .expect("Should have config");

                        if !config.supported {
                            continue;
                        }

                        let flag = config.flag.expect("Should have flag");

                        println!("  Testing {}: [{}, {}]", param.name, min, max);

                        // Test minimum value
                        let result = test_encode(
                            encoder_id,
                            &[(flag, &min.to_string())],
                            &input_path,
                            &output_path,
                        );

                        assert!(
                            result.is_ok(),
                            "Encoder {} should accept {}={} (documented min)",
                            encoder_id,
                            param.name,
                            min
                        );

                        // Test maximum value
                        let result = test_encode(
                            encoder_id,
                            &[(flag, &max.to_string())],
                            &input_path,
                            &output_path,
                        );

                        assert!(
                            result.is_ok(),
                            "Encoder {} should accept {}={} (documented max)",
                            encoder_id,
                            param.name,
                            max
                        );

                        // Test just below minimum (should fail)
                        if *min > i64::MIN {
                            let below_min = min - 1;
                            let result = test_encode(
                                encoder_id,
                                &[(flag, &below_min.to_string())],
                                &input_path,
                                &output_path,
                            );

                            if result.is_ok() {
                                println!(
                                    "⚠️  WARNING: {} accepted {}={} (below documented min {})",
                                    encoder_id, param.name, below_min, min
                                );
                            }
                        }

                        // Test just above maximum (should fail)
                        if *max < i64::MAX {
                            let above_max = max + 1;
                            let result = test_encode(
                                encoder_id,
                                &[(flag, &above_max.to_string())],
                                &input_path,
                                &output_path,
                            );

                            if result.is_ok() {
                                println!(
                                    "⚠️  WARNING: {} accepted {}={} (above documented max {})",
                                    encoder_id, param.name, above_max, max
                                );
                            }
                        }
                    }
                    Range::Enum { values } => {
                        println!("  Testing {}: {:?}", param.name, values);

                        let config = param
                            .get_encoder_config(encoder_id)
                            .expect("Should have config");

                        if !config.supported {
                            continue;
                        }

                        let flag = config.flag.expect("Should have flag");

                        // Test each valid enum value
                        for value in values {
                            let result = test_encode(
                                encoder_id,
                                &[(flag, value)],
                                &input_path,
                                &output_path,
                            );

                            assert!(
                                result.is_ok(),
                                "Encoder {} should accept {}={}",
                                encoder_id,
                                param.name,
                                value
                            );
                        }

                        // Test invalid enum value
                        let result = test_encode(
                            encoder_id,
                            &[(flag, "invalid_value_xyz")],
                            &input_path,
                            &output_path,
                        );

                        assert!(
                            result.is_err(),
                            "Encoder {} should reject invalid enum value for {}",
                            encoder_id,
                            param.name
                        );
                    }
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn verify_ffmpeg_available() {
        // Basic sanity check that FFmpeg is available
        let result = Command::new("ffmpeg").arg("-version").output();

        assert!(
            result.is_ok(),
            "FFmpeg must be installed to run smoke tests"
        );

        let output = result.unwrap();
        let version = String::from_utf8_lossy(&output.stdout);

        println!(
            "FFmpeg version: {}",
            version.lines().next().unwrap_or("unknown")
        );

        assert!(
            version.contains("ffmpeg version"),
            "FFmpeg version output unexpected"
        );
    }

    #[test]
    fn verify_toml_has_verification_metadata() {
        // Verify that parameters have verification metadata
        use std::fs;

        let toml_content = fs::read_to_string("src/engine/params/encoder-params.toml")
            .expect("Should be able to read TOML");

        // Check that verification fields exist
        assert!(
            toml_content.contains("verification"),
            "TOML should contain verification metadata"
        );

        // Check metadata section
        assert!(toml_content.contains("schema_version"));
        assert!(toml_content.contains("ffmpeg_version"));
        assert!(toml_content.contains("last_verified"));
    }

    #[test]
    fn check_parameter_coverage_vs_profile_fields() {
        // This test documents which Profile fields are NOT in the registry
        // (by design - they're global settings)

        let covered_params: Vec<&str> = ffdash::engine::params::PARAMS
            .iter()
            .map(|p| p.name)
            .collect();

        println!("\nCovered parameters ({} total):", covered_params.len());
        for param in &covered_params {
            println!("  ✅ {}", param);
        }

        // Known intentionally excluded fields (from COVERAGE.md)
        let intentionally_excluded = vec![
            "video_target_bitrate", // Global bitrate mode
            "video_min_bitrate",
            "video_max_bitrate",
            "gop_length", // Global GOP settings
            "keyint_min",
            "threads",     // Global FFmpeg option
            "passes",      // Workflow decision
            "output_dir",  // Application concern
            "video_codec", // Encoder selection
        ];

        println!(
            "\nIntentionally excluded ({} total):",
            intentionally_excluded.len()
        );
        for field in &intentionally_excluded {
            println!("  ⊝ {} (handled by Profile/application logic)", field);
        }
    }
}
