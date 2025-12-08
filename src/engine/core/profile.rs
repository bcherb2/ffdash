use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

// Default values for hardware encoding profile fields (backward compatibility)
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
    4
} // ICQ mode (best quality/size ratio)
fn default_hw_compression_level() -> u32 {
    4
} // Balanced speed/compression

/// Encoding profile configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub suffix: String,
    pub container: String,
    pub video_codec: String,
    pub audio_codec: String,
    pub audio_bitrate: u32,
    #[serde(default)]
    pub downmix_stereo: bool,

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
    pub gop_length: u32,
    pub keyint_min: u32,
    pub fixed_gop: bool,
    pub lag_in_frames: u32,
    pub auto_alt_ref: bool,

    // Alt-ref denoising (ARNR)
    pub arnr_max_frames: u32,
    pub arnr_strength: u32,
    pub arnr_type: i32,

    // Advanced tuning
    pub enable_tpl: bool,
    pub sharpness: i32,
    pub noise_sensitivity: u32,
    pub static_thresh: u32,
    pub max_intra_rate: u32,
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
}

/// Configuration for VAAPI hardware encoding
#[derive(Debug, Clone)]
pub struct HwEncodingConfig {
    /// Rate control mode: 1=CQP (Constant Quality), 2=CBR (Constant Bitrate),
    /// 3=VBR (Variable Bitrate), 4=ICQ (Intelligent Constant Quality)
    /// Default: 4 (ICQ - best quality/size ratio)
    pub rc_mode: u32,

    /// Quality setting (1-255): Lower = better quality/bigger files, Higher = worse quality/smaller files
    /// This value is passed DIRECTLY to FFmpeg's -global_quality parameter (no mapping)
    /// Recommended: 40=high quality, 70=good quality, 100=medium, 120+=low quality/small files
    /// Only used with CQP (rc_mode=1) or ICQ (rc_mode=4)
    pub global_quality: u32,

    /// Number of B-frames (0-4): Higher = better compression but slower
    /// 0 = no B-frames (safest for Intel Arc), 1 = moderate compression
    /// Requires bitstream filters when > 0
    pub b_frames: u32,

    /// Loop filter level (0-63): Controls deblocking filter strength
    /// Lower = more detail/blockier, Higher = smoother/less detail
    /// Default: 16
    pub loop_filter_level: u32,

    /// Loop filter sharpness (0-15): Controls edge filtering aggressiveness
    /// Lower = gentler, Higher = sharper edges
    /// Default: 4
    pub loop_filter_sharpness: u32,

    /// Compression level (0-7): Speed vs compression tradeoff
    /// 0 = fastest/least compression, 7 = slowest/most compression
    /// Default: 4 (balanced)
    pub compression_level: u32,
}

impl Default for HwEncodingConfig {
    fn default() -> Self {
        Self {
            rc_mode: 4,               // ICQ mode (best quality/size ratio)
            global_quality: 70,       // Good quality (balanced)
            b_frames: 0,              // No B-frames (safest for Intel Arc)
            loop_filter_level: 16,    // Default VP9 loop filter level
            loop_filter_sharpness: 4, // Default VP9 loop filter sharpness
            compression_level: 4,     // Balanced speed/compression
        }
    }
}

impl Profile {
    /// Get built-in profile by name
    pub fn get(name: &str) -> Self {
        match name {
            "vp9-good" => Self {
                name: "vp9-good".to_string(),
                suffix: "vp9good".to_string(),
                container: "webm".to_string(),
                video_codec: "libvpx-vp9".to_string(),
                audio_codec: "libopus".to_string(),
                audio_bitrate: 128,
                downmix_stereo: false,
                fps: 0,           // Source
                scale_width: -2,  // Source
                scale_height: -2, // Source
                crf: 31,
                video_target_bitrate: 0,
                video_min_bitrate: 0,
                video_max_bitrate: 0,
                video_bufsize: 0,
                undershoot_pct: -1,
                overshoot_pct: -1,
                cpu_used: 2,
                cpu_used_pass1: 4,
                cpu_used_pass2: 1,
                two_pass: false,
                quality_mode: "good".to_string(),
                vp9_profile: 0,
                pix_fmt: "yuv420p".to_string(),
                row_mt: true,
                tile_columns: 2,
                tile_rows: 0,
                threads: 0,
                frame_parallel: false,
                max_workers: 1, // Conservative default - sequential processing
                gop_length: 240,
                keyint_min: 0,
                fixed_gop: false,
                lag_in_frames: 25,
                auto_alt_ref: true,
                arnr_max_frames: 7,
                arnr_strength: 3,
                arnr_type: -1,
                enable_tpl: true,
                sharpness: -1,
                noise_sensitivity: 0,
                static_thresh: 0,
                max_intra_rate: 0,
                aq_mode: 1, // Variance AQ
                tune_content: "default".to_string(),
                colorspace: -1,
                color_primaries: -1,
                color_trc: -1,
                color_range: -1,
                // Hardware encoding (default to software)
                use_hardware_encoding: false,
                hw_rc_mode: 4, // ICQ
                hw_global_quality: 70,
                hw_b_frames: 0,
                hw_loop_filter_level: 16,
                hw_loop_filter_sharpness: 4,
                hw_compression_level: 4,
            },
            "vp9-best" => Self {
                name: "vp9-best".to_string(),
                suffix: "vp9best".to_string(),
                container: "webm".to_string(),
                video_codec: "libvpx-vp9".to_string(),
                audio_codec: "libopus".to_string(),
                audio_bitrate: 128,
                downmix_stereo: false,
                fps: 0,           // Source
                scale_width: -2,  // Source
                scale_height: -2, // Source
                crf: 24,
                video_target_bitrate: 0,
                video_min_bitrate: 0,
                video_max_bitrate: 0,
                video_bufsize: 0,
                undershoot_pct: -1,
                overshoot_pct: -1,
                cpu_used: 0,
                cpu_used_pass1: 4,
                cpu_used_pass2: 0,
                two_pass: true,
                quality_mode: "good".to_string(),
                vp9_profile: 0,
                pix_fmt: "yuv420p".to_string(),
                row_mt: true,
                tile_columns: 2,
                tile_rows: 0,
                threads: 0,
                frame_parallel: false,
                max_workers: 1, // Conservative default - sequential processing
                gop_length: 240,
                keyint_min: 0,
                fixed_gop: false,
                lag_in_frames: 25,
                auto_alt_ref: true,
                arnr_max_frames: 7,
                arnr_strength: 3,
                arnr_type: -1,
                enable_tpl: true,
                sharpness: -1,
                noise_sensitivity: 0,
                static_thresh: 0,
                max_intra_rate: 0,
                aq_mode: 1,
                tune_content: "default".to_string(),
                colorspace: -1,
                color_primaries: -1,
                color_trc: -1,
                color_range: -1,
                // Hardware encoding (default to software)
                use_hardware_encoding: false,
                hw_rc_mode: 4, // ICQ
                hw_global_quality: 70,
                hw_b_frames: 0,
                hw_loop_filter_level: 16,
                hw_loop_filter_sharpness: 4,
                hw_compression_level: 4,
            },
            "vp9-fast-preview" => Self {
                name: "vp9-fast-preview".to_string(),
                suffix: "vp9fast".to_string(),
                container: "webm".to_string(),
                video_codec: "libvpx-vp9".to_string(),
                audio_codec: "libopus".to_string(),
                audio_bitrate: 96,
                downmix_stereo: false,
                fps: 0,           // Source
                scale_width: -2,  // Source
                scale_height: -2, // Source
                crf: 40,
                video_target_bitrate: 0,
                video_min_bitrate: 0,
                video_max_bitrate: 0,
                video_bufsize: 0,
                undershoot_pct: -1,
                overshoot_pct: -1,
                cpu_used: 5,
                cpu_used_pass1: 5,
                cpu_used_pass2: 5,
                two_pass: false,
                quality_mode: "good".to_string(),
                vp9_profile: 0,
                pix_fmt: "yuv420p".to_string(),
                row_mt: true,
                tile_columns: 2,
                tile_rows: 0,
                threads: 0,
                frame_parallel: false,
                max_workers: 1, // Conservative default - sequential processing
                gop_length: 240,
                keyint_min: 0,
                fixed_gop: false,
                lag_in_frames: 25,
                auto_alt_ref: true,
                arnr_max_frames: 7,
                arnr_strength: 3,
                arnr_type: -1,
                enable_tpl: false,
                sharpness: -1,
                noise_sensitivity: 0,
                static_thresh: 0,
                max_intra_rate: 0,
                aq_mode: 1, // Variance AQ (community default)
                tune_content: "default".to_string(),
                colorspace: -1,
                color_primaries: -1,
                color_trc: -1,
                color_range: -1,
                // Hardware encoding (default to software)
                use_hardware_encoding: false,
                hw_rc_mode: 4, // ICQ
                hw_global_quality: 70,
                hw_b_frames: 0,
                hw_loop_filter_level: 16,
                hw_loop_filter_sharpness: 4,
                hw_compression_level: 4,
            },
            _ => Self::get("vp9-good"), // default
        }
    }

    /// List built-in profile names (user-friendly display names)
    pub fn builtin_names() -> Vec<String> {
        vec![
            "1080p Shrinker".to_string(),
            "Efficient 4K".to_string(),
            "Daily Driver".to_string(),
        ]
    }

    /// Get built-in profile by user-friendly name
    pub fn get_builtin(name: &str) -> Option<Self> {
        match name {
            "1080p Shrinker" => Some(Self {
                name: "1080p Shrinker".to_string(),
                suffix: "1080p_shrinker".to_string(),
                container: "webm".to_string(),
                video_codec: "libvpx-vp9".to_string(),
                audio_codec: "libopus".to_string(),
                audio_bitrate: 64, // Low bitrate for maximum space savings
                downmix_stereo: false,
                fps: 30,            // Limit to 30fps for space savings
                scale_width: 1920,  // 1080p width
                scale_height: 1080, // 1080p height
                crf: 37,            // Aggressive compression for maximum space savings
                video_target_bitrate: 0,
                video_min_bitrate: 0,
                video_max_bitrate: 0,
                video_bufsize: 0,
                undershoot_pct: -1,
                overshoot_pct: -1,
                cpu_used: 2, // Slower encoding for 5-10% more size reduction
                cpu_used_pass1: 4,
                cpu_used_pass2: 2,
                two_pass: false, // CQ mode - single pass is sufficient
                quality_mode: "good".to_string(),
                vp9_profile: 0,
                pix_fmt: "yuv420p".to_string(),
                row_mt: true,    // ESSENTIAL: 30-50% speedup with no quality loss
                tile_columns: 2, // Standard for 1080p
                tile_rows: 0,
                threads: 0, // Auto - let encoder manage based on tiles
                frame_parallel: true,
                max_workers: 1, // Conservative default - sequential processing
                gop_length: 240,
                keyint_min: 0,
                fixed_gop: false,
                lag_in_frames: 25,
                auto_alt_ref: true,
                arnr_max_frames: 7,
                arnr_strength: 3,
                arnr_type: -1,
                enable_tpl: true, // Quality improvement
                sharpness: -1,
                noise_sensitivity: 0,
                static_thresh: 0,
                max_intra_rate: 0,
                aq_mode: 1, // Variance AQ (community default)
                tune_content: "default".to_string(),
                colorspace: -1,
                color_primaries: -1,
                color_trc: -1,
                color_range: -1,
                // Hardware encoding (default to software)
                use_hardware_encoding: false,
                hw_rc_mode: 4, // ICQ
                hw_global_quality: 70,
                hw_b_frames: 0,
                hw_loop_filter_level: 16,
                hw_loop_filter_sharpness: 4,
                hw_compression_level: 4,
            }),
            "Efficient 4K" => Some(Self {
                name: "Efficient 4K".to_string(),
                suffix: "efficient_4k".to_string(),
                container: "webm".to_string(),
                video_codec: "libvpx-vp9".to_string(),
                audio_codec: "libopus".to_string(),
                audio_bitrate: 128, // Higher quality audio for 4K content
                downmix_stereo: false,
                fps: 0,             // Source
                scale_width: 3840,  // 4K width
                scale_height: 2160, // 4K height
                crf: 26,            // Higher quality to preserve 4K details
                video_target_bitrate: 0,
                video_min_bitrate: 0,
                video_max_bitrate: 0,
                video_bufsize: 0,
                undershoot_pct: -1,
                overshoot_pct: -1,
                cpu_used: 4, // Speed 4 is necessary compromise for 4K
                cpu_used_pass1: 4,
                cpu_used_pass2: 4,
                two_pass: false, // CQ mode - single pass is sufficient
                quality_mode: "good".to_string(),
                vp9_profile: 0,
                pix_fmt: "yuv420p".to_string(),
                row_mt: true,    // ESSENTIAL: 30-50% speedup with no quality loss
                tile_columns: 3, // For 4K: 2^3 = 8 columns (3840px / 8 = 480px per tile)
                tile_rows: 0,
                threads: 0, // Auto - let encoder manage based on tiles
                frame_parallel: true,
                max_workers: 1, // Conservative default - sequential processing
                gop_length: 240,
                keyint_min: 0,
                fixed_gop: false,
                lag_in_frames: 25,
                auto_alt_ref: true,
                arnr_max_frames: 7,
                arnr_strength: 3,
                arnr_type: -1,
                enable_tpl: true, // Quality improvement
                sharpness: -1,
                noise_sensitivity: 0,
                static_thresh: 0,
                max_intra_rate: 0,
                aq_mode: 1, // Variance AQ (community default)
                tune_content: "default".to_string(),
                colorspace: -1,
                color_primaries: -1,
                color_trc: -1,
                color_range: -1,
                // Hardware encoding (default to software)
                use_hardware_encoding: false,
                hw_rc_mode: 4, // ICQ
                hw_global_quality: 70,
                hw_b_frames: 0,
                hw_loop_filter_level: 16,
                hw_loop_filter_sharpness: 4,
                hw_compression_level: 4,
            }),
            "Daily Driver" => Some(Self {
                name: "Daily Driver".to_string(),
                suffix: "daily".to_string(),
                container: "webm".to_string(),
                video_codec: "libvpx-vp9".to_string(),
                audio_codec: "libopus".to_string(),
                audio_bitrate: 96, // Transparent for most listeners
                downmix_stereo: false,
                fps: 0,           // Source
                scale_width: -2,  // Source
                scale_height: -2, // Source
                crf: 30,          // Visually clear but efficient (community sweet spot)
                video_target_bitrate: 0,
                video_min_bitrate: 0,
                video_max_bitrate: 0,
                video_bufsize: 0,
                undershoot_pct: -1,
                overshoot_pct: -1,
                cpu_used: 4, // Speed 3 and 4 look identical, but 4 is faster
                cpu_used_pass1: 4,
                cpu_used_pass2: 4,
                two_pass: false, // CQ mode - single pass is sufficient
                quality_mode: "good".to_string(),
                vp9_profile: 0,
                pix_fmt: "yuv420p".to_string(),
                row_mt: true,    // ESSENTIAL: 30-50% speedup with no quality loss
                tile_columns: 2, // Standard for 1080p
                tile_rows: 0,
                threads: 0, // Auto - let encoder manage based on tiles
                frame_parallel: true,
                max_workers: 1, // Conservative default - sequential processing
                gop_length: 240,
                keyint_min: 0,
                fixed_gop: false,
                lag_in_frames: 25,
                auto_alt_ref: true,
                arnr_max_frames: 7,
                arnr_strength: 3,
                arnr_type: -1,
                enable_tpl: true, // Quality improvement
                sharpness: -1,
                noise_sensitivity: 0,
                static_thresh: 0,
                max_intra_rate: 0,
                aq_mode: 1, // Variance AQ (community default)
                tune_content: "default".to_string(),
                colorspace: -1,
                color_primaries: -1,
                color_trc: -1,
                color_range: -1,
                // Hardware encoding (default to software)
                use_hardware_encoding: false,
                hw_rc_mode: 4, // ICQ
                hw_global_quality: 70,
                hw_b_frames: 0,
                hw_loop_filter_level: 16,
                hw_loop_filter_sharpness: 4,
                hw_compression_level: 4,
            }),
            _ => None,
        }
    }

    /// Create a Profile from ConfigState
    pub fn from_config(name: String, config: &crate::ui::state::ConfigState) -> Self {
        use crate::ui::constants::*;
        use crate::ui::state::RateControlMode;

        // Map list state selections to actual values
        let quality_mode_idx = config.quality_mode_state.selected().unwrap_or(0);
        let quality_mode = QUALITY_MODES
            .get(quality_mode_idx)
            .unwrap_or(&"good")
            .to_string();

        let vp9_profile = config.profile_dropdown_state.selected().unwrap_or(0) as u8;

        let pix_fmt_idx = config.pix_fmt_state.selected().unwrap_or(0);
        let pix_fmt = PIX_FMTS.get(pix_fmt_idx).unwrap_or(&"yuv420p").to_string();

        let aq_mode_idx = config.aq_mode_state.selected().unwrap_or(0);
        let aq_mode = match aq_mode_idx {
            0 => -1, // Auto
            1 => 0,  // Off
            2 => 2,  // Variance
            3 => 1,  // Complexity
            4 => 3,  // Cyclic
            5 => 4,  // 360 Video
            _ => 2,  // Default to Variance
        };

        let tune_content_idx = config.tune_content_state.selected().unwrap_or(0);
        let tune_content = TUNE_CONTENTS
            .get(tune_content_idx)
            .unwrap_or(&"default")
            .to_string();

        let audio_codec_idx = config.codec_list_state.selected().unwrap_or(0);
        let audio_codec = AUDIO_CODECS
            .get(audio_codec_idx)
            .unwrap_or(&"libopus")
            .to_string();

        // Map colorspace dropdown selections to ffmpeg values
        let colorspace_idx = config.colorspace_state.selected().unwrap_or(0);
        let colorspace = match colorspace_idx {
            0 => -1, // Auto
            1 => 1,  // BT709
            2 => 5,  // BT470BG
            3 => 6,  // SMPTE170M
            4 => 9,  // BT2020
            _ => -1,
        };

        let color_primaries_idx = config.color_primaries_state.selected().unwrap_or(0);
        let color_primaries = match color_primaries_idx {
            0 => -1, // Auto
            1 => 1,  // BT709
            2 => 4,  // BT470M
            3 => 5,  // BT470BG
            4 => 9,  // BT2020
            _ => -1,
        };

        let color_trc_idx = config.color_trc_state.selected().unwrap_or(0);
        let color_trc = match color_trc_idx {
            0 => -1, // Auto
            1 => 1,  // BT709
            2 => 6,  // SMPTE170M
            3 => 16, // SMPTE2084 (PQ)
            4 => 18, // ARIB-B67 (HLG)
            _ => -1,
        };

        let color_range_idx = config.color_range_state.selected().unwrap_or(0);
        let color_range = match color_range_idx {
            0 => -1, // Auto
            1 => 0,  // TV/Limited
            2 => 1,  // PC/Full
            _ => -1,
        };

        let arnr_type_idx = config.arnr_type_state.selected().unwrap_or(0);
        let arnr_type = match arnr_type_idx {
            0 => -1, // Auto
            1 => 1,  // Backward
            2 => 2,  // Forward
            3 => 3,  // Centered
            _ => -1,
        };

        // Use FPS value directly from config (not dropdown mapping)
        let fps = config.fps;

        // Use resolution values directly from config (not dropdown mapping)
        let scale_width = config.scale_width;
        let scale_height = config.scale_height;

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

        Self {
            name: name.clone(),
            suffix: name.to_lowercase().replace(' ', "_"),
            container: "webm".to_string(),
            video_codec: "libvpx-vp9".to_string(),
            audio_codec,
            audio_bitrate: config.audio_bitrate,
            downmix_stereo: config.force_stereo,

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
            quality_mode,

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
            gop_length: config.gop_length,
            keyint_min: config.keyint_min,
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
            static_thresh: config.static_thresh,
            max_intra_rate: config.max_intra_rate,
            aq_mode,
            tune_content,

            // Color / HDR
            colorspace,
            color_primaries,
            color_trc,
            color_range,

            // Hardware encoding settings
            use_hardware_encoding: config.use_hardware_encoding,
            hw_rc_mode: config.vaapi_rc_mode.parse().unwrap_or(4), // Default to ICQ
            hw_global_quality: config.qsv_global_quality,
            hw_b_frames: config.vaapi_b_frames.parse().unwrap_or(0),
            hw_loop_filter_level: config.vaapi_loop_filter_level.parse().unwrap_or(16),
            hw_loop_filter_sharpness: config.vaapi_loop_filter_sharpness.parse().unwrap_or(4),
            hw_compression_level: config.vaapi_compression_level.parse().unwrap_or(4),
        }
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
        config.audio_bitrate = self.audio_bitrate;
        config.force_stereo = self.downmix_stereo;

        // Video output constraints
        config.fps = self.fps;
        config.scale_width = self.scale_width;
        config.scale_height = self.scale_height;

        // Map FPS value to dropdown index
        let fps_idx = match self.fps {
            0 => 0,    // Source
            24 => 2,   // 24
            25 => 3,   // 25
            30 => 5,   // 30
            50 => 6,   // 50
            60 => 8,   // 60
            120 => 9,  // 120
            144 => 10, // 144
            _ => 0,    // Default to Source
        };
        config.fps_dropdown_state.select(Some(fps_idx));

        // Map resolution to dropdown index
        let res_idx = match (self.scale_width, self.scale_height) {
            (-2, -2) => 0,     // Source
            (640, 360) => 1,   // 360p
            (854, 480) => 2,   // 480p
            (1280, 720) => 3,  // 720p
            (1920, 1080) => 4, // 1080p
            (2560, 1440) => 5, // 1440p
            (3840, 2160) => 6, // 2160p/4K
            _ => 0,            // Default to Source
        };
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
        config.gop_length = self.gop_length;
        config.keyint_min = self.keyint_min;
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
        config.static_thresh = self.static_thresh;
        config.max_intra_rate = self.max_intra_rate;

        // Map Profile values back to ListState selections

        // Quality mode: "good", "realtime", "best" → 0, 1, 2
        let quality_idx = match self.quality_mode.as_str() {
            "good" => 0,
            "realtime" => 1,
            "best" => 2,
            _ => 0,
        };
        config.quality_mode_state.select(Some(quality_idx));

        // VP9 profile: u8 → index
        config
            .profile_dropdown_state
            .select(Some(self.vp9_profile as usize));

        // Pixel format: "yuv420p", "yuv420p10le" → 0, 1
        let pix_fmt_idx = match self.pix_fmt.as_str() {
            "yuv420p" => 0,
            "yuv420p10le" => 1,
            _ => 0,
        };
        config.pix_fmt_state.select(Some(pix_fmt_idx));

        // AQ mode: ffmpeg value → index
        let aq_idx = match self.aq_mode {
            -1 => 0, // Auto
            0 => 1,  // Off
            2 => 2,  // Variance
            1 => 3,  // Complexity
            3 => 4,  // Cyclic
            4 => 5,  // 360 Video
            _ => 2,  // Default to Variance
        };
        config.aq_mode_state.select(Some(aq_idx));

        // Audio codec: string → index
        let audio_idx = match self.audio_codec.as_str() {
            "libopus" => 0,
            "aac" => 1,
            "mp3" => 2,
            "vorbis" => 3,
            _ => 0,
        };
        config.codec_list_state.select(Some(audio_idx));

        // Tune content: string → index
        let tune_idx = match self.tune_content.as_str() {
            "default" => 0,
            "screen" => 1,
            "film" => 2,
            _ => 0,
        };
        config.tune_content_state.select(Some(tune_idx));

        // Colorspace: ffmpeg value → index
        let colorspace_idx = match self.colorspace {
            -1 => 0, // Auto
            1 => 1,  // BT709
            5 => 2,  // BT470BG
            6 => 3,  // SMPTE170M
            9 => 4,  // BT2020
            _ => 0,
        };
        config.colorspace_state.select(Some(colorspace_idx));

        // Color primaries: ffmpeg value → index
        let primaries_idx = match self.color_primaries {
            -1 => 0, // Auto
            1 => 1,  // BT709
            4 => 2,  // BT470M
            5 => 3,  // BT470BG
            9 => 4,  // BT2020
            _ => 0,
        };
        config.color_primaries_state.select(Some(primaries_idx));

        // Color TRC: ffmpeg value → index
        let trc_idx = match self.color_trc {
            -1 => 0, // Auto
            1 => 1,  // BT709
            6 => 2,  // SMPTE170M
            16 => 3, // SMPTE2084 (PQ)
            18 => 4, // ARIB-B67 (HLG)
            _ => 0,
        };
        config.color_trc_state.select(Some(trc_idx));

        // Color range: ffmpeg value → index
        let range_idx = match self.color_range {
            -1 => 0, // Auto
            0 => 1,  // TV/Limited
            1 => 2,  // PC/Full
            _ => 0,
        };
        config.color_range_state.select(Some(range_idx));

        // ARNR type: ffmpeg value → index
        let arnr_type_idx = match self.arnr_type {
            -1 => 0, // Auto
            1 => 1,  // Backward
            2 => 2,  // Forward
            3 => 3,  // Centered
            _ => 0,
        };
        config.arnr_type_state.select(Some(arnr_type_idx));

        // Hardware encoding settings
        config.use_hardware_encoding = self.use_hardware_encoding;
        config.vaapi_rc_mode = self.hw_rc_mode.to_string();
        config.qsv_global_quality = self.hw_global_quality;
        config.vaapi_b_frames = self.hw_b_frames.to_string();
        config.vaapi_loop_filter_level = self.hw_loop_filter_level.to_string();
        config.vaapi_loop_filter_sharpness = self.hw_loop_filter_sharpness.to_string();
        config.vaapi_compression_level = self.hw_compression_level.to_string();
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
        let profile: Self = serde_json::from_str(&json)?;

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
