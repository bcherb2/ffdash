// Application state management

use crate::engine::hardware::HwPreflightResult;
use crate::stats::StatsState;
use crate::ui::focus::ConfigFocus;
use crate::ui::help::HelpModalState;
use ratatui::{
    layout::Rect,
    widgets::{ListState, TableState},
};
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Instant;
use sysinfo::System;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Dashboard,
    Config,
    Stats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateControlMode {
    CQ,         // Constant quality (b:v 0)
    CQCap,      // CQ with maxrate cap
    TwoPassVBR, // Two-pass variable bitrate
    CBR,        // Constant bitrate (for live/streaming)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,  // Normal navigation mode - global shortcuts active
    Editing, // Text editing mode - character input active, global shortcuts inactive
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CodecSelection {
    #[default]
    Vp9,
    Av1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorSpacePreset {
    #[default]
    Auto,  // Passthrough: -1, -1, -1, -1
    Sdr,   // BT709: 1, 1, 1, 0
    Hdr10, // BT2020+PQ: 9, 9, 16, 0
}

/// Audio primary track codec selection
/// Passthrough copies audio without re-encoding, others transcode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AudioPrimaryCodec {
    #[default]
    Passthrough, // -c:a copy
    Opus, // libopus
    Aac,  // aac
    Mp3,  // mp3
    Vorbis, // vorbis
}

impl AudioPrimaryCodec {
    pub fn is_passthrough(&self) -> bool {
        matches!(self, Self::Passthrough)
    }

    pub fn ffmpeg_codec(&self) -> Option<&'static str> {
        match self {
            Self::Passthrough => None,
            Self::Opus => Some("libopus"),
            Self::Aac => Some("aac"),
            Self::Mp3 => Some("mp3"),
            Self::Vorbis => Some("vorbis"),
        }
    }

    pub fn from_index(idx: usize) -> Self {
        match idx {
            0 => Self::Passthrough,
            1 => Self::Opus,
            2 => Self::Aac,
            3 => Self::Mp3,
            4 => Self::Vorbis,
            _ => Self::Opus,
        }
    }

    pub fn to_index(self) -> usize {
        match self {
            Self::Passthrough => 0,
            Self::Opus => 1,
            Self::Aac => 2,
            Self::Mp3 => 3,
            Self::Vorbis => 4,
        }
    }
}

/// Audio stereo compatibility track codec (no passthrough option)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AudioStereoCodec {
    #[default]
    Aac,    // aac (best compatibility)
    Opus,   // libopus
}

impl AudioStereoCodec {
    pub fn ffmpeg_codec(&self) -> &'static str {
        match self {
            Self::Aac => "aac",
            Self::Opus => "libopus",
        }
    }

    pub fn from_index(idx: usize) -> Self {
        match idx {
            0 => Self::Aac,
            1 => Self::Opus,
            _ => Self::Aac,
        }
    }

    pub fn to_index(self) -> usize {
        match self {
            Self::Aac => 0,
            Self::Opus => 1,
        }
    }
}

/// State for the quit confirmation modal
#[derive(Debug, Clone)]
pub struct QuitConfirmationState {
    /// Number of encodes currently in progress
    pub running_count: usize,
}

pub struct AppState {
    pub current_screen: Screen,
    pub dashboard: DashboardState,
    pub config: ConfigState,
    pub stats: StatsState,
    pub last_metrics_update: Instant,
    pub viewport: Rect,
    pub worker_pool: Option<Rc<crate::engine::worker::WorkerPool>>,
    pub enc_state: Option<crate::engine::EncState>,
    pub root_path: Option<std::path::PathBuf>,
    pub help_modal: Option<HelpModalState>,
    pub quit_confirmation: Option<QuitConfirmationState>, // Quit confirmation modal
    pub app_version: String,
    pub ffmpeg_version: Option<String>,
    pub ffprobe_version: Option<String>,
    pub hw_preflight_result: Option<HwPreflightResult>,
    pub huc_available: Option<bool>, // HuC firmware status (for VBR/CBR modes)
    pub scan_in_progress: bool,      // True while initial scan is running
    pub pending_autostart: bool,     // True if we should autostart after scan completes
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_screen: Screen::Dashboard,
            dashboard: DashboardState::default(),
            config: ConfigState::default(),
            stats: StatsState::default(),
            last_metrics_update: Instant::now(),
            viewport: Rect::default(),
            worker_pool: None,        // Initialized when encoding starts
            enc_state: None,          // Initialized when encoding starts
            root_path: None,          // Set when user provides a directory to encode
            help_modal: None,         // Opened when 'H' key is pressed
            quit_confirmation: None,  // Opened when 'q' pressed with active encodes
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            ffmpeg_version: None,      // Cached when help is first opened
            ffprobe_version: None,     // Cached when help is first opened
            hw_preflight_result: None, // Cached when help is first opened
            huc_available: None,       // Checked when help is first opened
            scan_in_progress: false,
            pending_autostart: false,
        }
    }
}

pub struct DashboardState {
    pub cpu_data: VecDeque<u64>,
    pub mem_data: VecDeque<u64>,
    pub table_state: TableState,
    pub foreground_job_index: usize,
    pub system: System,

    // Mouse support
    pub table_area: Option<Rect>,
    pub table_inner_area: Option<Rect>,
    pub hovered_row: Option<usize>,

    // Job data (if available)
    pub jobs: Vec<crate::engine::VideoJob>,

    // GPU monitoring
    pub gpu_data: VecDeque<u64>,     // GPU usage % ring buffer
    pub gpu_mem_data: VecDeque<u64>, // GPU memory % ring buffer
    pub gpu_available: bool,         // GPU monitoring tool detected
    pub gpu_model: Option<String>,   // e.g., "Intel Arc A770" or "NVIDIA GeForce RTX 4060"
    pub gpu_vendor: crate::engine::hardware::GpuVendor, // Detected GPU vendor

    // Uptime tracking
    pub start_time: Instant,
}

impl Default for DashboardState {
    fn default() -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        Self {
            cpu_data: VecDeque::with_capacity(240),
            mem_data: VecDeque::with_capacity(240),
            table_state,
            foreground_job_index: 0,
            system: System::new_all(),
            table_area: None,
            table_inner_area: None,
            hovered_row: None,
            jobs: Vec::new(),

            // GPU monitoring
            gpu_data: VecDeque::with_capacity(240),
            gpu_mem_data: VecDeque::with_capacity(240),
            gpu_available: false,
            gpu_model: None,
            gpu_vendor: crate::engine::hardware::GpuVendor::Unknown,

            start_time: Instant::now(),
        }
    }
}

pub struct ConfigState {
    pub focus: ConfigFocus,
    pub profile_list_state: ListState,
    pub quality_mode_state: ListState,
    pub profile_dropdown_state: ListState,
    pub pix_fmt_state: ListState,
    pub aq_mode_state: ListState,
    pub tune_content_state: ListState,
    pub colorspace_preset_state: ListState,
    pub arnr_type_state: ListState,
    pub fps_dropdown_state: ListState,
    pub resolution_dropdown_state: ListState,

    // Video codec selection (VP9 vs AV1)
    pub video_codec_state: ListState,
    pub codec_selection: CodecSelection,

    // AV1 software settings (libsvtav1)
    pub av1_preset: u32,           // 0-13, default 8
    pub av1_tune_state: ListState, // Visual Quality, SSIM, VMAF
    pub av1_film_grain: u32,       // 0-50, default 0
    pub av1_film_grain_denoise: bool, // denoise before grain synthesis
    pub av1_enable_overlays: bool,
    pub av1_scd: bool,            // Scene change detection
    pub av1_scm_state: ListState, // Screen content mode: Off, On, Auto
    pub av1_enable_tf: bool,      // Temporal filtering

    // AV1 hardware settings
    pub av1_hw_preset: u32, // Hardware preset: 1-7 (QSV numeric, NVENC adds 'p' prefix)
    pub av1_hw_cq: u32,     // Legacy: use per-encoder fields below

    // Per-encoder quality (AV1) - these take precedence over av1_hw_cq
    pub av1_svt_crf: u32,   // Software SVT-AV1: 0-63, lower=better
    pub av1_qsv_cq: u32,    // Intel QSV: 1-255, lower=better
    pub av1_nvenc_cq: u32,  // NVIDIA: 0-63, lower=better
    pub av1_vaapi_cq: u32,  // VAAPI: 1-255, lower=better

    pub av1_hw_lookahead: u32, // Lookahead frames
    pub av1_hw_tile_cols: u32, // Tile columns
    pub av1_hw_tile_rows: u32, // Tile rows

    // Profile tracking
    pub current_profile_name: Option<String>, // None = Custom
    pub is_modified: bool,                    // True if settings changed after profile load
    pub available_profiles: Vec<String>,      // Cached list of saved profiles

    // General settings
    pub output_dir: String,
    pub overwrite: bool,
    pub max_workers: u32, // Number of concurrent encoding jobs (1 = sequential)

    // Filename customization (template-based)
    // Supports: {filename}, {basename}, {profile}, {ext}
    pub filename_pattern: String,
    pub container_dropdown_state: ListState, // For selecting container extension

    // Additional FFmpeg arguments (appended to command before output file)
    pub additional_args: String,

    // Video output constraints (max FPS, max resolution)
    pub fps: u32,          // 0 = source (no limit), >0 = max fps cap
    pub scale_width: i32,  // -2 = source, -1 = auto, >0 = max width
    pub scale_height: i32, // -2 = source, -1 = auto, >0 = max height

    // Rate control
    pub rate_control_mode: RateControlMode,
    pub crf: u32,
    pub video_target_bitrate: u32,
    pub video_min_bitrate: u32,
    pub video_max_bitrate: u32,
    pub video_bufsize: u32,
    pub undershoot_pct: i32, // VBR undershoot % (-1 = auto, 0-100)
    pub overshoot_pct: i32,  // VBR overshoot % (-1 = auto, 0-1000)

    // Speed & quality
    pub cpu_used: u32,
    pub cpu_used_pass1: u32, // For 2-pass: Pass 1 speed (guide recommends 4)
    pub cpu_used_pass2: u32, // For 2-pass: Pass 2 speed (guide recommends 0-2)
    pub two_pass: bool,

    // Parallelism
    pub row_mt: bool,
    pub tile_columns: i32,
    pub tile_rows: i32,
    pub threads: u32,
    pub frame_parallel: bool,

    // GOP & keyframes
    pub gop_length: String,
    pub keyint_min: String, // Minimum keyframe interval ("0" = auto)
    pub fixed_gop: bool,
    pub lag_in_frames: u32,
    pub auto_alt_ref: u32,

    // Adaptive quantization
    // (aq_mode_state ListState above, selected index determines mode)

    // Alt-ref denoising (ARNR)
    pub arnr_max_frames: u32,
    pub arnr_strength: u32,
    pub arnr_type: i32, // -1=Auto, 1=Backward, 2=Forward, 3=Centered

    // Advanced tuning
    pub enable_tpl: bool,
    pub sharpness: i32,
    pub noise_sensitivity: u32,
    pub static_thresh: String, // Skip encoding blocks below this threshold ("0" = disabled)
    pub max_intra_rate: String, // Max I-frame bitrate percentage ("0" = disabled)

    // Color / HDR settings
    pub colorspace: i32,      // -1 = Auto, or specific colorspace value
    pub color_primaries: i32, // -1 = Auto, or specific primaries value
    pub color_trc: i32,       // -1 = Auto (transfer characteristics)
    pub color_range: i32,     // -1 = Auto, 0 = TV/limited, 1 = PC/full
    pub colorspace_preset: ColorSpacePreset, // UI preset combining the 4 above

    // Audio settings - multi-track support
    // Primary track: passthrough or transcode
    pub audio_primary_codec: AudioPrimaryCodec,
    pub audio_primary_codec_state: ListState, // For dropdown UI
    pub audio_primary_bitrate: u32,           // Ignored when passthrough
    pub audio_primary_downmix: bool,          // Downmix to stereo (2ch)

    // Compatibility track: AC3 5.1 for legacy receivers
    pub audio_add_ac3: bool,
    pub audio_ac3_bitrate: u32, // 384-640, default 448

    // Compatibility track: Stereo for mobile/web
    pub audio_add_stereo: bool,
    pub audio_stereo_codec: AudioStereoCodec,
    pub audio_stereo_codec_state: ListState, // For dropdown UI
    pub audio_stereo_bitrate: u32,           // 64-256, default 128

    // Hardware encoding settings (Intel Arc VAAPI)
    pub use_hardware_encoding: bool,
    pub hw_encoding_available: Option<bool>, // None=unchecked, Some=result
    pub hw_availability_message: Option<String>,
    pub gpu_vendor: crate::engine::hardware::GpuVendor, // Detected GPU vendor
    pub vaapi_rc_mode: String, // 1=CQP only (ICQ/VBR/CBR removed due to Arc driver bugs)
    pub qsv_global_quality: u32, // 1-255 (lower=better quality/larger files, higher=worse quality/smaller files), default 70
    pub vaapi_compression_level: String, // 0-7 (0=fastest, 7=slowest/best), default "4"
    pub vaapi_b_frames: String,  // 0-4, default "0"
    pub vaapi_loop_filter_level: String, // 0-63, default "16"
    pub vaapi_loop_filter_sharpness: String, // 0-15, default "4"

    // Hardware VPP filters (QSV: 0-100, VAAPI: 0-64)
    pub hw_denoise: String, // 0=off
    pub hw_detail: String,  // 0=off (sharpening)

    // VP9 QSV-specific controls (only apply when `vp9_qsv` is used)
    pub vp9_qsv_preset: u32, // 1-7 (1=best quality, 7=fastest)
    pub vp9_qsv_lookahead: bool,
    pub vp9_qsv_lookahead_depth: u32,

    // Auto-VAMF settings (quality calibration)
    pub auto_vmaf_enabled: bool,
    pub auto_vmaf_target: String, // Target VMAF score (e.g., "93.0")
    pub auto_vmaf_step: String,   // Quality step size per iteration (e.g., "2")
    pub auto_vmaf_max_attempts: String, // Maximum calibration attempts (e.g., "3")

    // Popup dropdown state
    pub active_dropdown: Option<ConfigFocus>,

    // Profile name input dialog (None = closed, Some(String) = open with current input)
    pub name_input_dialog: Option<String>,

    // Text input mode
    pub input_mode: InputMode,

    // Cursor position for text input fields (position in characters)
    pub cursor_pos: usize,

    // Mouse support - store widget areas
    pub overwrite_checkbox_area: Option<Rect>,
    pub two_pass_checkbox_area: Option<Rect>,
    pub row_mt_checkbox_area: Option<Rect>,
    pub frame_parallel_checkbox_area: Option<Rect>,
    pub fixed_gop_checkbox_area: Option<Rect>,
    pub auto_alt_ref_checkbox_area: Option<Rect>,
    pub enable_tpl_checkbox_area: Option<Rect>,
    pub save_button_area: Option<Rect>,
    pub delete_button_area: Option<Rect>,
    pub output_dir_area: Option<Rect>,
    pub filename_pattern_area: Option<Rect>,
    pub container_dropdown_area: Option<Rect>,
    pub fps_area: Option<Rect>,
    pub scale_width_area: Option<Rect>,
    pub scale_height_area: Option<Rect>,
    pub profile_list_area: Option<Rect>,
    pub quality_mode_area: Option<Rect>,
    pub vp9_profile_list_area: Option<Rect>,
    pub pix_fmt_area: Option<Rect>,
    pub aq_mode_area: Option<Rect>,
    pub tune_content_area: Option<Rect>,
    pub rate_control_mode_area: Option<Rect>,
    pub crf_slider_area: Option<Rect>,
    pub cpu_used_slider_area: Option<Rect>,
    pub cpu_used_pass1_slider_area: Option<Rect>,
    pub cpu_used_pass2_slider_area: Option<Rect>,
    pub tile_columns_slider_area: Option<Rect>,
    pub tile_rows_slider_area: Option<Rect>,
    pub threads_area: Option<Rect>,
    pub max_workers_area: Option<Rect>,
    pub gop_length_area: Option<Rect>,
    pub keyint_min_area: Option<Rect>,
    pub lag_in_frames_slider_area: Option<Rect>,
    pub arnr_max_frames_slider_area: Option<Rect>,
    pub arnr_strength_slider_area: Option<Rect>,
    pub sharpness_slider_area: Option<Rect>,
    pub noise_sensitivity_slider_area: Option<Rect>,
    pub video_target_bitrate_area: Option<Rect>,
    pub video_min_bitrate_area: Option<Rect>,
    pub video_max_bitrate_area: Option<Rect>,
    pub video_bufsize_area: Option<Rect>,
    // Audio areas
    pub audio_primary_codec_area: Option<Rect>,
    pub audio_primary_bitrate_area: Option<Rect>,
    pub audio_primary_downmix_area: Option<Rect>,
    pub audio_ac3_checkbox_area: Option<Rect>,
    pub audio_ac3_bitrate_area: Option<Rect>,
    pub audio_stereo_checkbox_area: Option<Rect>,
    pub audio_stereo_codec_area: Option<Rect>,
    pub audio_stereo_bitrate_area: Option<Rect>,
    pub colorspace_preset_area: Option<Rect>,
    pub arnr_type_area: Option<Rect>,
    pub static_thresh_area: Option<Rect>,
    pub max_intra_rate_area: Option<Rect>,
    pub undershoot_pct_area: Option<Rect>,
    pub overshoot_pct_area: Option<Rect>,

    // Hardware encoding areas
    pub hw_encoding_checkbox_area: Option<Rect>,
    pub qsv_quality_slider_area: Option<Rect>,
    pub vaapi_compression_level_slider_area: Option<Rect>,
    pub vaapi_b_frames_area: Option<Rect>,
    pub vaapi_loop_filter_level_area: Option<Rect>,
    pub vaapi_loop_filter_sharpness_area: Option<Rect>,
    pub hw_denoise_area: Option<Rect>,
    pub hw_detail_area: Option<Rect>,

    // Video codec selector area
    pub video_codec_area: Option<Rect>,

    // AV1 software areas
    pub av1_preset_slider_area: Option<Rect>,
    pub av1_tune_area: Option<Rect>,
    pub av1_film_grain_slider_area: Option<Rect>,
    pub av1_film_grain_denoise_checkbox_area: Option<Rect>,
    pub av1_enable_overlays_checkbox_area: Option<Rect>,
    pub av1_scd_checkbox_area: Option<Rect>,
    pub av1_scm_area: Option<Rect>,
    pub av1_enable_tf_checkbox_area: Option<Rect>,

    // AV1 hardware areas
    pub av1_hw_preset_area: Option<Rect>,
    pub av1_hw_cq_slider_area: Option<Rect>,
    pub av1_hw_lookahead_area: Option<Rect>,
    pub av1_hw_tile_cols_area: Option<Rect>,
    pub av1_hw_tile_rows_area: Option<Rect>,

    // VP9 QSV areas
    pub vp9_qsv_preset_area: Option<Rect>,
    pub vp9_qsv_lookahead_checkbox_area: Option<Rect>,
    pub vp9_qsv_lookahead_depth_area: Option<Rect>,

    // Auto-VMAF areas
    pub auto_vmaf_checkbox_area: Option<Rect>,
    pub auto_vmaf_target_area: Option<Rect>,
    pub auto_vmaf_step_area: Option<Rect>,
    pub auto_vmaf_max_attempts_area: Option<Rect>,

    // Additional args area
    pub additional_args_area: Option<Rect>,

    // Status message (message text, timestamp when shown)
    pub status_message: Option<(String, Instant)>,
}

impl Default for ConfigState {
    fn default() -> Self {
        let mut profile_list_state = ListState::default();
        profile_list_state.select(Some(0));

        let mut quality_mode_state = ListState::default();
        quality_mode_state.select(Some(0)); // good

        let mut profile_dropdown_state = ListState::default();
        profile_dropdown_state.select(Some(0)); // Profile 0 (8-bit)

        let mut pix_fmt_state = ListState::default();
        pix_fmt_state.select(Some(0)); // yuv420p (8-bit)

        let mut aq_mode_state = ListState::default();
        aq_mode_state.select(Some(2)); // Variance AQ (recommended for VOD)

        let mut tune_content_state = ListState::default();
        tune_content_state.select(Some(0)); // default

        let mut colorspace_preset_state = ListState::default();
        colorspace_preset_state.select(Some(0)); // Auto preset

        let mut arnr_type_state = ListState::default();
        arnr_type_state.select(Some(0)); // Auto

        let mut fps_dropdown_state = ListState::default();
        fps_dropdown_state.select(Some(0)); // Source

        let mut resolution_dropdown_state = ListState::default();
        resolution_dropdown_state.select(Some(0)); // Source

        let mut container_dropdown_state = ListState::default();
        container_dropdown_state.select(Some(0)); // webm

        // Video codec selection (AV1 by default)
        let mut video_codec_state = ListState::default();
        video_codec_state.select(Some(1)); // AV1

        // AV1 software settings
        let mut av1_tune_state = ListState::default();
        av1_tune_state.select(Some(0)); // Visual Quality

        let mut av1_scm_state = ListState::default();
        av1_scm_state.select(Some(0)); // Off

        Self {
            focus: ConfigFocus::default(),
            profile_list_state,
            quality_mode_state,
            profile_dropdown_state,
            pix_fmt_state,
            aq_mode_state,
            tune_content_state,
            colorspace_preset_state,
            arnr_type_state,
            fps_dropdown_state,
            resolution_dropdown_state,
            container_dropdown_state,

            // Video codec selection
            video_codec_state,
            codec_selection: CodecSelection::Av1,

            // AV1 software settings
            av1_preset: 8,
            av1_tune_state,
            av1_film_grain: 0,
            av1_film_grain_denoise: false,
            av1_enable_overlays: false,
            av1_scd: true,
            av1_scm_state,
            av1_enable_tf: true,

            // AV1 hardware settings
            av1_hw_preset: 4, // Default: 4 (Balanced)
            av1_hw_cq: 30,    // Legacy fallback

            // Per-encoder quality defaults (calibrated for balanced quality)
            av1_svt_crf: 28,   // SVT-AV1 default CRF
            av1_qsv_cq: 65,    // Intel QSV balanced
            av1_nvenc_cq: 16,  // NVIDIA (65/255*63 ≈ 16)
            av1_vaapi_cq: 65,  // VAAPI same as QSV

            av1_hw_lookahead: 0,
            av1_hw_tile_cols: 0,
            av1_hw_tile_rows: 0,

            // Profile tracking
            current_profile_name: Some("YouTube 4K".to_string()), // Default profile
            is_modified: false,
            available_profiles: Vec::new(), // Will be loaded on first render

            // General settings
            output_dir: std::env::current_dir()
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
                .unwrap_or_else(|| ".".to_string()),
            overwrite: true,
            max_workers: 1, // Default to sequential processing

            // Filename customization
            filename_pattern: "{basename}".to_string(),

            // Additional FFmpeg arguments (empty by default)
            additional_args: String::new(),

            // Video output constraints
            fps: 0,           // Source (no fps limit)
            scale_width: -2,  // Source (no resolution limit)
            scale_height: -2, // Source (no resolution limit)

            // Rate control (defaults for CQ mode)
            rate_control_mode: RateControlMode::CQ,
            crf: 30,
            video_target_bitrate: 0,
            video_min_bitrate: 0,
            video_max_bitrate: 0,
            video_bufsize: 0,
            undershoot_pct: -1, // Auto
            overshoot_pct: -1,  // Auto

            // Speed & quality (good defaults for VOD)
            cpu_used: 1,       // Used when two_pass is false
            cpu_used_pass1: 4, // Fast analysis for pass 1
            cpu_used_pass2: 1, // High quality for pass 2
            two_pass: true,    // Strongly recommended for VOD quality

            // Parallelism
            row_mt: true,
            tile_columns: 2, // Good for 1080p
            tile_rows: 0,
            threads: 0, // Auto
            frame_parallel: false,

            // GOP & keyframes (10 seconds at 25fps = 240)
            gop_length: "240".to_string(),
            keyint_min: "0".to_string(), // Auto (0 means no minimum constraint)
            fixed_gop: false,
            lag_in_frames: 25,
            auto_alt_ref: 1,

            // Alt-ref denoising (ARNR)
            arnr_max_frames: 7,
            arnr_strength: 3,
            arnr_type: -1, // Auto

            // Advanced tuning
            enable_tpl: true, // Recommended for 2-pass efficiency
            sharpness: -1,    // Auto
            noise_sensitivity: 0,
            static_thresh: "0".to_string(), // Disabled (no block skipping)
            max_intra_rate: "0".to_string(), // Disabled (no I-frame bitrate cap)

            // Color / HDR settings (all Auto by default)
            colorspace: -1,
            color_primaries: -1,
            color_trc: -1,
            color_range: -1,
            colorspace_preset: ColorSpacePreset::Auto,

            // Audio settings - multi-track
            audio_primary_codec: AudioPrimaryCodec::Opus,
            audio_primary_codec_state: {
                let mut state = ListState::default();
                state.select(Some(1)); // Opus is index 1
                state
            },
            audio_primary_bitrate: 128,
            audio_primary_downmix: false,
            audio_add_ac3: false,
            audio_ac3_bitrate: 448,
            audio_add_stereo: false,
            audio_stereo_codec: AudioStereoCodec::Aac,
            audio_stereo_codec_state: {
                let mut state = ListState::default();
                state.select(Some(0)); // AAC
                state
            },
            audio_stereo_bitrate: 128,

            // Hardware encoding settings
            use_hardware_encoding: false,
            hw_encoding_available: None,
            hw_availability_message: None,
            gpu_vendor: crate::engine::hardware::GpuVendor::Unknown,
            qsv_global_quality: 70,
            vaapi_rc_mode: "1".to_string(), // CQP mode (only supported mode)
            vaapi_compression_level: "4".to_string(),
            vaapi_b_frames: "0".to_string(),
            vaapi_loop_filter_level: "16".to_string(),
            vaapi_loop_filter_sharpness: "4".to_string(),
            hw_denoise: "0".to_string(),
            hw_detail: "0".to_string(),

            // VP9 QSV controls
            vp9_qsv_preset: 4,
            vp9_qsv_lookahead: true,
            vp9_qsv_lookahead_depth: 40,

            // Auto-VAMF settings (disabled by default)
            auto_vmaf_enabled: false,
            auto_vmaf_target: "93.0".to_string(),
            auto_vmaf_step: "2".to_string(),
            auto_vmaf_max_attempts: "3".to_string(),

            // Popup dropdown state
            active_dropdown: None,

            // Profile name input dialog
            name_input_dialog: None,

            // Text input mode (Normal = navigation, Editing = text entry)
            input_mode: InputMode::Normal,

            // Cursor position for text inputs
            cursor_pos: 0,

            // Mouse support - all None by default
            overwrite_checkbox_area: None,
            two_pass_checkbox_area: None,
            row_mt_checkbox_area: None,
            frame_parallel_checkbox_area: None,
            fixed_gop_checkbox_area: None,
            auto_alt_ref_checkbox_area: None,
            enable_tpl_checkbox_area: None,
            save_button_area: None,
            delete_button_area: None,
            output_dir_area: None,
            filename_pattern_area: None,
            container_dropdown_area: None,
            fps_area: None,
            scale_width_area: None,
            scale_height_area: None,
            profile_list_area: None,
            quality_mode_area: None,
            vp9_profile_list_area: None,
            pix_fmt_area: None,
            aq_mode_area: None,
            tune_content_area: None,
            rate_control_mode_area: None,
            crf_slider_area: None,
            cpu_used_slider_area: None,
            cpu_used_pass1_slider_area: None,
            cpu_used_pass2_slider_area: None,
            tile_columns_slider_area: None,
            tile_rows_slider_area: None,
            threads_area: None,
            max_workers_area: None,
            gop_length_area: None,
            keyint_min_area: None,
            lag_in_frames_slider_area: None,
            arnr_max_frames_slider_area: None,
            arnr_strength_slider_area: None,
            sharpness_slider_area: None,
            noise_sensitivity_slider_area: None,
            video_target_bitrate_area: None,
            video_min_bitrate_area: None,
            video_max_bitrate_area: None,
            video_bufsize_area: None,
            // Audio areas
            audio_primary_codec_area: None,
            audio_primary_bitrate_area: None,
            audio_primary_downmix_area: None,
            audio_ac3_checkbox_area: None,
            audio_ac3_bitrate_area: None,
            audio_stereo_checkbox_area: None,
            audio_stereo_codec_area: None,
            audio_stereo_bitrate_area: None,
            colorspace_preset_area: None,
            arnr_type_area: None,
            static_thresh_area: None,
            max_intra_rate_area: None,
            undershoot_pct_area: None,
            overshoot_pct_area: None,

            // Hardware encoding areas
            hw_encoding_checkbox_area: None,
            qsv_quality_slider_area: None,
            vaapi_compression_level_slider_area: None,
            vaapi_b_frames_area: None,
            vaapi_loop_filter_level_area: None,
            vaapi_loop_filter_sharpness_area: None,
            hw_denoise_area: None,
            hw_detail_area: None,

            // Video codec selector area
            video_codec_area: None,

            // AV1 software areas
            av1_preset_slider_area: None,
            av1_tune_area: None,
            av1_film_grain_slider_area: None,
            av1_film_grain_denoise_checkbox_area: None,
            av1_enable_overlays_checkbox_area: None,
            av1_scd_checkbox_area: None,
            av1_scm_area: None,
            av1_enable_tf_checkbox_area: None,

            // AV1 hardware areas
            av1_hw_preset_area: None,
            av1_hw_cq_slider_area: None,
            av1_hw_lookahead_area: None,
            av1_hw_tile_cols_area: None,
            av1_hw_tile_rows_area: None,

            // VP9 QSV areas
            vp9_qsv_preset_area: None,
            vp9_qsv_lookahead_checkbox_area: None,
            vp9_qsv_lookahead_depth_area: None,

            // Auto-VMAF areas
            auto_vmaf_checkbox_area: None,
            auto_vmaf_target_area: None,
            auto_vmaf_step_area: None,
            auto_vmaf_max_attempts_area: None,

            // Additional args area
            additional_args_area: None,

            status_message: None,
        }
    }
}

impl ConfigState {
    /// Load available profiles from disk and update the cached list
    pub fn refresh_available_profiles(&mut self) {
        use crate::engine::Profile;

        if let Ok(profiles_dir) = Profile::profiles_dir() {
            if let Ok(saved_profiles) = Profile::list_saved(&profiles_dir) {
                self.available_profiles = saved_profiles;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_list_states_initialized() {
        let state = ConfigState::default();

        // Check that all list states have valid initial selections
        assert!(state.profile_list_state.selected().is_some());
        assert!(state.quality_mode_state.selected().is_some());
        assert!(state.profile_dropdown_state.selected().is_some());
        assert!(state.audio_primary_codec_state.selected().is_some());
        assert!(state.audio_stereo_codec_state.selected().is_some());
        assert!(state.pix_fmt_state.selected().is_some());
        assert!(state.aq_mode_state.selected().is_some());
        assert!(state.tune_content_state.selected().is_some());
    }

    #[test]
    fn test_dropdown_item_counts() {
        // Validate that item counts match expectations
        let profiles = vec!["YouTube 4K", "Archival", "Low Latency", "Create New..."];
        let quality_modes = vec!["good", "realtime", "best"];
        let vp9_profiles = vec![
            "Profile 0 (8-bit)",
            "Profile 1 (8-bit)",
            "Profile 2 (10-bit)",
            "Profile 3 (10-bit)",
        ];
        let primary_codecs = vec!["Passthrough", "Opus", "AAC", "MP3", "Vorbis"];
        let stereo_codecs = vec!["AAC", "Opus"];
        let pix_fmts = vec!["yuv420p (8-bit)", "yuv420p10le (10-bit)"];
        let aq_modes = vec![
            "Auto",
            "Off",
            "Variance",
            "Complexity",
            "Cyclic",
            "360 Video",
        ];
        let tune_contents = vec!["default", "screen", "film"];

        assert_eq!(profiles.len(), 4);
        assert_eq!(quality_modes.len(), 3);
        assert_eq!(vp9_profiles.len(), 4);
        assert_eq!(primary_codecs.len(), 5);
        assert_eq!(stereo_codecs.len(), 2);
        assert_eq!(pix_fmts.len(), 2);
        assert_eq!(aq_modes.len(), 6);
        assert_eq!(tune_contents.len(), 3);
    }
}
