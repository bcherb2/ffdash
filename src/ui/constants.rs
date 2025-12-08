// UI dropdown list constants - single source of truth
// These arrays must match the mapping logic in src/engine/mod.rs Profile::from_config

// Container formats
pub const CONTAINER_FORMATS: &[&str] = &["webm", "mp4", "mkv", "avi"];

// Audio codecs
pub const AUDIO_CODECS: &[&str] = &["libopus", "aac", "mp3", "vorbis"];

// VP9 profiles
pub const VP9_PROFILES: &[&str] = &[
    "VP9 (8-bit)",
    "VP9 (8-bit 444)",
    "VP9 (10-bit)",
    "VP9 (10-bit 444)",
];

// Pixel formats - display versions (for UI)
pub const PIX_FMTS_DISPLAY: &[&str] = &["yuv420p (8-bit)", "yuv420p10le (10-bit)"];

// Pixel formats - actual FFmpeg values (for mapping)
pub const PIX_FMTS: &[&str] = &["yuv420p", "yuv420p10le"];

// Quality modes (libvpx-vp9)
pub const QUALITY_MODES: &[&str] = &["good", "realtime", "best"];

// Adaptive quantization modes
pub const AQ_MODES: &[&str] = &[
    "Auto",
    "Off",
    "Variance",
    "Complexity",
    "Cyclic",
    "360 Video",
];

// Tune content modes
pub const TUNE_CONTENTS: &[&str] = &["default", "screen", "film"];

// ARNR types
pub const ARNR_TYPES: &[&str] = &["Auto", "Backward", "Forward", "Centered"];

// Colorspace options
pub const COLORSPACES: &[&str] = &["Auto", "BT709", "BT470BG", "SMPTE170M", "BT2020"];

// Color primaries
pub const COLOR_PRIMARIES: &[&str] = &["Auto", "BT709", "BT470M", "BT470BG", "BT2020"];

// Color transfer characteristics
pub const COLOR_TRCS: &[&str] = &["Auto", "BT709", "SMPTE170M", "SMPTE2084", "ARIB-B67"];

// Color ranges
pub const COLOR_RANGES: &[&str] = &["Auto", "TV", "PC"];

// FPS options
pub const FPS_OPTIONS: &[&str] = &[
    "Source", "23.976", "24", "25", "29.97", "30", "50", "59.94", "60", "120", "144",
];

// FPS options - display versions (for dropdown UI with "fps" suffix)
pub const FPS_OPTIONS_DISPLAY: &[&str] = &[
    "Source",
    "23.976 fps",
    "24 fps",
    "25 fps",
    "29.97 fps",
    "30 fps",
    "50 fps",
    "59.94 fps",
    "60 fps",
    "120 fps",
    "144 fps",
];

// Resolution options - display versions
pub const RESOLUTION_OPTIONS: &[&str] = &[
    "Source",
    "360p",
    "480p",
    "720p",
    "1080p",
    "1440p",
    "2160p (4K)",
];

// Resolution options - for dropdown UI with dimensions
pub const RESOLUTION_OPTIONS_DISPLAY: &[&str] = &[
    "Source",
    "360p (640x360)",
    "480p (854x480)",
    "720p (1280x720)",
    "1080p (1920x1080)",
    "1440p (2560x1440)",
    "2160p/4K (3840x2160)",
];
