//! Schema-driven validation for profiles before building FFmpeg commands.

use crate::engine::core::Profile;
use crate::engine::hardware::{
    check_av1_nvenc_available, check_av1_qsv_available, check_av1_vaapi_available,
    check_libsvtav1_available, check_vp9_qsv_available, check_vp9_vaapi_available,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub encoder: String,
}

#[derive(Debug, Clone, Copy)]
pub struct HardwareAvailability {
    pub vp9_qsv: bool,
    pub vp9_vaapi: bool,
    pub av1_qsv: bool,
    pub av1_nvenc: bool,
    pub av1_vaapi: bool,
    pub av1_svt: bool,
}

impl Default for HardwareAvailability {
    fn default() -> Self {
        Self {
            vp9_qsv: check_vp9_qsv_available(),
            vp9_vaapi: check_vp9_vaapi_available(),
            av1_qsv: check_av1_qsv_available(),
            av1_nvenc: check_av1_nvenc_available(),
            av1_vaapi: check_av1_vaapi_available(),
            av1_svt: check_libsvtav1_available(),
        }
    }
}

/// Validate a profile against encoder-specific rules.
pub fn validate_profile(
    profile: &Profile,
    hw: HardwareAvailability,
) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    // Hardware availability vs requested encoder
    let encoder = profile.video_codec.clone();
    if profile.use_hardware_encoding {
        match encoder.as_str() {
            "vp9_qsv" if !hw.vp9_qsv => errors.push(err(
                "use_hardware_encoding",
                "VP9 QSV not available",
                &encoder,
            )),
            "vp9_vaapi" if !hw.vp9_vaapi => errors.push(err(
                "use_hardware_encoding",
                "VP9 VAAPI not available",
                &encoder,
            )),
            "av1_qsv" if !hw.av1_qsv => errors.push(err(
                "use_hardware_encoding",
                "AV1 QSV not available",
                &encoder,
            )),
            "av1_nvenc" if !hw.av1_nvenc => errors.push(err(
                "use_hardware_encoding",
                "AV1 NVENC not available",
                &encoder,
            )),
            "av1_vaapi" if !hw.av1_vaapi => errors.push(err(
                "use_hardware_encoding",
                "AV1 VAAPI not available",
                &encoder,
            )),
            _ => {}
        }
    }

    // Rate control constraints
    if (encoder == "av1_qsv" || encoder == "av1_nvenc") && profile.crf > 0 {
        errors.push(err(
            "crf",
            "CRF not supported for AV1 hardware; use hw_cq/global_quality",
            &encoder,
        ));
    }

    // Preset ranges
    if encoder == "av1_qsv" {
        if let Some(cfg) = profile.codec.as_av1() {
            let p = cfg.hw_preset.trim_start_matches('p');
            if let Ok(num) = p.parse::<u32>() {
                if !(1..=7).contains(&num) {
                    errors.push(err("av1_hw_preset", "QSV preset must be 1-7", &encoder));
                }
            }
        }
    }
    if encoder == "av1_nvenc" {
        if let Some(cfg) = profile.codec.as_av1() {
            let p = cfg.hw_preset.trim_start_matches('p');
            if let Ok(num) = p.parse::<u32>() {
                if !(1..=7).contains(&num) {
                    errors.push(err("av1_hw_preset", "NVENC preset must be p1-p7", &encoder));
                }
            }
        }
    }

    // GOP caps for hardware encoders
    if matches!(encoder.as_str(), "av1_qsv" | "av1_nvenc" | "av1_vaapi") {
        if let Ok(gop) = profile.gop_length.parse::<u32>() {
            if gop > 300 {
                errors.push(err(
                    "gop_length",
                    "GOP too large for hardware (max 300)",
                    &encoder,
                ));
            }
        }
    }

    // VAAPI global quality bounds
    if encoder.ends_with("_vaapi") && profile.hw_global_quality > 255 {
        errors.push(err(
            "hw_global_quality",
            "VAAPI global_quality must be 1-255",
            &encoder,
        ));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn err(field: &str, message: &str, encoder: &str) -> ValidationError {
    ValidationError {
        field: field.to_string(),
        message: message.to_string(),
        encoder: encoder.to_string(),
    }
}
