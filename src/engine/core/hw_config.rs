//! Hardware encoding configuration.
//!
//! Contains `HwEncodingConfig` for VAAPI and other hardware encoder settings.

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
