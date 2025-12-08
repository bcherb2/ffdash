// Focus management for config screen

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFocus {
    // Profile management
    #[default]
    ProfileList,
    SaveButton,
    DeleteButton,

    // General settings
    OutputDirectory,
    FilenamePattern,
    ContainerDropdown,
    OverwriteCheckbox,

    // Video output constraints
    FpsDropdown,
    ResolutionDropdown,

    // Rate control
    RateControlMode,
    CrfSlider,
    VideoTargetBitrateInput,
    VideoMinBitrateInput,
    VideoMaxBitrateInput,
    VideoBufsizeInput,
    UndershootPctInput,
    OvershootPctInput,

    // Speed & quality
    QualityMode,
    CpuUsedSlider,
    CpuUsedPass1Slider,
    CpuUsedPass2Slider,
    TwoPassCheckbox,

    // Hardware encoding (Intel Arc VAAPI)
    HardwareEncodingCheckbox,
    QsvGlobalQualitySlider,
    VaapiCompressionLevelSlider,
    VaapiBFramesInput,
    VaapiLoopFilterLevelInput,
    VaapiLoopFilterSharpnessInput,

    // VP9 settings
    ProfileDropdown,
    PixFmtDropdown,

    // Parallelism
    RowMtCheckbox,
    TileColumnsSlider,
    TileRowsSlider,
    ThreadsInput,
    MaxWorkersInput,
    FrameParallelCheckbox,

    // GOP & keyframes
    GopLengthInput,
    KeyintMinInput,
    FixedGopCheckbox,
    LagInFramesSlider,
    AutoAltRefCheckbox,

    // Adaptive quantization
    AqModeDropdown,

    // Alt-ref denoising
    ArnrMaxFramesSlider,
    ArnrStrengthSlider,
    ArnrTypeDropdown,

    // Advanced tuning
    TuneContentDropdown,
    EnableTplCheckbox,
    SharpnessSlider,
    NoiseSensitivitySlider,
    StaticThreshInput,
    MaxIntraRateInput,

    // Color / HDR
    ColorspaceDropdown,
    ColorPrimariesDropdown,
    ColorTrcDropdown,
    ColorRangeDropdown,

    // Audio
    AudioCodec,
    AudioBitrateSlider,
    ForceStereoCheckbox,
}

impl ConfigFocus {
    pub fn next(&self) -> Self {
        match self {
            Self::ProfileList => Self::SaveButton,
            Self::SaveButton => Self::DeleteButton,
            Self::DeleteButton => Self::OutputDirectory,
            Self::OutputDirectory => Self::FilenamePattern,
            Self::FilenamePattern => Self::ContainerDropdown,
            Self::ContainerDropdown => Self::OverwriteCheckbox,
            Self::OverwriteCheckbox => Self::FpsDropdown,
            Self::FpsDropdown => Self::ResolutionDropdown,
            Self::ResolutionDropdown => Self::RateControlMode,
            Self::RateControlMode => Self::CrfSlider,
            Self::CrfSlider => Self::VideoTargetBitrateInput,
            Self::VideoTargetBitrateInput => Self::VideoMinBitrateInput,
            Self::VideoMinBitrateInput => Self::VideoMaxBitrateInput,
            Self::VideoMaxBitrateInput => Self::VideoBufsizeInput,
            Self::VideoBufsizeInput => Self::UndershootPctInput,
            Self::UndershootPctInput => Self::OvershootPctInput,
            Self::OvershootPctInput => Self::QualityMode,
            Self::QualityMode => Self::CpuUsedSlider,
            Self::CpuUsedSlider => Self::CpuUsedPass1Slider,
            Self::CpuUsedPass1Slider => Self::CpuUsedPass2Slider,
            Self::CpuUsedPass2Slider => Self::TwoPassCheckbox,
            Self::TwoPassCheckbox => Self::HardwareEncodingCheckbox,
            Self::HardwareEncodingCheckbox => Self::QsvGlobalQualitySlider,
            Self::QsvGlobalQualitySlider => Self::VaapiCompressionLevelSlider,
            Self::VaapiCompressionLevelSlider => Self::VaapiBFramesInput,
            Self::VaapiBFramesInput => Self::VaapiLoopFilterLevelInput,
            Self::VaapiLoopFilterLevelInput => Self::VaapiLoopFilterSharpnessInput,
            Self::VaapiLoopFilterSharpnessInput => Self::ProfileDropdown,
            Self::ProfileDropdown => Self::PixFmtDropdown,
            Self::PixFmtDropdown => Self::RowMtCheckbox,
            Self::RowMtCheckbox => Self::TileColumnsSlider,
            Self::TileColumnsSlider => Self::TileRowsSlider,
            Self::TileRowsSlider => Self::ThreadsInput,
            Self::ThreadsInput => Self::MaxWorkersInput,
            Self::MaxWorkersInput => Self::FrameParallelCheckbox,
            Self::FrameParallelCheckbox => Self::GopLengthInput,
            Self::GopLengthInput => Self::KeyintMinInput,
            Self::KeyintMinInput => Self::FixedGopCheckbox,
            Self::FixedGopCheckbox => Self::LagInFramesSlider,
            Self::LagInFramesSlider => Self::AutoAltRefCheckbox,
            Self::AutoAltRefCheckbox => Self::AqModeDropdown,
            Self::AqModeDropdown => Self::ArnrMaxFramesSlider,
            Self::ArnrMaxFramesSlider => Self::ArnrStrengthSlider,
            Self::ArnrStrengthSlider => Self::ArnrTypeDropdown,
            Self::ArnrTypeDropdown => Self::TuneContentDropdown,
            Self::TuneContentDropdown => Self::EnableTplCheckbox,
            Self::EnableTplCheckbox => Self::SharpnessSlider,
            Self::SharpnessSlider => Self::NoiseSensitivitySlider,
            Self::NoiseSensitivitySlider => Self::StaticThreshInput,
            Self::StaticThreshInput => Self::MaxIntraRateInput,
            Self::MaxIntraRateInput => Self::ColorspaceDropdown,
            Self::ColorspaceDropdown => Self::ColorPrimariesDropdown,
            Self::ColorPrimariesDropdown => Self::ColorTrcDropdown,
            Self::ColorTrcDropdown => Self::ColorRangeDropdown,
            Self::ColorRangeDropdown => Self::AudioCodec,
            Self::AudioCodec => Self::ForceStereoCheckbox,
            Self::ForceStereoCheckbox => Self::AudioBitrateSlider,
            Self::AudioBitrateSlider => Self::ProfileList, // Wrap around
        }
    }

    pub fn previous(&self) -> Self {
        match self {
            Self::ProfileList => Self::AudioBitrateSlider, // Wrap around
            Self::SaveButton => Self::ProfileList,
            Self::DeleteButton => Self::SaveButton,
            Self::OutputDirectory => Self::DeleteButton,
            Self::FilenamePattern => Self::OutputDirectory,
            Self::ContainerDropdown => Self::FilenamePattern,
            Self::OverwriteCheckbox => Self::ContainerDropdown,
            Self::FpsDropdown => Self::OverwriteCheckbox,
            Self::ResolutionDropdown => Self::FpsDropdown,
            Self::RateControlMode => Self::ResolutionDropdown,
            Self::CrfSlider => Self::RateControlMode,
            Self::VideoTargetBitrateInput => Self::CrfSlider,
            Self::VideoMinBitrateInput => Self::VideoTargetBitrateInput,
            Self::VideoMaxBitrateInput => Self::VideoMinBitrateInput,
            Self::VideoBufsizeInput => Self::VideoMaxBitrateInput,
            Self::UndershootPctInput => Self::VideoBufsizeInput,
            Self::OvershootPctInput => Self::UndershootPctInput,
            Self::QualityMode => Self::OvershootPctInput,
            Self::CpuUsedSlider => Self::QualityMode,
            Self::CpuUsedPass1Slider => Self::CpuUsedSlider,
            Self::CpuUsedPass2Slider => Self::CpuUsedPass1Slider,
            Self::TwoPassCheckbox => Self::CpuUsedPass2Slider,
            Self::HardwareEncodingCheckbox => Self::TwoPassCheckbox,
            Self::QsvGlobalQualitySlider => Self::HardwareEncodingCheckbox,
            Self::VaapiCompressionLevelSlider => Self::QsvGlobalQualitySlider,
            Self::VaapiBFramesInput => Self::VaapiCompressionLevelSlider,
            Self::VaapiLoopFilterLevelInput => Self::VaapiBFramesInput,
            Self::VaapiLoopFilterSharpnessInput => Self::VaapiLoopFilterLevelInput,
            Self::ProfileDropdown => Self::VaapiLoopFilterSharpnessInput,
            Self::PixFmtDropdown => Self::ProfileDropdown,
            Self::RowMtCheckbox => Self::PixFmtDropdown,
            Self::TileColumnsSlider => Self::RowMtCheckbox,
            Self::TileRowsSlider => Self::TileColumnsSlider,
            Self::ThreadsInput => Self::TileRowsSlider,
            Self::MaxWorkersInput => Self::ThreadsInput,
            Self::FrameParallelCheckbox => Self::MaxWorkersInput,
            Self::GopLengthInput => Self::FrameParallelCheckbox,
            Self::KeyintMinInput => Self::GopLengthInput,
            Self::FixedGopCheckbox => Self::KeyintMinInput,
            Self::LagInFramesSlider => Self::FixedGopCheckbox,
            Self::AutoAltRefCheckbox => Self::LagInFramesSlider,
            Self::AqModeDropdown => Self::AutoAltRefCheckbox,
            Self::ArnrMaxFramesSlider => Self::AqModeDropdown,
            Self::ArnrStrengthSlider => Self::ArnrMaxFramesSlider,
            Self::ArnrTypeDropdown => Self::ArnrStrengthSlider,
            Self::TuneContentDropdown => Self::ArnrTypeDropdown,
            Self::EnableTplCheckbox => Self::TuneContentDropdown,
            Self::SharpnessSlider => Self::EnableTplCheckbox,
            Self::NoiseSensitivitySlider => Self::SharpnessSlider,
            Self::StaticThreshInput => Self::NoiseSensitivitySlider,
            Self::MaxIntraRateInput => Self::StaticThreshInput,
            Self::ColorspaceDropdown => Self::MaxIntraRateInput,
            Self::ColorPrimariesDropdown => Self::ColorspaceDropdown,
            Self::ColorTrcDropdown => Self::ColorPrimariesDropdown,
            Self::ColorRangeDropdown => Self::ColorTrcDropdown,
            Self::AudioCodec => Self::ColorRangeDropdown,
            Self::ForceStereoCheckbox => Self::AudioCodec,
            Self::AudioBitrateSlider => Self::ForceStereoCheckbox,
        }
    }
}
