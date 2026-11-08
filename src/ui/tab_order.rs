// Tab / Shift+Tab order for the config screen.
//
// The order is derived from what is actually on screen: section by section
// (Profile, General & Audio I/O, Core Video Encoding, Parallelism,
// GOP & Keyframes, Tuning & Filters), and within a section left to right,
// then top to bottom. Controls that aren't rendered (no area) are skipped.

use crate::ui::focus::ConfigFocus;
use crate::ui::state::ConfigState;
use ratatui::layout::Rect;

/// The on-screen sections of the config screen, in Tab order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConfigSection {
    Profile,
    GeneralAudio,
    CoreVideo,
    Parallelism,
    GopKeyframes,
    TuningFilters,
}

impl ConfigFocus {
    /// The section this control is rendered in.
    pub fn section(&self) -> ConfigSection {
        use ConfigSection::*;
        match self {
            Self::ProfileList | Self::SaveButton | Self::DeleteButton => Profile,

            Self::OutputDirectory
            | Self::FilenamePattern
            | Self::ContainerDropdown
            | Self::OverwriteCheckbox
            | Self::FpsDropdown
            | Self::ResolutionDropdown
            | Self::AudioPrimaryCodec
            | Self::AudioPrimaryBitrate
            | Self::AudioPrimaryDownmix
            | Self::AudioAc3Checkbox
            | Self::AudioAc3Bitrate
            | Self::AudioStereoCheckbox
            | Self::AudioStereoCodec
            | Self::AudioStereoBitrate
            | Self::AdditionalArgsInput => GeneralAudio,

            Self::RateControlMode
            | Self::CrfSlider
            | Self::VideoTargetBitrateInput
            | Self::VideoMinBitrateInput
            | Self::VideoMaxBitrateInput
            | Self::VideoBufsizeInput
            | Self::UndershootPctInput
            | Self::OvershootPctInput
            | Self::AutoVmafCheckbox
            | Self::AutoVmafTargetInput
            | Self::AutoVmafStepInput
            | Self::AutoVmafMaxAttemptsInput
            | Self::QualityMode
            | Self::CpuUsedSlider
            | Self::CpuUsedPass1Slider
            | Self::CpuUsedPass2Slider
            | Self::TwoPassCheckbox
            | Self::HardwareEncodingCheckbox
            | Self::QsvGlobalQualitySlider
            | Self::VaapiCompressionLevelSlider
            | Self::VideoCodecDropdown
            | Self::ProfileDropdown
            | Self::PixFmtDropdown
            | Self::Av1PresetSlider
            | Self::Av1TuneDropdown
            | Self::Av1FilmGrainSlider
            | Self::Av1FilmGrainDenoiseCheckbox
            | Self::Av1EnableOverlaysCheckbox
            | Self::Av1ScdCheckbox
            | Self::Av1ScmDropdown
            | Self::Av1EnableTfCheckbox
            | Self::Av1HwPresetSlider
            | Self::Av1HwCqSlider
            | Self::Av1HwLookaheadInput
            | Self::Av1HwTileColsInput
            | Self::Av1HwTileRowsInput
            | Self::HevcX265PresetDropdown
            | Self::HevcX265TuneDropdown
            | Self::HevcNvencPresetSlider
            | Self::HevcNvencCqSlider
            | Self::HevcNvencLookaheadInput
            | Self::HevcNvencBFramesInput
            | Self::HevcQsvQualityInput
            | Self::HevcQsvPresetInput
            | Self::HevcQsvLookaheadInput
            | Self::HevcQsvBFramesInput
            | Self::HevcAmfQpSlider
            | Self::HevcAmfPresetSlider
            | Self::HevcVideotoolboxQSlider
            | Self::HevcVaapiQualitySlider
            | Self::HevcVaapiBFramesInput => CoreVideo,

            Self::RowMtCheckbox
            | Self::TileColumnsSlider
            | Self::TileRowsSlider
            | Self::ThreadsInput
            | Self::MaxWorkersInput
            | Self::FrameParallelCheckbox => Parallelism,

            Self::GopLengthInput
            | Self::KeyintMinInput
            | Self::FixedGopCheckbox
            | Self::LagInFramesSlider
            | Self::AutoAltRefCheckbox => GopKeyframes,

            Self::AqModeDropdown
            | Self::ArnrMaxFramesSlider
            | Self::ArnrStrengthSlider
            | Self::ArnrTypeDropdown
            | Self::TuneContentDropdown
            | Self::EnableTplCheckbox
            | Self::SharpnessSlider
            | Self::NoiseSensitivitySlider
            | Self::StaticThreshInput
            | Self::MaxIntraRateInput
            | Self::ColorSpacePresetDropdown
            | Self::VaapiBFramesInput
            | Self::VaapiLoopFilterLevelInput
            | Self::VaapiLoopFilterSharpnessInput
            | Self::HwDenoiseInput
            | Self::HwDetailInput
            | Self::Vp9QsvPresetSlider
            | Self::Vp9QsvLookaheadCheckbox
            | Self::Vp9QsvLookaheadDepthInput => TuningFilters,
        }
    }

    /// Where this control was last rendered, or `None` if it isn't on screen.
    pub fn area(&self, c: &ConfigState) -> Option<Rect> {
        match self {
            Self::ProfileList => c.profile_list_area,
            Self::SaveButton => c.save_button_area,
            Self::DeleteButton => c.delete_button_area,
            Self::OutputDirectory => c.output_dir_area,
            Self::FilenamePattern => c.filename_pattern_area,
            Self::ContainerDropdown => c.container_dropdown_area,
            Self::OverwriteCheckbox => c.overwrite_checkbox_area,
            Self::FpsDropdown => c.fps_area,
            Self::ResolutionDropdown => c.scale_width_area,
            Self::RateControlMode => c.rate_control_mode_area,
            Self::CrfSlider => c.crf_slider_area,
            Self::VideoTargetBitrateInput => c.video_target_bitrate_area,
            Self::VideoMinBitrateInput => c.video_min_bitrate_area,
            Self::VideoMaxBitrateInput => c.video_max_bitrate_area,
            Self::VideoBufsizeInput => c.video_bufsize_area,
            Self::UndershootPctInput => c.undershoot_pct_area,
            Self::OvershootPctInput => c.overshoot_pct_area,
            Self::AutoVmafCheckbox => c.auto_vmaf_checkbox_area,
            Self::AutoVmafTargetInput => c.auto_vmaf_target_area,
            Self::AutoVmafStepInput => c.auto_vmaf_step_area,
            Self::AutoVmafMaxAttemptsInput => c.auto_vmaf_max_attempts_area,
            Self::QualityMode => c.quality_mode_area,
            Self::CpuUsedSlider => c.cpu_used_slider_area,
            Self::CpuUsedPass1Slider => c.cpu_used_pass1_slider_area,
            Self::CpuUsedPass2Slider => c.cpu_used_pass2_slider_area,
            Self::TwoPassCheckbox => c.two_pass_checkbox_area,
            Self::HardwareEncodingCheckbox => c.hw_encoding_checkbox_area,
            Self::QsvGlobalQualitySlider => c.qsv_quality_slider_area,
            Self::VaapiCompressionLevelSlider => c.vaapi_compression_level_slider_area,
            Self::VaapiBFramesInput => c.vaapi_b_frames_area,
            Self::VaapiLoopFilterLevelInput => c.vaapi_loop_filter_level_area,
            Self::VaapiLoopFilterSharpnessInput => c.vaapi_loop_filter_sharpness_area,
            Self::HwDenoiseInput => c.hw_denoise_area,
            Self::HwDetailInput => c.hw_detail_area,
            Self::Vp9QsvPresetSlider => c.vp9_qsv_preset_area,
            Self::Vp9QsvLookaheadCheckbox => c.vp9_qsv_lookahead_checkbox_area,
            Self::Vp9QsvLookaheadDepthInput => c.vp9_qsv_lookahead_depth_area,
            Self::VideoCodecDropdown => c.video_codec_area,
            Self::ProfileDropdown => c.vp9_profile_list_area,
            Self::PixFmtDropdown => c.pix_fmt_area,
            Self::Av1PresetSlider => c.av1_preset_slider_area,
            Self::Av1TuneDropdown => c.av1_tune_area,
            Self::Av1FilmGrainSlider => c.av1_film_grain_slider_area,
            Self::Av1FilmGrainDenoiseCheckbox => c.av1_film_grain_denoise_checkbox_area,
            Self::Av1EnableOverlaysCheckbox => c.av1_enable_overlays_checkbox_area,
            Self::Av1ScdCheckbox => c.av1_scd_checkbox_area,
            Self::Av1ScmDropdown => c.av1_scm_area,
            Self::Av1EnableTfCheckbox => c.av1_enable_tf_checkbox_area,
            Self::Av1HwPresetSlider => c.av1_hw_preset_area,
            Self::Av1HwCqSlider => c.av1_hw_cq_slider_area,
            Self::Av1HwLookaheadInput => c.av1_hw_lookahead_area,
            Self::Av1HwTileColsInput => c.av1_hw_tile_cols_area,
            Self::Av1HwTileRowsInput => c.av1_hw_tile_rows_area,
            Self::HevcX265PresetDropdown => c.hevc_x265_preset_area,
            Self::HevcX265TuneDropdown => c.hevc_x265_tune_area,
            Self::HevcNvencPresetSlider => c.hevc_nvenc_preset_area,
            Self::HevcNvencCqSlider => c.hevc_nvenc_cq_area,
            Self::HevcNvencLookaheadInput => c.hevc_nvenc_lookahead_area,
            Self::HevcNvencBFramesInput => c.hevc_nvenc_b_frames_area,
            Self::HevcQsvQualityInput => c.hevc_qsv_quality_area,
            Self::HevcQsvPresetInput => c.hevc_qsv_preset_area,
            Self::HevcQsvLookaheadInput => c.hevc_qsv_lookahead_area,
            Self::HevcQsvBFramesInput => c.hevc_qsv_b_frames_area,
            Self::HevcAmfQpSlider => c.hevc_amf_qp_area,
            Self::HevcAmfPresetSlider => c.hevc_amf_preset_area,
            Self::HevcVideotoolboxQSlider => c.hevc_videotoolbox_q_area,
            Self::HevcVaapiQualitySlider => c.hevc_vaapi_quality_slider_area,
            Self::HevcVaapiBFramesInput => c.hevc_vaapi_b_frames_area,
            Self::RowMtCheckbox => c.row_mt_checkbox_area,
            Self::TileColumnsSlider => c.tile_columns_slider_area,
            Self::TileRowsSlider => c.tile_rows_slider_area,
            Self::ThreadsInput => c.threads_area,
            Self::MaxWorkersInput => c.max_workers_area,
            Self::FrameParallelCheckbox => c.frame_parallel_checkbox_area,
            Self::GopLengthInput => c.gop_length_area,
            Self::KeyintMinInput => c.keyint_min_area,
            Self::FixedGopCheckbox => c.fixed_gop_checkbox_area,
            Self::LagInFramesSlider => c.lag_in_frames_slider_area,
            Self::AutoAltRefCheckbox => c.auto_alt_ref_checkbox_area,
            Self::AqModeDropdown => c.aq_mode_area,
            Self::ArnrMaxFramesSlider => c.arnr_max_frames_slider_area,
            Self::ArnrStrengthSlider => c.arnr_strength_slider_area,
            Self::ArnrTypeDropdown => c.arnr_type_area,
            Self::TuneContentDropdown => c.tune_content_area,
            Self::EnableTplCheckbox => c.enable_tpl_checkbox_area,
            Self::SharpnessSlider => c.sharpness_slider_area,
            Self::NoiseSensitivitySlider => c.noise_sensitivity_slider_area,
            Self::StaticThreshInput => c.static_thresh_area,
            Self::MaxIntraRateInput => c.max_intra_rate_area,
            Self::ColorSpacePresetDropdown => c.colorspace_preset_area,
            Self::AudioPrimaryCodec => c.audio_primary_codec_area,
            Self::AudioPrimaryBitrate => c.audio_primary_bitrate_area,
            Self::AudioPrimaryDownmix => c.audio_primary_downmix_area,
            Self::AudioAc3Checkbox => c.audio_ac3_checkbox_area,
            Self::AudioAc3Bitrate => c.audio_ac3_bitrate_area,
            Self::AudioStereoCheckbox => c.audio_stereo_checkbox_area,
            Self::AudioStereoCodec => c.audio_stereo_codec_area,
            Self::AudioStereoBitrate => c.audio_stereo_bitrate_area,
            Self::AdditionalArgsInput => c.additional_args_area,
        }
    }
}

/// Every focus target. Walking the old `next()` chain once visits each variant,
/// so this can't drift out of sync with the enum.
pub(crate) fn all_focus_targets() -> Vec<ConfigFocus> {
    let start = ConfigFocus::default();
    let mut all = vec![start];
    let mut f = start.next();
    while f != start {
        all.push(f);
        f = f.next();
    }
    all
}

impl ConfigFocus {
    /// Whether the control can currently be used. Some controls keep a render
    /// area even when they are hidden or greyed out, so rendering alone isn't enough.
    fn is_usable(&self, c: &ConfigState) -> bool {
        match self {
            // Greyed out and inert unless the Codec has a Hardware Encoder available
            Self::HardwareEncodingCheckbox => c.hardware_control_available(),
            // Passthrough has no bitrate or downmix
            Self::AudioPrimaryBitrate | Self::AudioPrimaryDownmix => {
                !c.audio_primary_codec.is_passthrough()
            }
            Self::AudioAc3Bitrate => c.audio_add_ac3,
            Self::AudioStereoCodec | Self::AudioStereoBitrate => c.audio_add_stereo,
            _ => true,
        }
    }
}

/// Tab order of the controls currently on screen.
pub fn tab_order(config: &ConfigState) -> Vec<ConfigFocus> {
    let mut items: Vec<(ConfigSection, u16, u16, ConfigFocus)> = all_focus_targets()
        .into_iter()
        .filter(|f| f.is_usable(config))
        .filter_map(|f| f.area(config).map(|a| (f.section(), a.y, a.x, f)))
        .collect();
    items.sort_by_key(|(s, y, x, _)| (*s, *y, *x));
    items.into_iter().map(|(_, _, _, f)| f).collect()
}

/// If focus is on a control that isn't focusable any more (it was greyed out
/// by a codec/backend change), move it to the next focusable control.
pub fn settle_focus(config: &mut ConfigState) {
    let order = tab_order(config);
    if order.is_empty() || order.contains(&config.focus) {
        return;
    }
    let mut f = config.focus.next();
    while f != config.focus {
        if order.contains(&f) {
            config.focus = f;
            return;
        }
        f = f.next();
    }
}

/// The control that Tab (`forward`) or Shift+Tab moves to from the current focus.
pub fn step_focus(config: &ConfigState, forward: bool) -> ConfigFocus {
    let order = tab_order(config);
    if order.is_empty() {
        return config.focus;
    }
    match order.iter().position(|f| *f == config.focus) {
        Some(i) if forward => order[(i + 1) % order.len()],
        Some(i) => order[(i + order.len() - 1) % order.len()],
        // Current focus isn't on screen (e.g. the mode changed): start at an end
        None if forward => order[0],
        None => order[order.len() - 1],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::ConfigScreen;
    use ratatui::{Terminal, backend::TestBackend};

    fn render(state: &mut ConfigState, w: u16, h: u16) {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        let mut viewport = Rect::default();
        term.draw(|f| ConfigScreen::render(f, state, &mut viewport))
            .unwrap();
    }

    #[test]
    fn tab_visits_sections_in_order_then_left_to_right_top_to_bottom() {
        let mut state = ConfigState::default();
        render(&mut state, 200, 60);
        let order = tab_order(&state);
        assert!(!order.is_empty());
        // Sections never go backwards
        for w in order.windows(2) {
            assert!(w[0].section() <= w[1].section(), "{:?} -> {:?}", w[0], w[1]);
        }
        // Within a section, position is (row, column) ascending
        for w in order.windows(2) {
            if w[0].section() == w[1].section() {
                let a = w[0].area(&state).unwrap();
                let b = w[1].area(&state).unwrap();
                assert!((a.y, a.x) <= (b.y, b.x), "{:?} -> {:?}", w[0], w[1]);
            }
        }
        // Every section is visited
        for s in [
            ConfigSection::Profile,
            ConfigSection::GeneralAudio,
            ConfigSection::CoreVideo,
            ConfigSection::Parallelism,
            ConfigSection::GopKeyframes,
            ConfigSection::TuningFilters,
        ] {
            assert!(order.iter().any(|f| f.section() == s), "no {s:?} controls");
        }
    }

    #[test]
    fn tab_skips_hidden_and_disabled_controls() {
        let mut state = ConfigState::default();
        state.audio_add_ac3 = false;
        state.audio_add_stereo = false;
        state.audio_primary_codec = crate::ui::state::AudioPrimaryCodec::Passthrough;
        render(&mut state, 200, 60);
        let order = tab_order(&state);
        for hidden in [
            ConfigFocus::AudioPrimaryBitrate,
            ConfigFocus::AudioPrimaryDownmix,
            ConfigFocus::AudioAc3Bitrate,
            ConfigFocus::AudioStereoCodec,
            ConfigFocus::AudioStereoBitrate,
        ] {
            assert!(!order.contains(&hidden), "{hidden:?} should be skipped");
        }
        assert!(order.contains(&ConfigFocus::AudioAc3Checkbox));
        assert!(order.contains(&ConfigFocus::AudioStereoCheckbox));
    }

    fn tab_order_with(
        codec: crate::ui::state::CodecSelection,
        availability: crate::engine::selection::EncoderAvailability,
    ) -> Vec<ConfigFocus> {
        let mut state = ConfigState {
            codec_selection: codec,
            encoder_availability: availability,
            ..Default::default()
        };
        render(&mut state, 200, 60);
        tab_order(&state)
    }

    #[test]
    fn hardware_checkbox_is_in_tab_order_iff_the_codec_has_a_hardware_encoder() {
        use crate::engine::hardware::VideoEncoder::*;
        use crate::engine::selection::EncoderAvailability;
        use crate::ui::state::CodecSelection::{Av1, Vp9};
        let hw = ConfigFocus::HardwareEncodingCheckbox;

        // Whatever the host OS: NVENC for AV1 enables it for AV1 only
        let nvenc = EncoderAvailability::default().with(Av1Nvenc, true);
        assert!(tab_order_with(Av1, nvenc.clone()).contains(&hw));
        assert!(!tab_order_with(Vp9, nvenc).contains(&hw));

        let qsv = EncoderAvailability::default().with(Vp9Qsv, false);
        assert!(tab_order_with(Vp9, qsv).contains(&hw));

        // Software only, or nothing probed: not offered
        let software = EncoderAvailability::default().with(LibsvtAv1, true);
        assert!(!tab_order_with(Av1, software).contains(&hw));
        assert!(!tab_order_with(Av1, EncoderAvailability::default()).contains(&hw));
    }

    #[test]
    fn tab_and_shift_tab_are_inverse_and_wrap() {
        let mut state = ConfigState::default();
        render(&mut state, 200, 60);
        let order = tab_order(&state);
        state.focus = order[0];
        for expected in order.iter().skip(1).chain(order.iter().take(1)) {
            state.focus = step_focus(&state, true);
            assert_eq!(state.focus, *expected);
        }
        for expected in order.iter().rev() {
            state.focus = step_focus(&state, false);
            assert_eq!(state.focus, *expected);
        }
    }
}
