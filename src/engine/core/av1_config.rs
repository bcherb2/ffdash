//! AV1-specific encoding configuration.
//!
//! Contains the `Av1Config` struct for AV1 codec settings used by
//! libsvtav1 (software), av1_qsv (Intel QSV), av1_nvenc (NVIDIA), and av1_vaapi (VAAPI).
//! Also contains the `Codec` enum for codec selection.

use serde::{Deserialize, Serialize};

use super::vp9_config::Vp9Config;

// Default functions for Av1Config
fn default_av1_preset() -> u32 {
    8
}
fn default_av1_scm() -> u32 {
    2 // auto
}
fn default_av1_hw_preset() -> String {
    "4".to_string() // medium
}
fn default_av1_hw_cq() -> u32 {
    30
}

// Per-encoder quality defaults (calibrated for balanced quality)
fn default_svt_crf() -> u32 {
    28 // SVT-AV1 CRF: 0-63, lower=better
}
fn default_qsv_cq() -> u32 {
    65 // Intel QSV: 1-255, lower=better
}
fn default_nvenc_cq() -> u32 {
    16 // NVIDIA: 0-63, lower=better (65/255*63 ≈ 16)
}
fn default_vaapi_cq() -> u32 {
    65 // VAAPI: 1-255, lower=better
}
fn default_av1_hw_lookahead() -> u32 {
    40
}
fn default_true() -> bool {
    true
}

/// AV1-specific encoding settings (for libsvtav1, av1_qsv, av1_nvenc, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Av1Config {
    // Software (libsvtav1) settings
    #[serde(default = "default_av1_preset")]
    pub preset: u32, // 0-13, default 8

    #[serde(default)]
    pub tune: u32, // 0=visual (PSNR), 1=SSIM, 2=VMAF

    #[serde(default)]
    pub film_grain: u32, // 0-50

    #[serde(default)]
    pub film_grain_denoise: bool, // denoise before grain synthesis

    #[serde(default = "default_true")]
    pub enable_overlays: bool,

    #[serde(default = "default_true")]
    pub scd: bool, // scene change detection

    #[serde(default = "default_av1_scm")]
    pub scm: u32, // screen content mode: 0=off, 1=on, 2=auto

    #[serde(default = "default_true")]
    pub enable_tf: bool, // temporal filtering

    // Hardware encoder settings
    #[serde(default = "default_av1_hw_preset")]
    pub hw_preset: String, // qsv: "1"-"7", nvenc: "p1"-"p7"

    #[serde(default = "default_av1_hw_cq")]
    pub hw_cq: u32, // Legacy: use encoder-specific fields below

    // Per-encoder quality (CQ/CRF) - these take precedence over hw_cq
    #[serde(default = "default_svt_crf")]
    pub svt_crf: u32, // Software SVT-AV1: 0-63, lower=better

    #[serde(default = "default_qsv_cq")]
    pub qsv_cq: u32, // Intel QSV: 1-255, lower=better

    #[serde(default = "default_nvenc_cq")]
    pub nvenc_cq: u32, // NVIDIA: 0-63, lower=better

    #[serde(default = "default_vaapi_cq")]
    pub vaapi_cq: u32, // VAAPI: 1-255, lower=better

    #[serde(default = "default_av1_hw_lookahead")]
    pub hw_lookahead: u32, // rc-lookahead depth

    #[serde(default)]
    pub hw_tile_cols: u32,

    #[serde(default)]
    pub hw_tile_rows: u32,

    #[serde(default)]
    pub hw_denoise: u32, // 0 = off, QSV: 0-100, VAAPI: 0-64

    #[serde(default)]
    pub hw_detail: u32, // 0 = off, QSV: 0-100, VAAPI: 0-64
}

impl Default for Av1Config {
    fn default() -> Self {
        Self {
            preset: default_av1_preset(),
            tune: 0,
            film_grain: 0,
            film_grain_denoise: false,
            enable_overlays: true,
            scd: true,
            scm: default_av1_scm(),
            enable_tf: true,
            hw_preset: default_av1_hw_preset(),
            hw_cq: default_av1_hw_cq(),
            svt_crf: default_svt_crf(),
            qsv_cq: default_qsv_cq(),
            nvenc_cq: default_nvenc_cq(),
            vaapi_cq: default_vaapi_cq(),
            hw_lookahead: default_av1_hw_lookahead(),
            hw_tile_cols: 0,
            hw_tile_rows: 0,
            hw_denoise: 0,
            hw_detail: 0,
        }
    }
}

/// Codec selection with embedded configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "codec_type")]
pub enum Codec {
    Vp9(Vp9Config),
    Av1(Av1Config),
}

impl Default for Codec {
    fn default() -> Self {
        Codec::Av1(Av1Config::default())
    }
}

impl Codec {
    /// Get the codec family name
    pub fn name(&self) -> &'static str {
        match self {
            Codec::Vp9(_) => "VP9",
            Codec::Av1(_) => "AV1",
        }
    }

    /// Check if this is VP9
    pub fn is_vp9(&self) -> bool {
        matches!(self, Codec::Vp9(_))
    }

    /// Check if this is AV1
    pub fn is_av1(&self) -> bool {
        matches!(self, Codec::Av1(_))
    }

    /// Get VP9 config if this is VP9
    pub fn as_vp9(&self) -> Option<&Vp9Config> {
        match self {
            Codec::Vp9(config) => Some(config),
            _ => None,
        }
    }

    /// Get AV1 config if this is AV1
    pub fn as_av1(&self) -> Option<&Av1Config> {
        match self {
            Codec::Av1(config) => Some(config),
            _ => None,
        }
    }

    /// Get mutable VP9 config if this is VP9
    pub fn as_vp9_mut(&mut self) -> Option<&mut Vp9Config> {
        match self {
            Codec::Vp9(config) => Some(config),
            _ => None,
        }
    }

    /// Get mutable AV1 config if this is AV1
    pub fn as_av1_mut(&mut self) -> Option<&mut Av1Config> {
        match self {
            Codec::Av1(config) => Some(config),
            _ => None,
        }
    }
}
