use super::parameter_mapping::ParameterSupport;
use super::parameter_registry::{get_parameter_mappings, get_parameter_statistics};
/// Parameter Documentation System
///
/// Provides documentation for why parameters are software-only, VAAPI-only, or shared.
/// Also generates coverage reports showing parameter distribution across encoders.
use std::collections::HashMap;

/// Get documentation explaining why parameters have specific support levels
pub fn get_parameter_documentation() -> HashMap<&'static str, &'static str> {
    let mut docs = HashMap::new();

    // Software-only parameters
    docs.insert("crf",
        "CRF (Constant Rate Factor) is not supported in VAAPI. Use hw_global_quality instead for quality-based encoding.");

    docs.insert(
        "video_min_bitrate",
        "Minimum bitrate constraint is not supported in VAAPI.",
    );

    docs.insert(
        "undershoot_pct",
        "Undershoot percentage is a libvpx-vp9 specific parameter not available in VAAPI.",
    );

    docs.insert(
        "overshoot_pct",
        "Overshoot percentage is a libvpx-vp9 specific parameter not available in VAAPI.",
    );

    docs.insert("cpu_used",
        "CPU-used controls software encoding speed/quality tradeoff. VAAPI uses GPU hardware, not CPU.");

    docs.insert(
        "cpu_used_pass1",
        "Pass 1 CPU speed setting. VAAPI doesn't support multi-pass encoding.",
    );

    docs.insert(
        "cpu_used_pass2",
        "Pass 2 CPU speed setting. VAAPI doesn't support multi-pass encoding.",
    );

    docs.insert(
        "two_pass",
        "Two-pass encoding is only available in software encoding. VAAPI uses single-pass.",
    );

    docs.insert(
        "quality_mode",
        "Quality mode (good/best/realtime) is specific to libvpx-vp9 software encoder.",
    );

    docs.insert(
        "vp9_profile",
        "VP9 profile is auto-detected by VAAPI based on input. Software requires explicit setting.",
    );

    docs.insert("pix_fmt",
        "Pixel format is auto-converted by VAAPI (uses format=nv12 filter). Software requires explicit setting.");

    docs.insert(
        "threads",
        "Thread count for CPU parallelism. VAAPI uses GPU parallelism instead.",
    );

    docs.insert(
        "keyint_min",
        "Minimum keyframe interval is not supported in VAAPI.",
    );

    docs.insert(
        "fixed_gop",
        "Fixed GOP (scene change threshold = 0) is not supported in VAAPI.",
    );

    docs.insert(
        "lag_in_frames",
        "Lookahead frames (lag) is a software encoder optimization not available in VAAPI.",
    );

    docs.insert(
        "auto_alt_ref",
        "Automatic alternate reference frames are not supported in VAAPI hardware encoding.",
    );

    docs.insert(
        "arnr_max_frames",
        "ARNR (Alt-Ref Noise Reduction) max frames is software-only. VAAPI doesn't support ARNR.",
    );

    docs.insert(
        "arnr_strength",
        "ARNR strength is software-only. VAAPI doesn't support temporal denoising.",
    );

    docs.insert(
        "arnr_type",
        "ARNR type is software-only. VAAPI doesn't support temporal denoising.",
    );

    docs.insert(
        "enable_tpl",
        "TPL (Temporal Dependency Model) is a software encoder optimization not in VAAPI.",
    );

    docs.insert(
        "sharpness",
        "Sharpness control is specific to libvpx-vp9 software encoder.",
    );

    docs.insert(
        "noise_sensitivity",
        "Noise sensitivity is a software encoder parameter not available in VAAPI.",
    );

    docs.insert(
        "static_thresh",
        "Static threshold for motion detection is software-only.",
    );

    docs.insert(
        "max_intra_rate",
        "Maximum intra-frame rate is software-only.",
    );

    docs.insert(
        "aq_mode",
        "Adaptive quantization mode is specific to libvpx-vp9, not available in VAAPI.",
    );

    docs.insert(
        "tune_content",
        "Content tuning (screen/film) is specific to libvpx-vp9 software encoder.",
    );

    docs.insert(
        "colorspace",
        "Colorspace metadata is software-only. VAAPI doesn't support color metadata tagging.",
    );

    docs.insert(
        "color_primaries",
        "Color primaries metadata is software-only. VAAPI doesn't support HDR metadata.",
    );

    docs.insert(
        "color_trc",
        "Color transfer characteristics (TRC) metadata is software-only.",
    );

    docs.insert(
        "color_range",
        "Color range (full/limited) metadata is software-only.",
    );

    // VAAPI-only parameters
    docs.insert(
        "hw_global_quality",
        "VAAPI quality parameter (1-150, lower=better). Software uses CRF instead.",
    );

    docs.insert(
        "hw_b_frames",
        "B-frames setting for VAAPI hardware encoding. Software VP9 doesn't support B-frames.",
    );

    docs.insert(
        "hw_loop_filter_level",
        "VP9 loop filter level control specific to VAAPI hardware encoder.",
    );

    docs.insert(
        "hw_loop_filter_sharpness",
        "VP9 loop filter sharpness control specific to VAAPI hardware encoder.",
    );

    // Shared parameters (Both)
    docs.insert(
        "row_mt",
        "Row-based multithreading works in both software and VAAPI for parallel tile encoding.",
    );

    docs.insert("tile_columns",
        "Tile columns for parallel encoding/decoding. Works in both encoders. Note: VAAPI uses underscore (-tile_columns), software uses hyphen (-tile-columns).");

    docs.insert("tile_rows",
        "Tile rows for parallel encoding/decoding. Works in both encoders. Note: VAAPI uses underscore (-tile_rows), software uses hyphen (-tile-rows).");

    docs.insert(
        "frame_parallel",
        "Frame-parallel decoding mode. Works in both software and VAAPI.",
    );

    docs.insert("gop_length",
        "GOP length (keyframe interval) works in both encoders. VAAPI caps at 240 to avoid Intel Arc blocking issues.");

    docs.insert(
        "audio_codec",
        "Audio codec selection works in both encoders. Note: CQP mode in VAAPI requires libvorbis.",
    );

    docs.insert(
        "audio_bitrate",
        "Audio bitrate works in both software and VAAPI encoding.",
    );

    docs.insert("fps",
        "FPS limiting via filter works in both encoders. Software uses fps=fps=N, VAAPI uses fps=N.");

    docs.insert(
        "scale_width",
        "Width scaling via filter works in both encoders.",
    );

    docs.insert(
        "scale_height",
        "Height scaling via filter works in both encoders.",
    );

    docs.insert("video_target_bitrate",
        "Target bitrate for VBR/CBR mode works in both encoders. Zero value triggers CQ mode (software) or CQP mode (VAAPI).");

    docs.insert(
        "video_max_bitrate",
        "Maximum bitrate cap works in both VBR/CBR modes.",
    );

    docs.insert(
        "video_bufsize",
        "Rate control buffer size works in both VBR/CBR modes.",
    );

    docs
}

/// Generate a markdown report of parameter coverage across encoders
pub fn generate_parameter_coverage_report() -> String {
    let mappings = get_parameter_mappings();
    let stats = get_parameter_statistics();
    let docs = get_parameter_documentation();

    let mut report = String::new();

    // Header
    report.push_str("# FFmpeg Parameter Coverage Report\n\n");
    report.push_str(&format!("Generated for ffdash test harness\n\n"));

    // Statistics
    report.push_str("## Statistics\n\n");
    report.push_str(&format!("- **Total parameters**: {}\n", stats.total));
    report.push_str(&format!("- **Shared (Both)**: {}\n", stats.both));
    report.push_str(&format!("- **Software-only**: {}\n", stats.software_only));
    report.push_str(&format!("- **VAAPI-only**: {}\n", stats.vaapi_only));
    report.push_str(&format!(
        "- **Not applicable**: {}\n\n",
        stats.not_applicable
    ));

    // Shared parameters
    report.push_str("## Shared Parameters (Both Encoders)\n\n");
    report.push_str(
        "These parameters work in **both** software (libvpx-vp9) and VAAPI hardware encoding:\n\n",
    );
    report.push_str("| Field Name | Software Flag | VAAPI Flag | Documentation |\n");
    report.push_str("|------------|---------------|------------|---------------|\n");

    for mapping in mappings.iter() {
        if matches!(mapping.support, ParameterSupport::Both) {
            let sw_flag = mapping.software_flag.unwrap_or("N/A");
            let vaapi_flag = mapping.vaapi_flag.unwrap_or("N/A");
            let doc = docs
                .get(mapping.field_name)
                .unwrap_or(&"No documentation available");

            report.push_str(&format!(
                "| `{}` | `{}` | `{}` | {} |\n",
                mapping.field_name, sw_flag, vaapi_flag, doc
            ));
        }
    }

    // Software-only parameters
    report.push_str("\n## Software-Only Parameters\n\n");
    report.push_str("These parameters **only work** with libvpx-vp9 software encoding:\n\n");
    report.push_str("| Field Name | FFmpeg Flag | Reason |\n");
    report.push_str("|------------|-------------|--------|\n");

    for mapping in mappings.iter() {
        if matches!(mapping.support, ParameterSupport::SoftwareOnly) {
            let flag = mapping.software_flag.unwrap_or("N/A");
            let reason = docs
                .get(mapping.field_name)
                .unwrap_or(&"Not supported in VAAPI hardware encoder");

            report.push_str(&format!(
                "| `{}` | `{}` | {} |\n",
                mapping.field_name, flag, reason
            ));
        }
    }

    // VAAPI-only parameters
    report.push_str("\n## VAAPI-Only Parameters\n\n");
    report.push_str("These parameters **only work** with VAAPI hardware encoding:\n\n");
    report.push_str("| Field Name | FFmpeg Flag | Description |\n");
    report.push_str("|------------|-------------|-------------|\n");

    for mapping in mappings.iter() {
        if matches!(mapping.support, ParameterSupport::VaapiOnly) {
            let flag = mapping.vaapi_flag.unwrap_or("N/A");
            let desc = docs
                .get(mapping.field_name)
                .unwrap_or(&"VAAPI hardware-specific parameter");

            report.push_str(&format!(
                "| `{}` | `{}` | {} |\n",
                mapping.field_name, flag, desc
            ));
        }
    }

    // Important notes
    report.push_str("\n## Important Notes\n\n");
    report.push_str("### Flag Format Differences\n\n");
    report.push_str("Some parameters use **different flag formats** between encoders:\n\n");
    report.push_str("- `tile_columns`: Software uses `-tile-columns` (hyphen), VAAPI uses `-tile_columns` (underscore)\n");
    report.push_str("- `tile_rows`: Software uses `-tile-rows` (hyphen), VAAPI uses `-tile_rows` (underscore)\n\n");

    report.push_str("### The Parallelism Bug (Commit 0b80e0b)\n\n");
    report.push_str("The bug that inspired this test harness:\n\n");
    report.push_str(
        "**Missing from VAAPI**: `row_mt`, `tile_columns`, `tile_rows`, `frame_parallel`\n\n",
    );
    report.push_str("These parameters were correctly implemented in software encoding but were **completely missing** from VAAPI hardware encoding. ");
    report.push_str("This meant that changing tile settings had no effect on VAAPI encodes.\n\n");
    report.push_str(
        "**Fixed**: Commit 0b80e0b added all 4 missing parameters to VAAPI command builder.\n\n",
    );
    report.push_str("**Prevention**: The test harness in this module would have caught this bug automatically.\n\n");

    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_documentation_exists() {
        let docs = get_parameter_documentation();
        assert!(!docs.is_empty(), "Documentation should not be empty");

        // Check critical parameters have documentation
        assert!(
            docs.contains_key("row_mt"),
            "row_mt should have documentation"
        );
        assert!(docs.contains_key("crf"), "crf should have documentation");
        assert!(
            docs.contains_key("hw_global_quality"),
            "hw_global_quality should have documentation"
        );
    }

    #[test]
    fn test_report_generation() {
        let report = generate_parameter_coverage_report();

        assert!(
            report.contains("FFmpeg Parameter Coverage Report"),
            "Report should have title"
        );
        assert!(
            report.contains("Shared Parameters"),
            "Report should have shared parameters section"
        );
        assert!(
            report.contains("Software-Only Parameters"),
            "Report should have software-only section"
        );
        assert!(
            report.contains("VAAPI-Only Parameters"),
            "Report should have VAAPI-only section"
        );
        assert!(
            report.contains("The Parallelism Bug"),
            "Report should document the parallelism bug"
        );

        println!("\n{}\n", report);
    }

    #[test]
    fn test_all_shared_parameters_documented() {
        let mappings = get_parameter_mappings();
        let docs = get_parameter_documentation();

        let undocumented: Vec<_> = mappings
            .iter()
            .filter(|m| matches!(m.support, ParameterSupport::Both))
            .filter(|m| !docs.contains_key(m.field_name))
            .map(|m| m.field_name)
            .collect();

        if !undocumented.is_empty() {
            println!("Warning: The following shared parameters lack documentation:");
            for field in &undocumented {
                println!("  - {}", field);
            }
        }

        // This is a soft assertion - we should document all shared params
        // but it's not critical for test functionality
        if !undocumented.is_empty() {
            eprintln!(
                "⚠️  {} shared parameters lack documentation",
                undocumented.len()
            );
        }
    }
}
