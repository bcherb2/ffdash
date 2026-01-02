use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

use crate::engine::validate::{HardwareAvailability, validate_profile};

// Re-export codec-specific configs from their dedicated modules
pub use super::av1_config::{Av1Config, Codec};
pub use super::hw_config::HwEncodingConfig;
pub use super::vp9_config::Vp9Config;

// Default values for Profile fields (hardware encoding)
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

fn default_240_string() -> String {
    "240".to_string()
}

fn default_output_dir() -> String {
    ".".to_string() // Current directory
}

fn default_filename_pattern() -> String {
    "{basename}".to_string()
}

fn default_audio_primary_codec() -> String {
    "libopus".to_string()
}

fn default_audio_primary_bitrate() -> u32 {
    128
}

fn default_audio_ac3_bitrate() -> u32 {
    448
}

fn default_audio_stereo_codec() -> String {
    "aac".to_string()
}

fn default_audio_stereo_bitrate() -> u32 {
    128
}

// ============================================================================
// Default values for VMAF (Auto-VAMF) feature
// ============================================================================

fn default_vmaf_target() -> f32 {
    93.0
}
fn default_vmaf_window_duration_sec() -> u32 {
    10
}
fn default_vmaf_analysis_budget_sec() -> u32 {
    60
}
fn default_vmaf_n_subsample() -> u32 {
    30
}
fn default_vmaf_max_attempts() -> u8 {
    3
}
fn default_vmaf_step() -> u8 {
    2
}

// ============================================================================
// Profile struct (with codec-specific config)
// ============================================================================

/// Encoding profile configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub suffix: String,
    pub container: String,
    pub video_codec: String,
    // Audio - multi-track support
    #[serde(default = "default_audio_primary_codec")]
    pub audio_primary_codec: String, // "passthrough", "libopus", "aac", "mp3", "vorbis"
    #[serde(default = "default_audio_primary_bitrate")]
    pub audio_primary_bitrate: u32,
    #[serde(default)]
    pub audio_primary_downmix: bool, // Downmix primary track to stereo (2ch)
    #[serde(default)]
    pub audio_add_ac3: bool,
    #[serde(default = "default_audio_ac3_bitrate")]
    pub audio_ac3_bitrate: u32,
    #[serde(default)]
    pub audio_add_stereo: bool,
    #[serde(default = "default_audio_stereo_codec")]
    pub audio_stereo_codec: String, // "aac", "libopus"
    #[serde(default = "default_audio_stereo_bitrate")]
    pub audio_stereo_bitrate: u32,

    // Legacy fields for backward compatibility (deprecated)
    #[serde(default, skip_serializing)]
    pub audio_codec: Option<String>,
    #[serde(default, skip_serializing)]
    pub audio_bitrate: Option<u32>,
    #[serde(default, skip_serializing)]
    pub downmix_stereo: Option<bool>,
    #[serde(default, skip_serializing)]
    pub audio_passthrough: Option<bool>,

    // Output settings
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
    #[serde(default = "default_filename_pattern")]
    pub filename_pattern: String,
    #[serde(default)]
    pub overwrite: bool,
    #[serde(default)]
    pub additional_args: String,

    // Video output constraints (max FPS, max resolution)
    pub fps: u32,          // 0 = source (no fps cap)
    pub scale_width: i32,  // -2 = source, -1 = auto, >0 = max width
    pub scale_height: i32, // -2 = source, -1 = auto, >0 = max height

    // Rate control
    pub crf: u32,
    pub video_target_bitrate: u32,
    pub video_min_bitrate: u32,
    pub video_max_bitrate: u32,
    pub video_bufsize: u32,
    pub undershoot_pct: i32,
    pub overshoot_pct: i32,

    // Speed & quality
    pub cpu_used: u32,
    pub cpu_used_pass1: u32,
    pub cpu_used_pass2: u32,
    pub two_pass: bool,
    pub quality_mode: String, // "good", "realtime", "best"

    // VP9 settings
    pub vp9_profile: u8,
    pub pix_fmt: String,

    // Parallelism
    pub row_mt: bool,
    pub tile_columns: i32,
    pub tile_rows: i32,
    pub threads: u32,
    pub frame_parallel: bool,
    pub max_workers: u32, // Number of concurrent encoding jobs

    // GOP & keyframes
    #[serde(default = "default_240_string")]
    pub gop_length: String,
    #[serde(default = "default_zero_string")]
    pub keyint_min: String,
    pub fixed_gop: bool,
    pub lag_in_frames: u32,
    pub auto_alt_ref: u32,

    // Alt-ref denoising (ARNR)
    pub arnr_max_frames: u32,
    pub arnr_strength: u32,
    pub arnr_type: i32,

    // Advanced tuning
    pub enable_tpl: bool,
    pub sharpness: i32,
    pub noise_sensitivity: u32,
    #[serde(default = "default_zero_string")]
    pub static_thresh: String,
    #[serde(default = "default_zero_string")]
    pub max_intra_rate: String,
    pub aq_mode: i32,
    pub tune_content: String,

    // Color / HDR
    pub colorspace: i32,
    pub color_primaries: i32,
    pub color_trc: i32,
    pub color_range: i32,

    // Hardware encoding settings (Intel Arc VAAPI)
    #[serde(default)]
    pub use_hardware_encoding: bool,

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

    // Codec-specific configuration (VP9 or AV1)
    // When loading old profiles without this field, defaults to VP9
    #[serde(default)]
    pub codec: Codec,

    // Auto-VAMF settings (quality calibration via VMAF)
    #[serde(default)]
    pub vmaf_enabled: bool,

    #[serde(default = "default_vmaf_target")]
    pub vmaf_target: f32,

    #[serde(default = "default_vmaf_window_duration_sec")]
    pub vmaf_window_duration_sec: u32,

    #[serde(default = "default_vmaf_analysis_budget_sec")]
    pub vmaf_analysis_budget_sec: u32,

    #[serde(default = "default_vmaf_n_subsample")]
    pub vmaf_n_subsample: u32,

    #[serde(default = "default_vmaf_max_attempts")]
    pub vmaf_max_attempts: u8,

    #[serde(default = "default_vmaf_step")]
    pub vmaf_step: u8,
}

impl Profile {
    pub fn from_config(name: String, config: &crate::ui::state::ConfigState) -> Self {
        use crate::ui::options;
        use crate::ui::state::RateControlMode;

        // Map list state selections to actual values
        let quality_mode_idx = config.quality_mode_state.selected().unwrap_or(0);
        let quality_mode = options::quality_mode_from_idx(quality_mode_idx).to_string();

        let vp9_profile = config.profile_dropdown_state.selected().unwrap_or(0) as u8;

        let pix_fmt_idx = config.pix_fmt_state.selected().unwrap_or(0);
        let pix_fmt = options::pix_fmt_from_idx(pix_fmt_idx).to_string();

        let aq_mode_idx = config.aq_mode_state.selected().unwrap_or(0);
        let aq_mode = options::aq_mode_from_idx(aq_mode_idx);

        let tune_content_idx = config.tune_content_state.selected().unwrap_or(0);
        let tune_content = options::tune_content_from_idx(tune_content_idx).to_string();

        // Audio primary codec from enum
        let audio_primary_codec = match config.audio_primary_codec {
            crate::ui::state::AudioPrimaryCodec::Passthrough => "passthrough".to_string(),
            crate::ui::state::AudioPrimaryCodec::Opus => "libopus".to_string(),
            crate::ui::state::AudioPrimaryCodec::Aac => "aac".to_string(),
            crate::ui::state::AudioPrimaryCodec::Mp3 => "mp3".to_string(),
            crate::ui::state::AudioPrimaryCodec::Vorbis => "vorbis".to_string(),
        };

        // Audio stereo codec from enum
        let audio_stereo_codec = match config.audio_stereo_codec {
            crate::ui::state::AudioStereoCodec::Aac => "aac".to_string(),
            crate::ui::state::AudioStereoCodec::Opus => "libopus".to_string(),
        };

        let container_idx = config.container_dropdown_state.selected().unwrap_or(0);
        let container = options::container_from_idx(container_idx).to_string();

        // Use numeric color values directly (synced from preset selection)
        let colorspace = config.colorspace;
        let color_primaries = config.color_primaries;
        let color_trc = config.color_trc;
        let color_range = config.color_range;

        let arnr_type_idx = config.arnr_type_state.selected().unwrap_or(0);
        let arnr_type = options::arnr_type_from_idx(arnr_type_idx);

        // FPS: prefer numeric field if set, otherwise derive from dropdown selection
        let fps_idx = config.fps_dropdown_state.selected().unwrap_or(0);
        let mut fps = options::fps_from_idx(fps_idx);
        if config.fps != 0 {
            fps = config.fps;
        }

        // Resolution: prefer explicit numeric fields if not "source" (-2), otherwise dropdown
        let res_idx = config.resolution_dropdown_state.selected().unwrap_or(0);
        let mut scale = options::resolution_from_idx(res_idx);
        if config.scale_width != -2 || config.scale_height != -2 {
            scale = (config.scale_width, config.scale_height);
        }
        let (scale_width, scale_height) = scale;

        // Map rate control mode to bitrate settings
        let (video_target_bitrate, video_min_bitrate, video_max_bitrate, video_bufsize) =
            match config.rate_control_mode {
                RateControlMode::CQ => (0, 0, 0, 0), // CRF mode
                RateControlMode::CQCap => (0, 0, config.video_max_bitrate, config.video_bufsize),
                RateControlMode::TwoPassVBR => (
                    config.video_target_bitrate,
                    config.video_min_bitrate,
                    config.video_max_bitrate,
                    config.video_bufsize,
                ),
                RateControlMode::CBR => (
                    config.video_target_bitrate,
                    config.video_target_bitrate,
                    config.video_target_bitrate,
                    config.video_bufsize,
                ),
            };

        let mut profile = Self {
            name: name.clone(),
            suffix: name.to_lowercase().replace(' ', "_"),
            container,
            video_codec: "libvpx-vp9".to_string(),
            audio_primary_codec,
            audio_primary_bitrate: config.audio_primary_bitrate,
            audio_primary_downmix: config.audio_primary_downmix,
            audio_add_ac3: config.audio_add_ac3,
            audio_ac3_bitrate: config.audio_ac3_bitrate,
            audio_add_stereo: config.audio_add_stereo,
            audio_stereo_codec,
            audio_stereo_bitrate: config.audio_stereo_bitrate,
            audio_codec: None,
            audio_bitrate: None,
            downmix_stereo: None,
            audio_passthrough: None,

            // Output settings
            output_dir: config.output_dir.clone(),
            filename_pattern: config.filename_pattern.clone(),
            overwrite: config.overwrite,
            additional_args: config.additional_args.clone(),

            // Video output constraints
            fps,
            scale_width,
            scale_height,

            // Rate control
            crf: config.crf,
            video_target_bitrate,
            video_min_bitrate,
            video_max_bitrate,
            video_bufsize,
            undershoot_pct: config.undershoot_pct,
            overshoot_pct: config.overshoot_pct,

            // Speed & quality
            cpu_used: config.cpu_used,
            cpu_used_pass1: config.cpu_used_pass1,
            cpu_used_pass2: config.cpu_used_pass2,
            two_pass: config.two_pass,
            quality_mode: quality_mode.clone(),

            // VP9 settings
            vp9_profile,
            pix_fmt,

            // Parallelism
            row_mt: config.row_mt,
            tile_columns: config.tile_columns,
            tile_rows: config.tile_rows,
            threads: config.threads,
            frame_parallel: config.frame_parallel,
            max_workers: config.max_workers,

            // GOP & keyframes
            gop_length: config.gop_length.clone(),
            keyint_min: config.keyint_min.clone(),
            fixed_gop: config.fixed_gop,
            lag_in_frames: config.lag_in_frames,
            auto_alt_ref: config.auto_alt_ref,

            // Alt-ref denoising
            arnr_max_frames: config.arnr_max_frames,
            arnr_strength: config.arnr_strength,
            arnr_type,

            // Advanced tuning
            enable_tpl: config.enable_tpl,
            sharpness: config.sharpness,
            noise_sensitivity: config.noise_sensitivity,
            static_thresh: config.static_thresh.clone(),
            max_intra_rate: config.max_intra_rate.clone(),
            aq_mode,
            tune_content: tune_content.clone(),

            // Color / HDR
            colorspace,
            color_primaries,
            color_trc,
            color_range,

            // Hardware encoding settings
            use_hardware_encoding: config.use_hardware_encoding,
            hw_rc_mode: config.vaapi_rc_mode.parse().unwrap_or(1), // Default to CQP
            hw_global_quality: config.qsv_global_quality,
            hw_b_frames: config.vaapi_b_frames.parse().unwrap_or(0),
            hw_loop_filter_level: config.vaapi_loop_filter_level.parse().unwrap_or(16),
            hw_loop_filter_sharpness: config.vaapi_loop_filter_sharpness.parse().unwrap_or(4),
            hw_compression_level: config.vaapi_compression_level.parse().unwrap_or(4),

            // Codec-specific config
            codec: match config.codec_selection {
                crate::ui::state::CodecSelection::Vp9 => Codec::Vp9(Vp9Config {
                    vp9_profile,
                    quality_mode,
                    cpu_used: config.cpu_used,
                    cpu_used_pass1: config.cpu_used_pass1,
                    cpu_used_pass2: config.cpu_used_pass2,
                    row_mt: config.row_mt,
                    tile_columns: config.tile_columns,
                    tile_rows: config.tile_rows,
                    threads: config.threads,
                    frame_parallel: config.frame_parallel,
                    auto_alt_ref: config.auto_alt_ref,
                    arnr_max_frames: config.arnr_max_frames,
                    arnr_strength: config.arnr_strength,
                    arnr_type,
                    lag_in_frames: config.lag_in_frames,
                    enable_tpl: config.enable_tpl,
                    sharpness: config.sharpness,
                    noise_sensitivity: config.noise_sensitivity,
                    static_thresh: config.static_thresh.clone(),
                    max_intra_rate: config.max_intra_rate.clone(),
                    aq_mode,
                    tune_content,
                    undershoot_pct: config.undershoot_pct,
                    overshoot_pct: config.overshoot_pct,
                    hw_rc_mode: config.vaapi_rc_mode.parse().unwrap_or(4),
                    hw_global_quality: config.qsv_global_quality,
                    hw_b_frames: config.vaapi_b_frames.parse().unwrap_or(0),
                    hw_loop_filter_level: config.vaapi_loop_filter_level.parse().unwrap_or(16),
                    hw_loop_filter_sharpness: config
                        .vaapi_loop_filter_sharpness
                        .parse()
                        .unwrap_or(4),
                    hw_compression_level: config.vaapi_compression_level.parse().unwrap_or(4),
                    qsv_preset: config.vp9_qsv_preset,
                    qsv_look_ahead: config.vp9_qsv_lookahead,
                    qsv_look_ahead_depth: config.vp9_qsv_lookahead_depth,
                    hw_denoise: config.hw_denoise.parse().unwrap_or(0),
                    hw_detail: config.hw_detail.parse().unwrap_or(0),
                }),
                crate::ui::state::CodecSelection::Av1 => {
                    let tune_idx = config.av1_tune_state.selected().unwrap_or(0);
                    let scm_idx = config.av1_scm_state.selected().unwrap_or(0);

                    let hw_lookahead = config.av1_hw_lookahead.min(100);
                    Codec::Av1(Av1Config {
                        preset: config.av1_preset,
                        tune: tune_idx as u32,
                        film_grain: config.av1_film_grain,
                        film_grain_denoise: config.av1_film_grain_denoise,
                        enable_overlays: config.av1_enable_overlays,
                        scd: config.av1_scd,
                        scm: scm_idx as u32,
                        enable_tf: config.av1_enable_tf,
                        hw_preset: config.av1_hw_preset.to_string(),
                        hw_cq: config.av1_hw_cq,
                        svt_crf: config.av1_svt_crf,
                        qsv_cq: config.av1_qsv_cq,
                        nvenc_cq: config.av1_nvenc_cq,
                        vaapi_cq: config.av1_vaapi_cq,
                        hw_lookahead,
                        hw_tile_cols: config.av1_hw_tile_cols,
                        hw_tile_rows: config.av1_hw_tile_rows,
                        hw_denoise: config.hw_denoise.parse().unwrap_or(0),
                        hw_detail: config.hw_detail.parse().unwrap_or(0),
                    })
                }
            },

            // Auto-VAMF settings
            vmaf_enabled: config.auto_vmaf_enabled,
            vmaf_target: config.auto_vmaf_target.parse().unwrap_or(93.0),
            vmaf_window_duration_sec: 10, // Not exposed in UI for v1
            vmaf_analysis_budget_sec: 60, // Not exposed in UI for v1
            vmaf_n_subsample: 30,         // Not exposed in UI for v1
            vmaf_max_attempts: config.auto_vmaf_max_attempts.parse().unwrap_or(3),
            vmaf_step: config.auto_vmaf_step.parse().unwrap_or(2),
        };

        if let Err(errs) = validate_profile(&profile, HardwareAvailability::default()) {
            if let Some(first) = errs.first() {
                profile.suffix = format!(
                    "{}__invalid_{}",
                    profile.suffix,
                    first.field.replace(' ', "_")
                );
            }
        }

        profile
    }

    /// Apply this Profile's settings to a ConfigState
    pub fn apply_to_config(&self, config: &mut crate::ui::state::ConfigState) {
        use crate::ui::state::RateControlMode;

        // Apply encoding parameters
        config.crf = self.crf;
        config.cpu_used = self.cpu_used;
        config.cpu_used_pass1 = self.cpu_used_pass1;
        config.cpu_used_pass2 = self.cpu_used_pass2;
        config.two_pass = self.two_pass;

        // Audio settings - multi-track
        config.audio_primary_codec = match self.audio_primary_codec.as_str() {
            "passthrough" => crate::ui::state::AudioPrimaryCodec::Passthrough,
            "libopus" => crate::ui::state::AudioPrimaryCodec::Opus,
            "aac" => crate::ui::state::AudioPrimaryCodec::Aac,
            "mp3" => crate::ui::state::AudioPrimaryCodec::Mp3,
            "vorbis" => crate::ui::state::AudioPrimaryCodec::Vorbis,
            _ => crate::ui::state::AudioPrimaryCodec::Opus,
        };
        config.audio_primary_codec_state.select(Some(config.audio_primary_codec.to_index()));
        config.audio_primary_bitrate = self.audio_primary_bitrate;
        config.audio_primary_downmix = self.audio_primary_downmix;
        config.audio_add_ac3 = self.audio_add_ac3;
        config.audio_ac3_bitrate = self.audio_ac3_bitrate;
        config.audio_add_stereo = self.audio_add_stereo;
        config.audio_stereo_codec = match self.audio_stereo_codec.as_str() {
            "aac" => crate::ui::state::AudioStereoCodec::Aac,
            "libopus" => crate::ui::state::AudioStereoCodec::Opus,
            _ => crate::ui::state::AudioStereoCodec::Aac,
        };
        config.audio_stereo_codec_state.select(Some(config.audio_stereo_codec.to_index()));
        config.audio_stereo_bitrate = self.audio_stereo_bitrate;

        // Output settings
        config.output_dir = self.output_dir.clone();
        config.filename_pattern = self.filename_pattern.clone();
        config.overwrite = self.overwrite;
        config.additional_args = self.additional_args.clone();

        // Video output constraints
        config.fps = self.fps;
        config.scale_width = self.scale_width;
        config.scale_height = self.scale_height;

        // Map FPS value to dropdown index
        let fps_idx = crate::ui::options::fps_to_idx(self.fps);
        config.fps_dropdown_state.select(Some(fps_idx));

        // Map resolution to dropdown index
        let res_idx = crate::ui::options::resolution_to_idx(self.scale_width, self.scale_height);
        config.resolution_dropdown_state.select(Some(res_idx));

        // Rate control
        config.video_target_bitrate = self.video_target_bitrate;
        config.video_min_bitrate = self.video_min_bitrate;
        config.video_max_bitrate = self.video_max_bitrate;
        config.video_bufsize = self.video_bufsize;
        config.undershoot_pct = self.undershoot_pct;
        config.overshoot_pct = self.overshoot_pct;

        // Determine rate control mode from bitrate settings
        config.rate_control_mode = if self.video_target_bitrate == 0 && self.video_max_bitrate == 0
        {
            RateControlMode::CQ
        } else if self.video_target_bitrate == 0 && self.video_max_bitrate > 0 {
            RateControlMode::CQCap
        } else if self.video_target_bitrate == self.video_min_bitrate
            && self.video_target_bitrate == self.video_max_bitrate
        {
            RateControlMode::CBR
        } else {
            RateControlMode::TwoPassVBR
        };

        // Parallelism
        config.row_mt = self.row_mt;
        config.tile_columns = self.tile_columns;
        config.tile_rows = self.tile_rows;
        config.threads = self.threads;
        config.frame_parallel = self.frame_parallel;
        config.max_workers = self.max_workers;

        // GOP & keyframes
        config.gop_length = self.gop_length.clone();
        config.keyint_min = self.keyint_min.clone();
        config.fixed_gop = self.fixed_gop;
        config.lag_in_frames = self.lag_in_frames;
        config.auto_alt_ref = self.auto_alt_ref;

        // Alt-ref denoising
        config.arnr_max_frames = self.arnr_max_frames;
        config.arnr_strength = self.arnr_strength;

        // Advanced tuning
        config.enable_tpl = self.enable_tpl;
        config.sharpness = self.sharpness;
        config.noise_sensitivity = self.noise_sensitivity;
        config.static_thresh = self.static_thresh.clone();
        config.max_intra_rate = self.max_intra_rate.clone();

        // Map Profile values back to ListState selections

        // Quality mode: "good", "realtime", "best" → 0, 1, 2
        let quality_idx = crate::ui::options::quality_mode_to_idx(self.quality_mode.as_str());
        config.quality_mode_state.select(Some(quality_idx));

        // VP9 profile: u8 → index
        config
            .profile_dropdown_state
            .select(Some(self.vp9_profile as usize));

        // Pixel format: "yuv420p", "yuv420p10le" → 0, 1
        let pix_fmt_idx = crate::ui::options::pix_fmt_to_idx(self.pix_fmt.as_str());
        config.pix_fmt_state.select(Some(pix_fmt_idx));

        // AQ mode: ffmpeg value → index
        let aq_idx = crate::ui::options::aq_mode_to_idx(self.aq_mode);
        config.aq_mode_state.select(Some(aq_idx));

        // Audio is handled in apply_to_config()

        // Container: string → index
        let container_idx = crate::ui::options::container_to_idx(self.container.as_str());
        config.container_dropdown_state.select(Some(container_idx));

        // Tune content: string → index
        let tune_idx = crate::ui::options::tune_content_to_idx(self.tune_content.as_str());
        config.tune_content_state.select(Some(tune_idx));

        // Detect preset from color values, or default to Auto if custom
        if let Some(preset) = crate::ui::options::colorspace_values_to_preset(
            self.colorspace,
            self.color_primaries,
            self.color_trc,
            self.color_range,
        ) {
            config.colorspace_preset = preset;
            config.colorspace_preset_state.select(Some(crate::ui::options::colorspace_preset_to_idx(preset)));
        } else {
            // Custom values → default to Auto in UI, but preserve actual values
            config.colorspace_preset = crate::ui::state::ColorSpacePreset::Auto;
            config.colorspace_preset_state.select(Some(0));
        }

        // Always preserve actual numeric values
        config.colorspace = self.colorspace;
        config.color_primaries = self.color_primaries;
        config.color_trc = self.color_trc;
        config.color_range = self.color_range;

        // ARNR type: ffmpeg value → index
        let arnr_type_idx = crate::ui::options::arnr_type_to_idx(self.arnr_type);
        config.arnr_type_state.select(Some(arnr_type_idx));

        // Synchronize numeric fields with dropdown states
        // While these are read from dropdowns when saving profiles (from_config),
        // we keep them synchronized here so ConfigState accurately reflects loaded values
        config.colorspace = self.colorspace;
        config.color_primaries = self.color_primaries;
        config.color_trc = self.color_trc;
        config.color_range = self.color_range;
        config.arnr_type = self.arnr_type;

        // Hardware encoding settings
        config.use_hardware_encoding = self.use_hardware_encoding;
        config.vaapi_rc_mode = self.hw_rc_mode.to_string();
        config.qsv_global_quality = self.hw_global_quality;
        config.vaapi_b_frames = self.hw_b_frames.to_string();
        config.vaapi_loop_filter_level = self.hw_loop_filter_level.to_string();
        config.vaapi_loop_filter_sharpness = self.hw_loop_filter_sharpness.to_string();
        config.vaapi_compression_level = self.hw_compression_level.to_string();

        // Apply codec-specific settings
        match &self.codec {
            Codec::Vp9(vp9) => {
                config.codec_selection = crate::ui::options::codec_selection_from_idx(0);
                config
                    .video_codec_state
                    .select(Some(crate::ui::options::codec_selection_to_idx(
                        config.codec_selection,
                    )));
                // VP9-specific fields are already applied above (common fields)
                config.vp9_qsv_preset = vp9.qsv_preset;
                config.vp9_qsv_lookahead = vp9.qsv_look_ahead;
                config.vp9_qsv_lookahead_depth = vp9.qsv_look_ahead_depth;
                config.hw_denoise = vp9.hw_denoise.to_string();
                config.hw_detail = vp9.hw_detail.to_string();
            }
            Codec::Av1(av1) => {
                config.codec_selection = crate::ui::options::codec_selection_from_idx(1);
                config
                    .video_codec_state
                    .select(Some(crate::ui::options::codec_selection_to_idx(
                        config.codec_selection,
                    )));

                // AV1 software settings
                config.av1_preset = av1.preset;
                config
                    .av1_tune_state
                    .select(Some(crate::ui::options::av1_tune_to_idx(av1.tune)));
                config.av1_film_grain = av1.film_grain;
                config.av1_film_grain_denoise = av1.film_grain_denoise;
                config.av1_enable_overlays = av1.enable_overlays;
                config.av1_scd = av1.scd;
                config
                    .av1_scm_state
                    .select(Some(crate::ui::options::av1_scm_to_idx(av1.scm)));
                config.av1_enable_tf = av1.enable_tf;

                // AV1 hardware settings - convert string preset to u32
                config.av1_hw_preset = av1.hw_preset.trim_start_matches('p').parse().unwrap_or(4);
                config.av1_hw_cq = av1.hw_cq;
                config.av1_svt_crf = av1.svt_crf;
                config.av1_qsv_cq = av1.qsv_cq;
                config.av1_nvenc_cq = av1.nvenc_cq;
                config.av1_vaapi_cq = av1.vaapi_cq;
                config.av1_hw_lookahead = av1.hw_lookahead;
                config.av1_hw_tile_cols = av1.hw_tile_cols;
                config.av1_hw_tile_rows = av1.hw_tile_rows;
                config.hw_denoise = av1.hw_denoise.to_string();
                config.hw_detail = av1.hw_detail.to_string();
            }
        }

        // Auto-VAMF settings
        config.auto_vmaf_enabled = self.vmaf_enabled;
        config.auto_vmaf_target = self.vmaf_target.to_string();
        config.auto_vmaf_step = self.vmaf_step.to_string();
        config.auto_vmaf_max_attempts = self.vmaf_max_attempts.to_string();
    }

    /// Get the profiles directory path (creates if doesn't exist)
    /// Priority: ~/.config/ffdash/profiles/ (XDG standard)
    /// Fallback: ./.ffdash_profiles/ (current directory)
    pub fn profiles_dir() -> io::Result<std::path::PathBuf> {
        use std::env;
        use std::fs;

        // Use ~/.config/ffdash/profiles/ for macOS and Linux (XDG standard)
        // Use %APPDATA%/ffdash/profiles/ for Windows
        let config_dir = if cfg!(target_os = "windows") {
            env::var("APPDATA")
                .ok()
                .map(|a| std::path::PathBuf::from(a).join("ffdash"))
        } else {
            // macOS, Linux, and other Unix-like systems - use XDG config
            env::var("XDG_CONFIG_HOME")
                .ok()
                .map(|c| std::path::PathBuf::from(c).join("ffdash"))
                .or_else(|| {
                    env::var("HOME")
                        .ok()
                        .map(|h| std::path::PathBuf::from(h).join(".config").join("ffdash"))
                })
        };

        let profiles_path = if let Some(config) = config_dir {
            config.join("profiles")
        } else {
            // Fallback to current directory
            std::path::PathBuf::from(".ffdash_profiles")
        };

        // Create directory if it doesn't exist
        fs::create_dir_all(&profiles_path)?;

        Ok(profiles_path)
    }

    /// Synchronize legacy fields from the codec-specific configuration.
    ///
    /// This ensures video_codec, crf, hw_global_quality, and other legacy fields
    /// match the active Codec enum configuration. This is necessary because:
    /// 1. The UI updates codec-specific fields (e.g., Av1Config::hw_cq)
    /// 2. But command builders may still read legacy fields
    /// 3. Deserialized profiles may have stale legacy values
    ///
    /// Call this after loading a profile or before encoding to ensure consistency.
    pub fn sync_legacy_fields(&mut self) {
        match &self.codec {
            Codec::Vp9(vp9) => {
                // Sync video_codec hint (will be refined by select_encoder)
                // Don't hardcode a specific encoder - just set the codec family
                self.video_codec = if self.use_hardware_encoding {
                    "vp9_vaapi".to_string() // Placeholder - select_encoder will choose best
                } else {
                    "libvpx-vp9".to_string()
                };

                // Sync quality settings
                if self.use_hardware_encoding {
                    self.hw_global_quality = vp9.hw_global_quality;
                } else {
                    // Software VP9 uses crf from profile root (not in Vp9Config)
                    // Keep existing self.crf value
                }

                // Sync VP9-specific settings
                self.cpu_used = vp9.cpu_used;
                self.cpu_used_pass1 = vp9.cpu_used_pass1;
                self.cpu_used_pass2 = vp9.cpu_used_pass2;
                self.quality_mode = vp9.quality_mode.clone();
                self.vp9_profile = vp9.vp9_profile;
                self.row_mt = vp9.row_mt;
                self.tile_columns = vp9.tile_columns;
                self.tile_rows = vp9.tile_rows;
                self.lag_in_frames = vp9.lag_in_frames;
                self.auto_alt_ref = vp9.auto_alt_ref;
                self.arnr_max_frames = vp9.arnr_max_frames;
                self.arnr_strength = vp9.arnr_strength;
                self.arnr_type = vp9.arnr_type;
                self.enable_tpl = vp9.enable_tpl;
                self.sharpness = vp9.sharpness;
                self.noise_sensitivity = vp9.noise_sensitivity;
                self.static_thresh = vp9.static_thresh.clone();
                self.max_intra_rate = vp9.max_intra_rate.clone();
                self.aq_mode = vp9.aq_mode;
                self.tune_content = vp9.tune_content.clone();
                self.undershoot_pct = vp9.undershoot_pct;
                self.overshoot_pct = vp9.overshoot_pct;

                // Sync hardware-specific settings
                self.hw_rc_mode = vp9.hw_rc_mode;
                self.hw_b_frames = vp9.hw_b_frames;
                self.hw_loop_filter_level = vp9.hw_loop_filter_level;
                self.hw_loop_filter_sharpness = vp9.hw_loop_filter_sharpness;
                self.hw_compression_level = vp9.hw_compression_level;
            }
            Codec::Av1(av1) => {
                // Sync video_codec hint (will be refined by select_encoder)
                self.video_codec = if self.use_hardware_encoding {
                    "av1_qsv".to_string() // Placeholder - select_encoder will choose best
                } else {
                    "libsvtav1".to_string()
                };

                // Sync quality settings
                if self.use_hardware_encoding {
                    // Hardware AV1 uses hw_cq -> hw_global_quality
                    self.hw_global_quality = av1.hw_cq;
                } else {
                    // Software AV1: libsvtav1 doesn't use CRF in the same way
                    // Keep existing self.crf if set, otherwise use preset-based quality
                    // Note: libsvtav1 primarily uses preset + crf combination
                }
            }
        }
    }

    /// Determine the encoder ID that will be used for this profile
    ///
    /// Used for PARAMS validation to determine which encoder-specific
    /// parameter ranges to check against.
    #[cfg(feature = "dev-tools")]
    pub fn resolved_encoder_id(&self) -> String {
        match &self.codec {
            Codec::Vp9(_) => {
                if self.use_hardware_encoding {
                    // Check video_codec hint to determine QSV vs VAAPI
                    match self.video_codec.as_str() {
                        "vp9_qsv" => "vp9_qsv",
                        _ => "vp9_vaapi", // Default to VAAPI for VP9 hardware
                    }
                } else {
                    "libvpx-vp9"
                }
            }
            Codec::Av1(_) => {
                if self.use_hardware_encoding {
                    // Check video_codec hint to determine encoder
                    match self.video_codec.as_str() {
                        "av1_nvenc" => "av1_nvenc",
                        "av1_vaapi" => "av1_vaapi",
                        "av1_amf" => "av1_amf",
                        _ => "av1_qsv", // Default to QSV for AV1 hardware
                    }
                } else {
                    "libsvtav1"
                }
            }
        }
        .to_string()
    }

    /// Save profile to JSON file
    pub fn save(&self, profiles_dir: &Path) -> io::Result<()> {
        use std::fs;

        fs::create_dir_all(profiles_dir)?;
        let filename = format!("{}.json", self.name.to_lowercase().replace(' ', "_"));
        let path = profiles_dir.join(filename);

        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;

        Ok(())
    }

    /// Load profile from JSON file
    pub fn load(profiles_dir: &Path, name: &str) -> io::Result<Self> {
        use std::fs;

        let filename = format!("{}.json", name.to_lowercase().replace(' ', "_"));
        let path = profiles_dir.join(filename);

        let json = fs::read_to_string(path)?;
        let mut profile: Self = serde_json::from_str(&json)?;

        // Synchronize legacy fields from codec configuration
        // This fixes profiles that have stale video_codec/crf values
        profile.sync_legacy_fields();

        // [Phase 4] Validate and clamp parameters when dev-tools enabled
        #[cfg(feature = "dev-tools")]
        {
            use crate::engine::params::validate_and_clamp_profile;

            let encoder_id = profile.resolved_encoder_id();
            let clamps = validate_and_clamp_profile(&mut profile, &encoder_id);

            // Parameters clamped silently - validation still occurs, just no console output
            let _ = clamps;
        }

        Ok(profile)
    }

    /// List all saved profiles
    pub fn list_saved(profiles_dir: &Path) -> io::Result<Vec<String>> {
        use std::fs;

        if !profiles_dir.exists() {
            return Ok(Vec::new());
        }

        let mut profiles = Vec::new();
        for entry in fs::read_dir(profiles_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                // Read the JSON file to get the actual profile name
                if let Ok(json) = fs::read_to_string(&path) {
                    if let Ok(profile) = serde_json::from_str::<Profile>(&json) {
                        profiles.push(profile.name.clone());
                    }
                }
            }
        }

        Ok(profiles)
    }

    /// Delete a saved profile
    pub fn delete(profiles_dir: &Path, name: &str) -> io::Result<()> {
        use std::fs;

        let filename = format!("{}.json", name.to_lowercase().replace(' ', "_"));
        let path = profiles_dir.join(filename);

        fs::remove_file(path)?;

        Ok(())
    }
}

/// Derive output path from input path and profile
/// Default format: <basename>.<container>
/// Example: movie.mp4 -> movie.webm
///
/// If profile has custom filename pattern, applies template variable substitution:
/// - {basename}: input filename without extension (e.g., "video")
/// - {filename}: full input filename with extension (e.g., "video.mp4")
/// - {profile}: profile suffix (e.g., "vp9good")
/// - {ext}: output container extension (e.g., "webm")
///
/// Examples:
/// - Append: `{filename}_converted` → video.mp4 → video.mp4_converted.webm
/// - Prepend: `encoded_{filename}` → video.mp4 → encoded_video.mp4.webm
/// - Just basename: `{basename}` → video.mp4 → video.webm
pub fn derive_output_path(
    input_path: &Path,
    profile: &str,
    custom_output_dir: Option<&str>,
    custom_pattern: Option<&str>,
    custom_container: Option<&str>,
) -> std::path::PathBuf {
    let profile_obj = Profile::get(profile);

    // Use custom output directory if provided, otherwise use input file's directory
    let output_dir = if let Some(dir) = custom_output_dir {
        Path::new(dir)
    } else {
        input_path.parent().unwrap_or_else(|| Path::new("."))
    };

    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let original_filename = input_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    // Use custom container or profile's default
    let container = custom_container.unwrap_or(&profile_obj.container);

    // Use custom pattern (filename_pattern is now a global setting, not part of profiles)
    let filename = if let Some(pat) = custom_pattern {
        // Custom template-based filename transformation
        let result = pat
            .replace("{basename}", stem)
            .replace("{filename}", original_filename)
            .replace("{profile}", &profile_obj.suffix)
            .replace("{ext}", container);

        // Add extension
        format!("{}.{}", result, container)
    } else {
        // Default behavior: <basename>.<container>
        format!("{}.{}", stem, container)
    };

    output_dir.join(filename)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_legacy_fields_av1_hardware() {
        // Create a profile with mismatched legacy fields (simulating user's bug)
        let mut profile = Profile {
            name: "Test".to_string(),
            video_codec: "libvpx-vp9".to_string(), // WRONG: VP9 encoder for AV1 codec
            crf: 37, // STALE: Old value
            hw_global_quality: 70, // STALE: Old value
            use_hardware_encoding: true,
            codec: Codec::Av1(Av1Config {
                hw_cq: 105, // CORRECT: User's intended quality
                preset: 8,
                tune: 0,
                film_grain: 0,
                enable_overlays: true,
                scd: true,
                scm: 2,
                enable_tf: true,
                hw_preset: "4".to_string(),
                hw_lookahead: 40,
                hw_tile_cols: 0,
                hw_tile_rows: 0,
                ..Default::default()
            }),
            ..Profile::get("av1-qsv") // Use defaults for other fields
        };

        // Apply sync
        profile.sync_legacy_fields();

        // Verify sync worked
        assert_eq!(
            profile.hw_global_quality, 105,
            "hw_global_quality should be synced from codec.hw_cq"
        );
        assert!(
            profile.video_codec.contains("av1"),
            "video_codec should be set to an AV1 encoder variant, got: {}",
            profile.video_codec
        );
    }

    #[test]
    fn test_sync_legacy_fields_vp9_hardware() {
        let mut profile = Profile {
            name: "Test VP9".to_string(),
            use_hardware_encoding: true,
            codec: Codec::Vp9(Vp9Config {
                hw_global_quality: 80,
                ..Vp9Config::default()
            }),
            ..Profile::get("vp9-vaapi-streaming")
        };

        profile.sync_legacy_fields();

        assert_eq!(
            profile.hw_global_quality, 80,
            "hw_global_quality should be synced from Vp9Config"
        );
    }
}
