//! VP9-specific encoding configuration.
//!
//! Contains the `Vp9Config` struct for VP9 codec settings used by
//! libvpx-vp9 (software), vp9_qsv (Intel QSV), and vp9_vaapi (VAAPI).

use serde::{Deserialize, Serialize};

// Default functions for Vp9Config hardware settings
fn default_hw_quality() -> u32 {
    70
}
fn default_hw_loop_filter() -> u32 {
    16
}
fn default_hw_loop_filter_sharpness() -> u32 {
    4
}
fn default_hw_rc_mode() -> u32 {
    1
} // CQP mode (only supported - ICQ/VBR/CBR removed due to Arc driver bugs)
fn default_hw_compression_level() -> u32 {
    4
} // Balanced speed/compression

fn default_zero_string() -> String {
    "0".to_string()
}

// Default functions for Vp9Config software settings
fn default_quality_mode() -> String {
    "good".to_string()
}
fn default_cpu_used() -> u32 {
    2
}
fn default_cpu_used_pass1() -> u32 {
    4
}
fn default_cpu_used_pass2() -> u32 {
    1
}
fn default_row_mt() -> bool {
    true
}
fn default_tile_columns() -> i32 {
    2
}
fn default_auto_alt_ref() -> u32 {
    1 // Enabled (0=disabled, 1=enabled, 2=enabled with statistics)
}
fn default_arnr_max_frames() -> u32 {
    7
}
fn default_arnr_strength() -> u32 {
    3
}
fn default_arnr_type() -> i32 {
    -1
}
fn default_lag_in_frames() -> u32 {
    25
}
fn default_enable_tpl() -> bool {
    true
}
fn default_sharpness() -> i32 {
    -1
}
fn default_aq_mode() -> i32 {
    1
}
fn default_tune_content() -> String {
    "default".to_string()
}
fn default_vp9_qsv_preset() -> u32 {
    4
}
fn default_vp9_qsv_look_ahead() -> bool {
    true
}
fn default_vp9_qsv_look_ahead_depth() -> u32 {
    40
}
fn default_undershoot_pct() -> i32 {
    -1
}
fn default_overshoot_pct() -> i32 {
    -1
}

/// VP9-specific encoding settings (for libvpx-vp9, vp9_qsv, and vp9_vaapi)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vp9Config {
    // Profile
    #[serde(default)]
    pub vp9_profile: u8, // 0-3

    #[serde(default = "default_quality_mode")]
    pub quality_mode: String, // "good", "realtime", "best"

    // Speed settings
    #[serde(default = "default_cpu_used")]
    pub cpu_used: u32, // Single-pass: 0-8

    #[serde(default = "default_cpu_used_pass1")]
    pub cpu_used_pass1: u32,

    #[serde(default = "default_cpu_used_pass2")]
    pub cpu_used_pass2: u32,

    // Parallelism
    #[serde(default = "default_row_mt")]
    pub row_mt: bool,

    #[serde(default = "default_tile_columns")]
    pub tile_columns: i32, // log2: 0-6

    #[serde(default)]
    pub tile_rows: i32,

    #[serde(default)]
    pub threads: u32,

    #[serde(default)]
    pub frame_parallel: bool,

    // Alt-ref & lookahead
    #[serde(default = "default_auto_alt_ref")]
    pub auto_alt_ref: u32,

    #[serde(default = "default_arnr_max_frames")]
    pub arnr_max_frames: u32,

    #[serde(default = "default_arnr_strength")]
    pub arnr_strength: u32,

    #[serde(default = "default_arnr_type")]
    pub arnr_type: i32,

    #[serde(default = "default_lag_in_frames")]
    pub lag_in_frames: u32,

    // Advanced tuning
    #[serde(default = "default_enable_tpl")]
    pub enable_tpl: bool,

    #[serde(default = "default_sharpness")]
    pub sharpness: i32,

    #[serde(default)]
    pub noise_sensitivity: u32,

    #[serde(default = "default_zero_string")]
    pub static_thresh: String,

    #[serde(default = "default_zero_string")]
    pub max_intra_rate: String,

    #[serde(default = "default_aq_mode")]
    pub aq_mode: i32,

    #[serde(default = "default_tune_content")]
    pub tune_content: String,

    #[serde(default = "default_undershoot_pct")]
    pub undershoot_pct: i32,

    #[serde(default = "default_overshoot_pct")]
    pub overshoot_pct: i32,

    // Hardware encoding (VAAPI VP9)
    #[serde(default = "default_hw_rc_mode")]
    pub hw_rc_mode: u32,

    #[serde(default = "default_hw_quality")]
    pub hw_global_quality: u32,

    #[serde(default)]
    pub hw_b_frames: u32,

    #[serde(default = "default_hw_loop_filter")]
    pub hw_loop_filter_level: u32,

    #[serde(default = "default_hw_loop_filter_sharpness")]
    pub hw_loop_filter_sharpness: u32,

    #[serde(default = "default_hw_compression_level")]
    pub hw_compression_level: u32,

    #[serde(default)]
    pub hw_denoise: u32, // 0 = off, QSV: 0-100, VAAPI: 0-64

    #[serde(default)]
    pub hw_detail: u32, // 0 = off, QSV: 0-100, VAAPI: 0-64

    // QSV-only controls (used when `vp9_qsv` is selected)
    #[serde(default = "default_vp9_qsv_preset")]
    pub qsv_preset: u32, // 1-7 (1=best quality, 7=fastest)

    #[serde(default = "default_vp9_qsv_look_ahead")]
    pub qsv_look_ahead: bool,

    #[serde(default = "default_vp9_qsv_look_ahead_depth")]
    pub qsv_look_ahead_depth: u32, // frames
}

impl Default for Vp9Config {
    fn default() -> Self {
        Self {
            vp9_profile: 0,
            quality_mode: default_quality_mode(),
            cpu_used: default_cpu_used(),
            cpu_used_pass1: default_cpu_used_pass1(),
            cpu_used_pass2: default_cpu_used_pass2(),
            row_mt: default_row_mt(),
            tile_columns: default_tile_columns(),
            tile_rows: 0,
            threads: 0,
            frame_parallel: false,
            auto_alt_ref: default_auto_alt_ref(),
            arnr_max_frames: default_arnr_max_frames(),
            arnr_strength: default_arnr_strength(),
            arnr_type: default_arnr_type(),
            lag_in_frames: default_lag_in_frames(),
            enable_tpl: default_enable_tpl(),
            sharpness: default_sharpness(),
            noise_sensitivity: 0,
            static_thresh: "0".to_string(),
            max_intra_rate: "0".to_string(),
            aq_mode: default_aq_mode(),
            tune_content: default_tune_content(),
            undershoot_pct: default_undershoot_pct(),
            overshoot_pct: default_overshoot_pct(),
            hw_rc_mode: default_hw_rc_mode(),
            hw_global_quality: default_hw_quality(),
            hw_b_frames: 0,
            hw_loop_filter_level: default_hw_loop_filter(),
            hw_loop_filter_sharpness: default_hw_loop_filter_sharpness(),
            hw_compression_level: default_hw_compression_level(),
            hw_denoise: 0,
            hw_detail: 0,
            qsv_preset: default_vp9_qsv_preset(),
            qsv_look_ahead: default_vp9_qsv_look_ahead(),
            qsv_look_ahead_depth: default_vp9_qsv_look_ahead_depth(),
        }
    }
}
