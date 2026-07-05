/// Type definitions for the encoder parameter registry.
///
/// These types define the structure of parameter definitions that are
/// generated from encoder-params.toml at build time.
use std::fmt;

/// The codec family (VP9 or AV1)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codec {
    Vp9,
    Av1,
}

/// The type of encoder (software vs hardware)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncoderType {
    Software,
    Hardware,
}

/// Hardware API type for hardware encoders
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareApi {
    Vaapi,
    Qsv,
    Nvenc,
    Amf,
}

/// Rust type that a parameter uses
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamType {
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    Bool,
    String,
    F32,
    F64,
}

impl fmt::Display for ParamType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParamType::U8 => write!(f, "u8"),
            ParamType::U16 => write!(f, "u16"),
            ParamType::U32 => write!(f, "u32"),
            ParamType::U64 => write!(f, "u64"),
            ParamType::I8 => write!(f, "i8"),
            ParamType::I16 => write!(f, "i16"),
            ParamType::I32 => write!(f, "i32"),
            ParamType::I64 => write!(f, "i64"),
            ParamType::Bool => write!(f, "bool"),
            ParamType::String => write!(f, "String"),
            ParamType::F32 => write!(f, "f32"),
            ParamType::F64 => write!(f, "f64"),
        }
    }
}

/// Valid range for a parameter
#[derive(Debug, Clone, PartialEq)]
pub enum Range {
    /// Integer range (min, max) - inclusive
    Int { min: i64, max: i64 },

    /// Float range (min, max) - inclusive
    Float { min: f64, max: f64 },

    /// String must be one of these values
    Enum { values: Vec<String> },

    /// Boolean (no range needed)
    Bool,

    /// No specific range (e.g., for metadata fields)
    Any,
}

impl Range {
    /// Check if an integer value is within this range
    pub fn contains_int(&self, value: i64) -> bool {
        match self {
            Range::Int { min, max } => value >= *min && value <= *max,
            Range::Any => true,
            _ => false,
        }
    }

    /// Check if a float value is within this range
    pub fn contains_float(&self, value: f64) -> bool {
        match self {
            Range::Float { min, max } => value >= *min && value <= *max,
            Range::Any => true,
            _ => false,
        }
    }

    /// Check if a string value is valid for this range
    pub fn contains_str(&self, value: &str) -> bool {
        match self {
            Range::Enum { values } => values.iter().any(|v| v == value),
            Range::Any => true,
            _ => false,
        }
    }
}

/// Condition for when to include a parameter in the FFmpeg command
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    /// Always include this parameter
    Always,

    /// Include only if the value is non-zero
    NonZero,

    /// Include only if the value is non-negative (>= 0)
    NonNegative,

    /// Include only if the boolean is true
    BoolTrue,

    /// Include only if the string is non-empty
    NonEmpty,
}

impl fmt::Display for Condition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Condition::Always => write!(f, "always"),
            Condition::NonZero => write!(f, "non_zero"),
            Condition::NonNegative => write!(f, "non_negative"),
            Condition::BoolTrue => write!(f, "bool_true"),
            Condition::NonEmpty => write!(f, "non_empty"),
        }
    }
}

/// Default value for a parameter
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    Bool(bool),
    String(String),
    Str(&'static str), // For static contexts (generated code)
    F32(f32),
    F64(f64),
}

/// Encoder-specific parameter configuration
#[derive(Debug, Clone, PartialEq)]
pub struct EncoderParam {
    /// FFmpeg command-line flag (e.g., "-crf", "-cpu-used")
    /// None if parameter is not supported by this encoder
    pub flag: Option<&'static str>,

    /// Whether this encoder supports this parameter
    pub supported: bool,

    /// Reason why this parameter is not supported (if supported = false)
    pub reason: Option<&'static str>,

    /// Encoder-specific range override (if different from global range)
    pub range_override: Option<Range>,

    /// Condition for when to include this parameter
    pub condition: Condition,

    /// Optional note about encoder-specific behavior
    pub note: Option<&'static str>,
}

/// Definition of a single parameter
#[derive(Debug, Clone, PartialEq)]
pub struct ParamDef {
    /// Parameter name (e.g., "crf", "cpu_used")
    pub name: &'static str,

    /// Field name in the Profile struct
    pub field_path: &'static str,

    /// Rust type of this parameter
    pub rust_type: ParamType,

    /// Human-readable description
    pub description: &'static str,

    /// Parameter group (e.g., "rate_control", "quality")
    pub group: &'static str,

    /// Valid range for this parameter
    pub range: Range,

    /// Default value
    pub default: Value,

    /// Per-encoder support information
    /// Each entry is (encoder_id, encoder_config)
    pub encoder_support: &'static [(&'static str, EncoderParam)],
}

impl ParamDef {
    /// Get encoder-specific configuration for this parameter
    pub fn get_encoder_config(&self, encoder_id: &str) -> Option<&EncoderParam> {
        self.encoder_support
            .iter()
            .find(|(id, _)| *id == encoder_id)
            .map(|(_, config)| config)
    }

    /// Check if this parameter is supported by the given encoder
    pub fn is_supported_by(&self, encoder_id: &str) -> bool {
        self.get_encoder_config(encoder_id)
            .map(|cfg| cfg.supported)
            .unwrap_or(false)
    }
}

/// Definition of an encoder
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncoderDef {
    /// Encoder ID (e.g., "libvpx-vp9", "vp9_vaapi")
    pub id: &'static str,

    /// Codec family
    pub codec: Codec,

    /// Encoder type
    pub encoder_type: EncoderType,

    /// FFmpeg encoder name (e.g., "libvpx-vp9")
    pub ffmpeg_name: &'static str,

    /// Hardware API (for hardware encoders)
    pub hw_api: Option<HardwareApi>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_contains_int() {
        let range = Range::Int { min: 0, max: 63 };
        assert!(range.contains_int(0));
        assert!(range.contains_int(31));
        assert!(range.contains_int(63));
        assert!(!range.contains_int(-1));
        assert!(!range.contains_int(64));
    }

    #[test]
    fn test_range_contains_str() {
        let range = Range::Enum {
            values: vec![
                "good".to_string(),
                "best".to_string(),
                "realtime".to_string(),
            ],
        };
        assert!(range.contains_str("good"));
        assert!(range.contains_str("best"));
        assert!(!range.contains_str("invalid"));
    }

    #[test]
    fn test_param_type_display() {
        assert_eq!(ParamType::U32.to_string(), "u32");
        assert_eq!(ParamType::String.to_string(), "String");
        assert_eq!(ParamType::Bool.to_string(), "bool");
    }
}
