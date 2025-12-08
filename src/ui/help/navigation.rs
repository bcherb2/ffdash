use crate::engine::hardware::HwPreflightResult;

#[derive(Debug, Clone)]
pub struct HelpModalState {
    pub current_section: HelpSection,
    pub scroll_offset: u16,
    pub max_scroll: u16,
    pub app_version: String,
    pub ffmpeg_version: Option<String>,
    pub ffprobe_version: Option<String>,
    pub hw_preflight_result: Option<HwPreflightResult>,
    pub huc_available: Option<bool>,
    pub gpu_metrics_available: bool,
    pub vmaf_available: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpSection {
    About,
    GeneralSettings,
    HardwareEncoding,
    RateControl,
    Parallelism,
    GopKeyframes,
    AdvancedTuning,
    AudioSettings,
    KeyboardShortcuts,
}

impl HelpSection {
    pub fn next(self) -> Self {
        match self {
            Self::About => Self::GeneralSettings,
            Self::GeneralSettings => Self::HardwareEncoding,
            Self::HardwareEncoding => Self::RateControl,
            Self::RateControl => Self::Parallelism,
            Self::Parallelism => Self::GopKeyframes,
            Self::GopKeyframes => Self::AdvancedTuning,
            Self::AdvancedTuning => Self::AudioSettings,
            Self::AudioSettings => Self::KeyboardShortcuts,
            Self::KeyboardShortcuts => Self::About,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Self::About => Self::KeyboardShortcuts,
            Self::GeneralSettings => Self::About,
            Self::HardwareEncoding => Self::GeneralSettings,
            Self::RateControl => Self::HardwareEncoding,
            Self::Parallelism => Self::RateControl,
            Self::GopKeyframes => Self::Parallelism,
            Self::AdvancedTuning => Self::GopKeyframes,
            Self::AudioSettings => Self::AdvancedTuning,
            Self::KeyboardShortcuts => Self::AudioSettings,
        }
    }

    pub fn title(&self) -> &str {
        match self {
            Self::About => "About",
            Self::GeneralSettings => "General Settings",
            Self::HardwareEncoding => "Hardware Encoding",
            Self::RateControl => "Rate Control",
            Self::Parallelism => "Parallelism",
            Self::GopKeyframes => "GOP & Keyframes",
            Self::AdvancedTuning => "Advanced Tuning",
            Self::AudioSettings => "Audio Settings",
            Self::KeyboardShortcuts => "Keyboard Shortcuts",
        }
    }

    pub fn all_sections() -> Vec<Self> {
        vec![
            Self::About,
            Self::GeneralSettings,
            Self::HardwareEncoding,
            Self::RateControl,
            Self::Parallelism,
            Self::GopKeyframes,
            Self::AdvancedTuning,
            Self::AudioSettings,
            Self::KeyboardShortcuts,
        ]
    }
}
