// Which config controls apply to the selected codec and encoder backend.
//
// Controls that don't apply are drawn greyed out and are not focusable or
// clickable (they get no hit area). Render code and the tests that cross-check
// the ffmpeg command builders both use `applies`, so the UI cannot drift from
// what is actually passed to the encoder.

use crate::engine::hardware::GpuVendor;
use crate::ui::focus::ConfigFocus;
use crate::ui::state::CodecSelection;

/// Where the encode runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Software,
    Hardware(GpuVendor),
}

impl Backend {
    pub fn is_hardware(&self) -> bool {
        matches!(self, Backend::Hardware(_))
    }
}

/// Whether `field` has any effect for this codec and backend.
pub fn applies(field: ConfigFocus, codec: CodecSelection, backend: Backend) -> bool {
    use ConfigFocus::*;

    let vp9 = codec == CodecSelection::Vp9;
    let av1 = codec == CodecSelection::Av1;
    let hevc = codec == CodecSelection::Hevc;
    let software = !backend.is_hardware();
    let hardware = backend.is_hardware();

    match field {
        // libvpx-vp9 (software) only
        RowMtCheckbox
        | TileColumnsSlider
        | TileRowsSlider
        | FrameParallelCheckbox
        | FixedGopCheckbox
        | KeyintMinInput
        | LagInFramesSlider
        | AutoAltRefCheckbox
        | AqModeDropdown
        | ArnrMaxFramesSlider
        | ArnrStrengthSlider
        | ArnrTypeDropdown
        | TuneContentDropdown
        | EnableTplCheckbox
        | SharpnessSlider
        | NoiseSensitivitySlider
        | StaticThreshInput
        | MaxIntraRateInput
        | QualityMode
        | CpuUsedSlider
        | CpuUsedPass1Slider
        | CpuUsedPass2Slider
        | TwoPassCheckbox => vp9 && software,

        // libsvtav1 (software) only
        Av1PresetSlider
        | Av1TuneDropdown
        | Av1FilmGrainSlider
        | Av1FilmGrainDenoiseCheckbox
        | Av1EnableOverlaysCheckbox
        | Av1ScdCheckbox
        | Av1ScmDropdown
        | Av1EnableTfCheckbox => av1 && software,

        // libx265 (software) only. Extension point: each HEVC hardware Encoder adds its
        // own focus variants here with `hevc && backend == Backend::Hardware(<vendor>)`.
        HevcX265PresetDropdown | HevcX265TuneDropdown => hevc && software,
        // hevc_amf: constant QP and the shared hardware preset (no lookahead or B-frames)
        HevcAmfQpSlider | HevcAmfPresetSlider => {
            hevc && backend == Backend::Hardware(GpuVendor::Amd)
        }
        // hevc_videotoolbox: its `q:v` quality is the only setting
        HevcVideotoolboxQSlider => hevc && backend == Backend::Hardware(GpuVendor::Apple),

        // hevc_nvenc
        HevcNvencPresetSlider
        | HevcNvencCqSlider
        | HevcNvencLookaheadInput
        | HevcNvencBFramesInput => hevc && backend == Backend::Hardware(GpuVendor::Nvidia),
        // hevc_qsv: quality, preset, lookahead and b-frames (Intel Quick Sync)
        HevcQsvQualityInput | HevcQsvPresetInput | HevcQsvLookaheadInput | HevcQsvBFramesInput => {
            hevc && backend == Backend::Hardware(GpuVendor::Intel)
        }

        // hevc_vaapi: quality (QP) and B-frames. VAAPI is the hardware path when the
        // GPU vendor is not one with a dedicated Encoder (NVENC, QSV, AMF, VideoToolbox).
        HevcVaapiQualitySlider | HevcVaapiBFramesInput => {
            hevc && matches!(backend, Backend::Hardware(GpuVendor::Unknown))
        }

        // AV1 hardware encoders
        Av1HwPresetSlider | Av1HwCqSlider | Av1HwLookaheadInput | Av1HwTileColsInput
        | Av1HwTileRowsInput => av1 && hardware,

        // VP9 hardware encoders
        QsvGlobalQualitySlider
        | VaapiCompressionLevelSlider
        | VaapiBFramesInput
        | VaapiLoopFilterLevelInput
        | VaapiLoopFilterSharpnessInput => vp9 && hardware,
        Vp9QsvPresetSlider | Vp9QsvLookaheadCheckbox | Vp9QsvLookaheadDepthInput => {
            vp9 && backend == Backend::Hardware(GpuVendor::Intel)
        }

        // VPP denoise/detail: QSV and VAAPI, not NVENC
        HwDenoiseInput | HwDetailInput => {
            !hevc && matches!(backend, Backend::Hardware(v) if v != GpuVendor::Nvidia)
        }

        // Passed as -threads by the software builders only
        ThreadsInput => software,

        ProfileDropdown => vp9,

        // Everything else is shared by every codec and backend
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ConfigFocus::*;

    const SW: Backend = Backend::Software;
    const INTEL: Backend = Backend::Hardware(GpuVendor::Intel);
    const NVIDIA: Backend = Backend::Hardware(GpuVendor::Nvidia);
    const AMD: Backend = Backend::Hardware(GpuVendor::Amd);
    const UNKNOWN: Backend = Backend::Hardware(GpuVendor::Unknown);

    #[test]
    fn vp9_only_software_controls_are_inactive_for_av1() {
        for f in [
            RowMtCheckbox,
            TileColumnsSlider,
            TileRowsSlider,
            FrameParallelCheckbox,
            FixedGopCheckbox,
            LagInFramesSlider,
            AutoAltRefCheckbox,
            AqModeDropdown,
            ArnrMaxFramesSlider,
            ArnrStrengthSlider,
            ArnrTypeDropdown,
            TuneContentDropdown,
            EnableTplCheckbox,
            SharpnessSlider,
            NoiseSensitivitySlider,
            StaticThreshInput,
            MaxIntraRateInput,
            QualityMode,
            CpuUsedSlider,
            TwoPassCheckbox,
        ] {
            assert!(
                applies(f, CodecSelection::Vp9, SW),
                "{f:?} should apply to VP9 software"
            );
            assert!(
                !applies(f, CodecSelection::Av1, SW),
                "{f:?} should not apply to AV1 software"
            );
            assert!(
                !applies(f, CodecSelection::Vp9, INTEL),
                "{f:?} should not apply to VP9 hardware"
            );
        }
    }

    #[test]
    fn av1_software_controls_only_apply_to_av1_software() {
        for f in [
            Av1PresetSlider,
            Av1TuneDropdown,
            Av1FilmGrainSlider,
            Av1FilmGrainDenoiseCheckbox,
            Av1EnableOverlaysCheckbox,
            Av1ScdCheckbox,
            Av1ScmDropdown,
            Av1EnableTfCheckbox,
        ] {
            assert!(applies(f, CodecSelection::Av1, SW), "{f:?}");
            assert!(!applies(f, CodecSelection::Vp9, SW), "{f:?}");
            assert!(!applies(f, CodecSelection::Av1, INTEL), "{f:?}");
        }
    }

    #[test]
    fn av1_hardware_controls_only_apply_to_av1_hardware() {
        for f in [
            Av1HwPresetSlider,
            Av1HwCqSlider,
            Av1HwLookaheadInput,
            Av1HwTileColsInput,
            Av1HwTileRowsInput,
        ] {
            assert!(applies(f, CodecSelection::Av1, NVIDIA), "{f:?}");
            assert!(!applies(f, CodecSelection::Av1, SW), "{f:?}");
            assert!(!applies(f, CodecSelection::Vp9, NVIDIA), "{f:?}");
        }
    }

    #[test]
    fn vp9_qsv_controls_need_vp9_on_intel() {
        for f in [Vp9QsvLookaheadCheckbox, Vp9QsvLookaheadDepthInput] {
            assert!(applies(f, CodecSelection::Vp9, INTEL), "{f:?}");
            assert!(!applies(f, CodecSelection::Vp9, AMD), "{f:?}");
            assert!(!applies(f, CodecSelection::Av1, INTEL), "{f:?}");
            assert!(!applies(f, CodecSelection::Vp9, SW), "{f:?}");
        }
    }

    #[test]
    fn vpp_filters_apply_to_non_nvidia_hardware_for_both_codecs() {
        for f in [HwDenoiseInput, HwDetailInput] {
            for codec in [CodecSelection::Vp9, CodecSelection::Av1] {
                assert!(applies(f, codec, INTEL), "{f:?} {codec:?}");
                assert!(applies(f, codec, AMD), "{f:?} {codec:?}");
                assert!(!applies(f, codec, NVIDIA), "{f:?} {codec:?}");
                assert!(!applies(f, codec, SW), "{f:?} {codec:?}");
            }
        }
    }

    #[test]
    fn threads_apply_to_software_of_either_codec_only() {
        for codec in [CodecSelection::Vp9, CodecSelection::Av1] {
            assert!(applies(ThreadsInput, codec, SW));
            assert!(!applies(ThreadsInput, codec, INTEL));
        }
    }

    // ---- Cross-check against the real ffmpeg command builders -------------

    use crate::engine::core::{Codec, Profile, VideoJob};
    use crate::engine::hardware::VideoEncoder;
    use crate::engine::{EncodePlan, build_encode_cmds};

    fn software_cmd(profile: &Profile, encoder: VideoEncoder) -> std::process::Command {
        build_encode_cmds(&job_for(profile), profile, &EncodePlan::new(encoder))
            .pop()
            .unwrap()
    }
    use std::path::PathBuf;

    /// Control -> ffmpeg flag it produces. Controls that share a flag across
    /// codecs (GOP length, pixel format, colour space) are listed too.
    const FLAGS: &[(ConfigFocus, &str)] = &[
        (RowMtCheckbox, "-row-mt"),
        (TileColumnsSlider, "-tile-columns"),
        (TileRowsSlider, "-tile-rows"),
        (FrameParallelCheckbox, "-frame-parallel"),
        (ThreadsInput, "-threads"),
        (GopLengthInput, "-g"),
        (KeyintMinInput, "-keyint_min"),
        (FixedGopCheckbox, "-sc_threshold"),
        (LagInFramesSlider, "-lag-in-frames"),
        (AutoAltRefCheckbox, "-auto-alt-ref"),
        (AqModeDropdown, "-aq-mode"),
        (ArnrMaxFramesSlider, "-arnr-maxframes"),
        (ArnrStrengthSlider, "-arnr-strength"),
        (ArnrTypeDropdown, "-arnr-type"),
        (EnableTplCheckbox, "-enable-tpl"),
        (SharpnessSlider, "-sharpness"),
        (NoiseSensitivitySlider, "-noise-sensitivity"),
        (StaticThreshInput, "-static-thresh"),
        (MaxIntraRateInput, "-max-intra-rate"),
        (TuneContentDropdown, "-tune-content"),
        (QualityMode, "-quality"),
        (CpuUsedSlider, "-cpu-used"),
        (ColorSpacePresetDropdown, "-colorspace:v"),
    ];

    /// A profile with every optional setting turned on, so a builder emits a
    /// flag whenever it supports the control at all.
    fn exercised(mut p: Profile) -> Profile {
        p.row_mt = true;
        p.tile_columns = 2;
        p.tile_rows = 1;
        p.threads = 4;
        p.frame_parallel = true;
        p.keyint_min = "10".to_string();
        p.fixed_gop = true;
        p.lag_in_frames = 10;
        p.auto_alt_ref = 1;
        p.aq_mode = 1;
        p.arnr_max_frames = 5;
        p.arnr_strength = 3;
        p.arnr_type = 1;
        p.enable_tpl = true;
        p.sharpness = 2;
        p.noise_sensitivity = 1;
        p.static_thresh = "100".to_string();
        p.max_intra_rate = "300".to_string();
        p.tune_content = "film".to_string();
        p.colorspace = 1;
        // The entry point syncs legacy fields from the codec config, so VP9 values must live there.
        if let Codec::Vp9(v) = &mut p.codec {
            v.row_mt = p.row_mt;
            v.tile_columns = p.tile_columns;
            v.tile_rows = p.tile_rows;
            v.lag_in_frames = p.lag_in_frames;
            v.auto_alt_ref = p.auto_alt_ref;
            v.aq_mode = p.aq_mode;
            v.arnr_max_frames = p.arnr_max_frames;
            v.arnr_strength = p.arnr_strength;
            v.arnr_type = p.arnr_type;
            v.enable_tpl = p.enable_tpl;
            v.sharpness = p.sharpness;
            v.noise_sensitivity = p.noise_sensitivity;
            v.static_thresh = p.static_thresh.clone();
            v.max_intra_rate = p.max_intra_rate.clone();
            v.tune_content = p.tune_content.clone();
        }
        p
    }

    fn emitted_flags(cmd: &std::process::Command) -> Vec<String> {
        cmd.get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect()
    }

    fn job_for(profile: &Profile) -> VideoJob {
        VideoJob::new(
            PathBuf::from("does-not-exist.mkv"),
            PathBuf::from("out.mkv"),
            profile.name.clone(),
        )
    }

    fn assert_matches_builder(args: &[String], codec: CodecSelection) {
        for (field, flag) in FLAGS {
            let stream_flag = format!("{flag}:v");
            let emitted = args.iter().any(|a| a == flag || *a == stream_flag);
            assert_eq!(
                applies(*field, codec, SW),
                emitted,
                "{field:?} ({flag}) on {codec:?} software: applies() disagrees with the builder (emitted = {emitted})"
            );
        }
    }

    #[test]
    fn applies_matches_flags_the_vp9_software_builder_emits() {
        let profile = exercised(Profile::get("vp9-good"));
        let args = emitted_flags(&software_cmd(&profile, VideoEncoder::LibvpxVp9));
        assert_matches_builder(&args, CodecSelection::Vp9);
    }

    #[test]
    fn applies_matches_flags_the_av1_software_builder_emits() {
        let profile = exercised(Profile::get("av1-svt"));
        let args = emitted_flags(&software_cmd(&profile, VideoEncoder::LibsvtAv1));
        assert_matches_builder(&args, CodecSelection::Av1);
    }

    #[test]
    fn applies_matches_flags_the_hevc_software_builder_emits() {
        let profile = exercised(Profile::get("hevc-x265"));
        let args = emitted_flags(&software_cmd(&profile, VideoEncoder::Libx265));
        assert_matches_builder(&args, CodecSelection::Hevc);
    }

    #[test]
    fn x265_controls_apply_only_to_hevc_software() {
        for f in [HevcX265PresetDropdown, HevcX265TuneDropdown] {
            assert!(applies(f, CodecSelection::Hevc, SW), "{f:?}");
            for hw in [INTEL, NVIDIA, AMD] {
                assert!(!applies(f, CodecSelection::Hevc, hw), "{f:?} {hw:?}");
            }
            assert!(!applies(f, CodecSelection::Vp9, SW), "{f:?}");
            assert!(!applies(f, CodecSelection::Av1, SW), "{f:?}");
        }
    }

    #[test]
    fn hevc_qsv_controls_apply_only_to_hevc_on_intel_hardware() {
        for f in [
            HevcQsvQualityInput,
            HevcQsvPresetInput,
            HevcQsvLookaheadInput,
            HevcQsvBFramesInput,
        ] {
            assert!(applies(f, CodecSelection::Hevc, INTEL), "{f:?}");
            for other in [SW, NVIDIA, AMD] {
                assert!(!applies(f, CodecSelection::Hevc, other), "{f:?} {other:?}");
            }
            assert!(!applies(f, CodecSelection::Av1, INTEL), "{f:?}");
            assert!(!applies(f, CodecSelection::Vp9, INTEL), "{f:?}");
        }
    }

    #[test]
    fn hevc_qsv_settings_are_shown_and_focusable_only_for_hevc_qsv() {
        let qsv = [
            HevcQsvQualityInput,
            HevcQsvPresetInput,
            HevcQsvLookaheadInput,
            HevcQsvBFramesInput,
        ];
        let mut hw = state_for(CodecSelection::Hevc, true);
        hw.gpu_vendor = GpuVendor::Intel;
        let text = render_text(&mut hw);
        assert!(text.contains("Quality (QP):"), "{text}");
        for f in qsv {
            assert!(f.area(&hw).is_some(), "{f:?} should be clickable");
            assert!(tab_order(&hw).contains(&f), "{f:?} should be in Tab order");
        }
        // The x265 controls give way to them
        assert!(HevcX265PresetDropdown.area(&hw).is_none());

        let mut sw = state_for(CodecSelection::Hevc, false);
        render_text(&mut sw);
        for f in qsv {
            assert!(
                f.area(&sw).is_none(),
                "{f:?} must have no hit area on software"
            );
        }
    }

    #[test]
    fn vp9_and_av1_controls_are_greyed_for_hevc() {
        for f in [
            RowMtCheckbox,
            AqModeDropdown,
            TwoPassCheckbox,
            Av1PresetSlider,
            Av1TuneDropdown,
            Av1HwCqSlider,
            QsvGlobalQualitySlider,
            HwDenoiseInput,
        ] {
            for backend in [SW, INTEL, NVIDIA] {
                assert!(
                    !applies(f, CodecSelection::Hevc, backend),
                    "{f:?} {backend:?}"
                );
            }
        }
        // shared controls stay active
        for f in [GopLengthInput, PixFmtDropdown, ColorSpacePresetDropdown] {
            assert!(applies(f, CodecSelection::Hevc, SW), "{f:?}");
            assert!(applies(f, CodecSelection::Hevc, NVIDIA), "{f:?}");
        }
    }

    // ---- Rendering: inactive controls are greyed, not focusable ------------

    use crate::ui::{ConfigScreen, state::ConfigState, tab_order::tab_order};
    use ratatui::{Terminal, backend::TestBackend, layout::Rect};

    /// Render the config screen and return its text, row by row.
    fn render_text(state: &mut ConfigState) -> String {
        let mut term = Terminal::new(TestBackend::new(220, 80)).unwrap();
        let mut viewport = Rect::default();
        term.draw(|f| ConfigScreen::render(f, state, &mut viewport))
            .unwrap();
        let buf = term.backend().buffer().clone();
        let mut out = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    fn state_for(codec: CodecSelection, hardware: bool) -> ConfigState {
        let mut s = ConfigState::default();
        s.codec_selection = codec;
        s.use_hardware_encoding = hardware;
        s
    }

    #[test]
    fn vp9_only_parallelism_rows_are_greyed_and_unfocusable_for_av1() {
        let mut state = state_for(CodecSelection::Av1, false);
        let text = render_text(&mut state);

        assert!(text.contains("Frame Parallel"), "row should still be drawn");
        assert!(
            !text.contains("sw only)") && !text.contains("hw only)"),
            "greyed rows should not carry a '(VP9 sw only)'-style tag"
        );
        for f in [
            RowMtCheckbox,
            FrameParallelCheckbox,
            TileColumnsSlider,
            TileRowsSlider,
        ] {
            assert!(f.area(&state).is_none(), "{f:?} must have no hit area");
            assert!(
                !tab_order(&state).contains(&f),
                "{f:?} must be skipped by Tab"
            );
        }
    }

    #[test]
    fn vp9_only_parallelism_rows_stay_active_for_vp9_software() {
        let mut state = state_for(CodecSelection::Vp9, false);
        let text = render_text(&mut state);

        assert!(text.contains("Frame Parallel"));
        for f in [
            RowMtCheckbox,
            FrameParallelCheckbox,
            TileColumnsSlider,
            TileRowsSlider,
        ] {
            assert!(f.area(&state).is_some(), "{f:?} should be clickable");
            assert!(
                tab_order(&state).contains(&f),
                "{f:?} should be in Tab order"
            );
        }
    }

    #[test]
    fn vp9_only_gop_and_tuning_rows_are_greyed_for_av1_but_shared_ones_stay_active() {
        let mut state = state_for(CodecSelection::Av1, false);
        let text = render_text(&mut state);

        for (f, label) in [
            (FixedGopCheckbox, "Fixed GOP"),
            (KeyintMinInput, "Min Keyframe Interval"),
            (AutoAltRefCheckbox, "Auto Alt-Ref"),
            (EnableTplCheckbox, "Enable TPL"),
            (SharpnessSlider, "Sharpness"),
            (StaticThreshInput, "Static Thresh"),
            (MaxIntraRateInput, "Max I-Rate"),
        ] {
            assert!(text.contains(label), "{label} should still be drawn");
            assert!(f.area(&state).is_none(), "{f:?} must have no hit area");
        }
        for f in [
            GopLengthInput,
            ColorSpacePresetDropdown,
            MaxWorkersInput,
            ThreadsInput,
        ] {
            assert!(
                f.area(&state).is_some(),
                "{f:?} should stay active for AV1 software"
            );
        }
    }

    #[test]
    fn focus_moves_off_a_control_that_turns_grey() {
        let mut state = state_for(CodecSelection::Vp9, false);
        render_text(&mut state);
        state.focus = FrameParallelCheckbox;
        assert!(tab_order(&state).contains(&FrameParallelCheckbox));

        state.codec_selection = CodecSelection::Av1;
        render_text(&mut state);

        assert_ne!(
            state.focus, FrameParallelCheckbox,
            "focus stayed on a greyed row"
        );
        assert!(
            tab_order(&state).contains(&state.focus),
            "focus {:?} is not a focusable control",
            state.focus
        );
    }

    #[test]
    fn av1_hardware_shows_the_greyed_two_pass_row_and_auto_vmaf_on_separate_lines() {
        let mut state = state_for(CodecSelection::Av1, true);
        state.gpu_vendor = GpuVendor::Intel;
        let text = render_text(&mut state);

        let two_pass = text.lines().position(|l| l.contains("2-Pass Encoding"));
        let vmaf = text
            .lines()
            .position(|l| l.contains("Auto-VMAF Quality Calibration"));
        assert!(two_pass.is_some(), "greyed 2-Pass row is missing");
        assert!(vmaf.is_some(), "Auto-VMAF row is missing");
        assert_ne!(two_pass, vmaf);
    }

    #[test]
    fn hevc_software_shows_x265_controls_and_hardware_greys_them() {
        let mut sw = state_for(CodecSelection::Hevc, false);
        let text = render_text(&mut sw);
        assert!(text.contains("Preset:"), "x265 preset row missing");
        assert!(text.contains("Bit Depth:"));
        for f in [HevcX265PresetDropdown, HevcX265TuneDropdown] {
            assert!(f.area(&sw).is_some(), "{f:?} should be clickable");
            assert!(tab_order(&sw).contains(&f), "{f:?} should be in Tab order");
        }
        // VP9-only rows are greyed for HEVC
        assert!(RowMtCheckbox.area(&sw).is_none());

        let mut hw = state_for(CodecSelection::Hevc, true);
        hw.gpu_vendor = GpuVendor::Nvidia;
        render_text(&mut hw);
        for f in [HevcX265PresetDropdown, HevcX265TuneDropdown] {
            assert!(
                f.area(&hw).is_none(),
                "{f:?} must have no hit area on hardware"
            );
            assert!(!tab_order(&hw).contains(&f), "{f:?} must be skipped by Tab");
        }
    }

    const NVENC_CONTROLS: [ConfigFocus; 4] = [
        HevcNvencPresetSlider,
        HevcNvencCqSlider,
        HevcNvencLookaheadInput,
        HevcNvencBFramesInput,
    ];

    #[test]
    fn hevc_nvenc_controls_apply_only_to_hevc_on_nvidia_hardware() {
        for f in NVENC_CONTROLS {
            assert!(applies(f, CodecSelection::Hevc, NVIDIA), "{f:?}");
            assert!(!applies(f, CodecSelection::Hevc, SW), "{f:?}");
            assert!(!applies(f, CodecSelection::Hevc, INTEL), "{f:?}");
            assert!(!applies(f, CodecSelection::Hevc, AMD), "{f:?}");
            assert!(!applies(f, CodecSelection::Av1, NVIDIA), "{f:?}");
            assert!(!applies(f, CodecSelection::Vp9, NVIDIA), "{f:?}");
        }
    }

    #[test]
    fn hevc_nvenc_shows_only_its_settings() {
        let mut nv = state_for(CodecSelection::Hevc, true);
        nv.gpu_vendor = GpuVendor::Nvidia;
        let text = render_text(&mut nv);
        assert!(text.contains("NVENC Preset"), "{text}");
        assert!(text.contains("NVENC CQ"), "{text}");
        for f in NVENC_CONTROLS {
            assert!(f.area(&nv).is_some(), "{f:?} should be clickable");
            assert!(tab_order(&nv).contains(&f), "{f:?} should be in Tab order");
        }
        // libx265 and AV1 hardware controls stay off
        assert!(HevcX265PresetDropdown.area(&nv).is_none());
        assert!(Av1HwCqSlider.area(&nv).is_none());

        // Software HEVC and non-NVIDIA hardware hide them
        let mut sw = state_for(CodecSelection::Hevc, false);
        render_text(&mut sw);
        let mut intel = state_for(CodecSelection::Hevc, true);
        intel.gpu_vendor = GpuVendor::Intel;
        render_text(&mut intel);
        for f in NVENC_CONTROLS {
            assert!(f.area(&sw).is_none(), "{f:?} on software");
            assert!(f.area(&intel).is_none(), "{f:?} on Intel");
            assert!(!tab_order(&sw).contains(&f));
        }
    }

    #[test]
    fn hevc_amf_and_videotoolbox_show_only_their_own_controls() {
        let amf_only = [HevcAmfQpSlider, HevcAmfPresetSlider];
        let vt_only = [HevcVideotoolboxQSlider];

        let mut amd = state_for(CodecSelection::Hevc, true);
        amd.gpu_vendor = GpuVendor::Amd;
        let text = render_text(&mut amd);
        assert!(text.contains("QP (Lower=Better"), "{text}");
        for f in amf_only {
            assert!(f.area(&amd).is_some(), "{f:?} should be clickable on AMD");
            assert!(tab_order(&amd).contains(&f), "{f:?} should be in Tab order");
        }
        for f in vt_only.into_iter().chain([HevcX265PresetDropdown]) {
            assert!(f.area(&amd).is_none(), "{f:?} must not show on AMD");
        }

        let mut apple = state_for(CodecSelection::Hevc, true);
        apple.gpu_vendor = GpuVendor::Apple;
        let text = render_text(&mut apple);
        assert!(text.contains("Quality (Higher=Better"), "{text}");
        for f in vt_only {
            assert!(
                f.area(&apple).is_some(),
                "{f:?} should be clickable on Apple"
            );
            assert!(
                tab_order(&apple).contains(&f),
                "{f:?} should be in Tab order"
            );
        }
        for f in amf_only {
            assert!(f.area(&apple).is_none(), "{f:?} must not show on Apple");
        }

        // Neither shows for other vendors, software, or other codecs
        let mut nv = state_for(CodecSelection::Hevc, true);
        nv.gpu_vendor = GpuVendor::Nvidia;
        render_text(&mut nv);
        let mut sw = state_for(CodecSelection::Hevc, false);
        render_text(&mut sw);
        for f in amf_only.into_iter().chain(vt_only) {
            assert!(f.area(&nv).is_none(), "{f:?}");
            assert!(f.area(&sw).is_none(), "{f:?}");
            assert!(!applies(f, CodecSelection::Av1, AMD), "{f:?}");
            assert!(!applies(
                f,
                CodecSelection::Av1,
                Backend::Hardware(GpuVendor::Apple)
            ));
        }
    }

    #[test]
    fn applies_matches_flags_the_hevc_amf_and_videotoolbox_builders_emit() {
        let amf = emitted_flags(&software_cmd(
            &exercised(Profile::get("hevc-amf")),
            VideoEncoder::HevcAmf,
        ));
        let amd = Backend::Hardware(GpuVendor::Amd);
        let emitted = |args: &[String], flag: &str| args.iter().any(|a| a == flag);
        assert_eq!(
            applies(HevcAmfQpSlider, CodecSelection::Hevc, amd),
            emitted(&amf, "-qp_i")
        );
        assert_eq!(
            applies(HevcAmfPresetSlider, CodecSelection::Hevc, amd),
            emitted(&amf, "-quality")
        );
        assert!(!emitted(&amf, "-look_ahead") && !emitted(&amf, "-bf"));

        let vt = emitted_flags(&software_cmd(
            &exercised(Profile::get("hevc-videotoolbox")),
            VideoEncoder::HevcVideotoolbox,
        ));
        let apple = Backend::Hardware(GpuVendor::Apple);
        assert_eq!(
            applies(HevcVideotoolboxQSlider, CodecSelection::Hevc, apple),
            emitted(&vt, "-q:v")
        );
    }

    #[test]
    fn hevc_vaapi_controls_apply_only_to_hevc_vaapi_hardware() {
        for f in [HevcVaapiQualitySlider, HevcVaapiBFramesInput] {
            assert!(applies(f, CodecSelection::Hevc, UNKNOWN), "{f:?}");
            assert!(!applies(f, CodecSelection::Hevc, SW), "{f:?}");
            assert!(!applies(f, CodecSelection::Hevc, NVIDIA), "{f:?}");
            assert!(!applies(f, CodecSelection::Av1, UNKNOWN), "{f:?}");
            assert!(!applies(f, CodecSelection::Vp9, UNKNOWN), "{f:?}");
        }
    }

    #[test]
    fn hevc_vaapi_shows_only_the_settings_hevc_vaapi_supports() {
        let mut hw = state_for(CodecSelection::Hevc, true);
        hw.gpu_vendor = GpuVendor::Unknown;
        let text = render_text(&mut hw);
        assert!(text.contains("QP (1-51"), "VAAPI quality row missing");
        assert!(text.contains("B-Frames"), "VAAPI b-frames row missing");
        for f in [HevcVaapiQualitySlider, HevcVaapiBFramesInput] {
            assert!(f.area(&hw).is_some(), "{f:?} should be clickable");
            assert!(tab_order(&hw).contains(&f), "{f:?} should be in Tab order");
        }
        // x265 settings and the preset/lookahead hevc_vaapi lacks are not offered
        for f in [HevcX265PresetDropdown, HevcX265TuneDropdown] {
            assert!(f.area(&hw).is_none(), "{f:?}");
        }
        assert!(!text.contains("Lookahead"));

        let mut sw = state_for(CodecSelection::Hevc, false);
        let text = render_text(&mut sw);
        assert!(!text.contains("B-Frames"));
        assert!(HevcVaapiQualitySlider.area(&sw).is_none());
    }

    #[test]
    fn the_other_codecs_block_collapses_into_one_dimmed_line() {
        let mut vp9 = state_for(CodecSelection::Vp9, false);
        let text = render_text(&mut vp9);
        assert!(text.contains("AV1 options (select AV1 to edit)"));
        assert!(!text.contains("VP9 options (select VP9 to edit)"));

        let mut av1 = state_for(CodecSelection::Av1, false);
        let text = render_text(&mut av1);
        assert!(text.contains("VP9 options (select VP9 to edit)"));
        assert!(!text.contains("AV1 options (select AV1 to edit)"));
    }

    #[test]
    fn hardware_keeps_max_workers_but_greys_threads_and_software_options() {
        for codec in [CodecSelection::Vp9, CodecSelection::Av1] {
            let mut state = state_for(codec, true);
            state.gpu_vendor = GpuVendor::Intel;
            let text = render_text(&mut state);

            assert!(text.contains("Threads"), "Threads should still be drawn");
            assert!(
                ThreadsInput.area(&state).is_none(),
                "{codec:?}: Threads is software-only"
            );
            assert!(RowMtCheckbox.area(&state).is_none(), "{codec:?}");
            assert!(
                MaxWorkersInput.area(&state).is_some(),
                "{codec:?}: Max Workers is shared"
            );
            assert!(
                GopLengthInput.area(&state).is_some(),
                "{codec:?}: GOP length is shared"
            );
        }
    }

    #[test]
    fn shared_controls_always_apply() {
        for f in [
            GopLengthInput,
            MaxWorkersInput,
            ColorSpacePresetDropdown,
            PixFmtDropdown,
            VideoCodecDropdown,
            HardwareEncodingCheckbox,
            AudioPrimaryCodec,
            AdditionalArgsInput,
        ] {
            for codec in [CodecSelection::Vp9, CodecSelection::Av1] {
                for backend in [SW, INTEL, NVIDIA, AMD] {
                    assert!(applies(f, codec, backend), "{f:?} {codec:?} {backend:?}");
                }
            }
        }
    }
}
