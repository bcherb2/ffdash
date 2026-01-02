//! Shared option mappings for dropdown-backed controls.
//! These helpers keep index <-> value conversions in one place so rendering,
//! event handlers, and profile serialization stay in sync.

use crate::ui::constants::*;
use crate::ui::state::{AudioPrimaryCodec, AudioStereoCodec, CodecSelection, ColorSpacePreset};

pub fn colorspace_from_idx(idx: usize) -> i32 {
    match idx {
        0 => -1,
        1 => 1,
        2 => 5,
        3 => 6,
        4 => 9,
        _ => -1,
    }
}

pub fn colorspace_to_idx(val: i32) -> usize {
    match val {
        1 => 1,
        5 => 2,
        6 => 3,
        9 => 4,
        _ => 0,
    }
}

pub fn color_primaries_from_idx(idx: usize) -> i32 {
    match idx {
        0 => -1,
        1 => 1,
        2 => 4,
        3 => 5,
        4 => 9,
        _ => -1,
    }
}

pub fn color_primaries_to_idx(val: i32) -> usize {
    match val {
        1 => 1,
        4 => 2,
        5 => 3,
        9 => 4,
        _ => 0,
    }
}

pub fn color_trc_from_idx(idx: usize) -> i32 {
    match idx {
        0 => -1,
        1 => 1,
        2 => 6,
        3 => 16,
        4 => 18,
        _ => -1,
    }
}

pub fn color_trc_to_idx(val: i32) -> usize {
    match val {
        1 => 1,
        6 => 2,
        16 => 3,
        18 => 4,
        _ => 0,
    }
}

pub fn color_range_from_idx(idx: usize) -> i32 {
    match idx {
        0 => -1,
        1 => 0,
        2 => 1,
        _ => -1,
    }
}

pub fn color_range_to_idx(val: i32) -> usize {
    match val {
        0 => 1,
        1 => 2,
        _ => 0,
    }
}

// Color Space Preset mappings
pub fn colorspace_preset_from_idx(idx: usize) -> ColorSpacePreset {
    match idx {
        0 => ColorSpacePreset::Auto,
        1 => ColorSpacePreset::Sdr,
        2 => ColorSpacePreset::Hdr10,
        _ => ColorSpacePreset::Auto,
    }
}

pub fn colorspace_preset_to_idx(preset: ColorSpacePreset) -> usize {
    match preset {
        ColorSpacePreset::Auto => 0,
        ColorSpacePreset::Sdr => 1,
        ColorSpacePreset::Hdr10 => 2,
    }
}

// Map preset to (colorspace, color_primaries, color_trc, color_range)
pub fn colorspace_preset_to_values(preset: ColorSpacePreset) -> (i32, i32, i32, i32) {
    match preset {
        ColorSpacePreset::Auto => (-1, -1, -1, -1),
        ColorSpacePreset::Sdr => (1, 1, 1, 0),    // BT709, BT709, BT709, TV
        ColorSpacePreset::Hdr10 => (9, 9, 16, 0), // BT2020, BT2020, SMPTE2084 (PQ), TV
    }
}

// Reverse mapping: detect preset from values (for profile loading)
pub fn colorspace_values_to_preset(cs: i32, cp: i32, ct: i32, cr: i32) -> Option<ColorSpacePreset> {
    match (cs, cp, ct, cr) {
        (-1, -1, -1, -1) => Some(ColorSpacePreset::Auto),
        (1, 1, 1, 0) => Some(ColorSpacePreset::Sdr),
        (9, 9, 16, 0) => Some(ColorSpacePreset::Hdr10),
        _ => None, // Custom values don't map to a preset
    }
}

pub fn arnr_type_from_idx(idx: usize) -> i32 {
    match idx {
        0 => -1,
        1 => 1,
        2 => 2,
        3 => 3,
        _ => -1,
    }
}

pub fn arnr_type_to_idx(val: i32) -> usize {
    match val {
        1 => 1,
        2 => 2,
        3 => 3,
        _ => 0,
    }
}

pub fn fps_from_idx(idx: usize) -> u32 {
    match idx {
        0 => 0,  // Source
        1 => 24, // 23.976 ≈ 24
        2 => 24,
        3 => 25,
        4 => 30, // 29.97 ≈ 30
        5 => 30,
        6 => 50,
        7 => 60, // 59.94 ≈ 60
        8 => 60,
        9 => 120,
        10 => 144,
        _ => 0,
    }
}

pub fn fps_to_idx(val: u32) -> usize {
    match val {
        24 => 2,
        25 => 3,
        30 => 5,
        50 => 6,
        60 => 8,
        120 => 9,
        144 => 10,
        _ => 0,
    }
}

pub fn resolution_from_idx(idx: usize) -> (i32, i32) {
    match idx {
        0 => (-2, -2),     // Source
        1 => (640, 360),   // 360p
        2 => (854, 480),   // 480p
        3 => (1280, 720),  // 720p
        4 => (1920, 1080), // 1080p
        5 => (2560, 1440), // 1440p
        6 => (3840, 2160), // 2160p/4K
        _ => (-2, -2),
    }
}

pub fn resolution_to_idx(width: i32, height: i32) -> usize {
    match (width, height) {
        (-2, -2) => 0,
        (640, 360) => 1,
        (854, 480) => 2,
        (1280, 720) => 3,
        (1920, 1080) => 4,
        (2560, 1440) => 5,
        (3840, 2160) => 6,
        _ => 0,
    }
}

/// Get display name for audio primary codec from index
pub fn audio_primary_codec_display(idx: usize) -> &'static str {
    AUDIO_PRIMARY_CODECS.get(idx).copied().unwrap_or("Opus")
}

/// Get AudioPrimaryCodec enum from index
pub fn audio_primary_codec_from_idx(idx: usize) -> AudioPrimaryCodec {
    AudioPrimaryCodec::from_index(idx)
}

/// Get index from AudioPrimaryCodec enum
pub fn audio_primary_codec_to_idx(codec: AudioPrimaryCodec) -> usize {
    codec.to_index()
}

/// Get display name for audio stereo codec from index
pub fn audio_stereo_codec_display(idx: usize) -> &'static str {
    AUDIO_STEREO_CODECS.get(idx).copied().unwrap_or("AAC")
}

/// Get AudioStereoCodec enum from index
pub fn audio_stereo_codec_from_idx(idx: usize) -> AudioStereoCodec {
    AudioStereoCodec::from_index(idx)
}

/// Get index from AudioStereoCodec enum
pub fn audio_stereo_codec_to_idx(codec: AudioStereoCodec) -> usize {
    codec.to_index()
}

pub fn container_from_idx(idx: usize) -> &'static str {
    CONTAINER_FORMATS.get(idx).copied().unwrap_or("webm")
}

pub fn container_to_idx(value: &str) -> usize {
    match value {
        "webm" => 0,
        "mp4" => 1,
        "mkv" => 2,
        "avi" => 3,
        _ => 0,
    }
}

pub fn pix_fmt_from_idx(idx: usize) -> &'static str {
    PIX_FMTS.get(idx).copied().unwrap_or("auto")
}

pub fn pix_fmt_to_idx(value: &str) -> usize {
    match value {
        "auto" => 0,
        "yuv420p" => 1,
        "yuv420p10le" => 2,
        _ => 0,
    }
}

pub fn quality_mode_from_idx(idx: usize) -> &'static str {
    QUALITY_MODES.get(idx).copied().unwrap_or("good")
}

pub fn quality_mode_to_idx(value: &str) -> usize {
    match value {
        "good" => 0,
        "realtime" => 1,
        "best" => 2,
        _ => 0,
    }
}

pub fn tune_content_from_idx(idx: usize) -> &'static str {
    TUNE_CONTENTS.get(idx).copied().unwrap_or("default")
}

pub fn tune_content_to_idx(value: &str) -> usize {
    match value {
        "screen" => 1,
        "film" => 2,
        "default" => 0,
        _ => 0,
    }
}

pub fn aq_mode_from_idx(idx: usize) -> i32 {
    match idx {
        0 => -1,
        1 => 0,
        2 => 2,
        3 => 1,
        4 => 3,
        5 => 4,
        _ => 2,
    }
}

pub fn aq_mode_to_idx(value: i32) -> usize {
    match value {
        -1 => 0,
        0 => 1,
        2 => 2,
        1 => 3,
        3 => 4,
        4 => 5,
        _ => 2,
    }
}

pub fn codec_selection_from_idx(idx: usize) -> CodecSelection {
    match idx {
        0 => CodecSelection::Vp9,
        1 => CodecSelection::Av1,
        _ => CodecSelection::Vp9,
    }
}

pub fn codec_selection_to_idx(selection: CodecSelection) -> usize {
    match selection {
        CodecSelection::Vp9 => 0,
        CodecSelection::Av1 => 1,
    }
}

pub fn av1_tune_from_idx(idx: usize) -> u32 {
    idx as u32
}

pub fn av1_tune_to_idx(val: u32) -> usize {
    val as usize
}

pub fn av1_scm_from_idx(idx: usize) -> u32 {
    idx as u32
}

pub fn av1_scm_to_idx(val: u32) -> usize {
    val as usize
}
