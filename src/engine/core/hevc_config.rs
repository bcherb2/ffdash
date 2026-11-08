//! HEVC-specific encoding configuration.
//!
//! `HevcConfig` holds the settings of every HEVC Encoder: libx265 (the Software
//! Fallback) and the hardware Encoders (`hevc_nvenc`, `hevc_qsv`, `hevc_vaapi`,
//! `hevc_amf`, `hevc_videotoolbox`).
//!
//! Layout: shared software (x265) settings, shared hardware settings, then one
//! quality field per Encoder on that Encoder's own native scale (ADR 0002: the
//! scales differ, so there is deliberately no common "hardware quality" field).

use serde::{Deserialize, Serialize};

fn default_x265_preset() -> String {
    "medium".to_string()
}
fn default_x265_tune() -> String {
    "none".to_string()
}
fn default_hw_preset() -> String {
    "4".to_string() // medium (QSV "1"-"7", NVENC "p1"-"p7"; each Encoder maps it)
}
fn default_hw_rc_mode() -> String {
    "cq".to_string() // constant quality: the only mode ffdash uses (ADR 0001)
}
fn default_hw_lookahead() -> u32 {
    40
}
fn default_hw_b_frames() -> u32 {
    2
}

// Per-Encoder quality defaults, each on the Encoder's own scale.
fn default_x265_crf() -> u32 {
    28 // libx265 CRF: 0-51, lower=better
}
fn default_qsv_quality() -> u32 {
    26 // hevc_qsv `-q:v` (CQP): 1-51, lower=better
}
fn default_nvenc_quality() -> u32 {
    28 // hevc_nvenc `-cq`: 0-51, lower=better
}
fn default_vaapi_quality() -> u32 {
    26 // hevc_vaapi QP: 1-51, lower=better
}
fn default_amf_qp() -> u32 {
    26 // hevc_amf `-qp_i/-qp_p`: 0-51, lower=better
}
fn default_videotoolbox_q() -> u32 {
    60 // hevc_videotoolbox `-q:v`: 1-100, HIGHER=better
}

/// HEVC encoding settings (libx265 and the hardware Encoders).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HevcConfig {
    // ---- Software (libx265) settings ---------------------------------------
    #[serde(default = "default_x265_preset")]
    pub x265_preset: String, // ultrafast .. veryslow

    #[serde(default = "default_x265_tune")]
    pub x265_tune: String, // none, psnr, ssim, grain, zerolatency, fastdecode, animation

    // ---- Shared hardware settings ------------------------------------------
    #[serde(default = "default_hw_preset")]
    pub hw_preset: String,

    #[serde(default = "default_hw_lookahead")]
    pub hw_lookahead: u32, // lookahead depth in frames, 0 = off

    #[serde(default = "default_hw_b_frames")]
    pub hw_b_frames: u32,

    #[serde(default = "default_hw_rc_mode")]
    pub hw_rc_mode: String, // "cq" (constant quality); each Encoder maps it to its own flags

    // ---- Per-Encoder quality, on each Encoder's native scale ----------------
    #[serde(default = "default_x265_crf")]
    pub x265_crf: u32, // libx265: 0-51, lower=better

    #[serde(default = "default_qsv_quality")]
    pub qsv_quality: u32, // hevc_qsv: 1-51, lower=better

    #[serde(default = "default_nvenc_quality")]
    pub nvenc_quality: u32, // hevc_nvenc: 0-51, lower=better

    #[serde(default = "default_vaapi_quality")]
    pub vaapi_quality: u32, // hevc_vaapi: 1-51, lower=better

    #[serde(default = "default_amf_qp")]
    pub amf_qp: u32, // hevc_amf: 0-51, lower=better

    #[serde(default = "default_videotoolbox_q")]
    pub videotoolbox_q: u32, // hevc_videotoolbox: 1-100, higher=better
}

impl Default for HevcConfig {
    fn default() -> Self {
        Self {
            x265_preset: default_x265_preset(),
            x265_tune: default_x265_tune(),
            hw_preset: default_hw_preset(),
            hw_lookahead: default_hw_lookahead(),
            hw_b_frames: default_hw_b_frames(),
            hw_rc_mode: default_hw_rc_mode(),
            x265_crf: default_x265_crf(),
            qsv_quality: default_qsv_quality(),
            nvenc_quality: default_nvenc_quality(),
            vaapi_quality: default_vaapi_quality(),
            amf_qp: default_amf_qp(),
            videotoolbox_q: default_videotoolbox_q(),
        }
    }
}
