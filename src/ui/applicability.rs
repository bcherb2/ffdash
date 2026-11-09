// Which config controls apply to the selected Encoder.
//
// Controls that don't apply are drawn greyed out and are not focusable or
// clickable (they get no hit area). Render code and the tests that cross-check
// the ffmpeg command builders both use `applies`, so the UI cannot drift from
// what is actually passed to the encoder. The Encoder is the one
// `select_encoder` picks from Encoder Availability, never a guess from the GPU vendor.

use crate::engine::hardware::{Backend, VideoEncoder};
use crate::engine::selection::CodecKind;
use crate::ui::focus::ConfigFocus;

/// Whether `field` has any effect when encoding with `encoder`.
pub fn applies(field: ConfigFocus, encoder: VideoEncoder) -> bool {
    use ConfigFocus::*;

    let codec = encoder.codec();
    let vp9 = codec == CodecKind::Vp9;
    let av1 = codec == CodecKind::Av1;
    let hevc = codec == CodecKind::Hevc;
    let software = !encoder.is_hardware();
    let hardware = encoder.is_hardware();

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
        // own focus variants here with `encoder == VideoEncoder::<Encoder>`.
        HevcX265PresetDropdown | HevcX265TuneDropdown => encoder == VideoEncoder::Libx265,
        // hevc_amf: constant QP and the shared hardware preset (no lookahead or B-frames)
        HevcAmfQpSlider | HevcAmfPresetSlider => encoder == VideoEncoder::HevcAmf,
        // hevc_videotoolbox: its `q:v` quality is the only setting
        HevcVideotoolboxQSlider => encoder == VideoEncoder::HevcVideotoolbox,

        // hevc_nvenc
        HevcNvencPresetSlider
        | HevcNvencCqSlider
        | HevcNvencLookaheadInput
        | HevcNvencBFramesInput => encoder == VideoEncoder::HevcNvenc,
        // hevc_qsv: quality, preset, lookahead and b-frames (Intel Quick Sync)
        HevcQsvQualityInput | HevcQsvPresetInput | HevcQsvLookaheadInput | HevcQsvBFramesInput => {
            encoder == VideoEncoder::HevcQsv
        }

        // hevc_vaapi: quality (QP) and B-frames
        HevcVaapiQualitySlider | HevcVaapiBFramesInput => encoder == VideoEncoder::HevcVaapi,

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
            encoder == VideoEncoder::Vp9Qsv
        }

        // VPP denoise/detail: QSV and VAAPI, not NVENC
        HwDenoiseInput | HwDetailInput => !hevc && hardware && encoder.backend() != Backend::Nvenc,

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
    use crate::engine::selection::EncoderAvailability;
    use crate::ui::state::CodecSelection;
    use ConfigFocus::*;
    use VideoEncoder as E;

    const VP9_SW: VideoEncoder = E::LibvpxVp9;
    const AV1_SW: VideoEncoder = E::LibsvtAv1;
    const HEVC_SW: VideoEncoder = E::Libx265;

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
            assert!(applies(f, VP9_SW), "{f:?} should apply to VP9 software");
            assert!(
                !applies(f, AV1_SW),
                "{f:?} should not apply to AV1 software"
            );
            assert!(
                !applies(f, E::Vp9Qsv),
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
            assert!(applies(f, AV1_SW), "{f:?}");
            assert!(!applies(f, VP9_SW), "{f:?}");
            assert!(!applies(f, E::Av1Qsv), "{f:?}");
        }
    }

    #[test]
    fn av1_hardware_controls_apply_to_every_av1_hardware_encoder_only() {
        for f in [
            Av1HwPresetSlider,
            Av1HwCqSlider,
            Av1HwLookaheadInput,
            Av1HwTileColsInput,
            Av1HwTileRowsInput,
        ] {
            for enc in [E::Av1Nvenc, E::Av1Qsv, E::Av1Vaapi, E::Av1Amf] {
                assert!(applies(f, enc), "{f:?} {enc:?}");
            }
            assert!(!applies(f, AV1_SW), "{f:?}");
            assert!(!applies(f, E::Vp9Qsv), "{f:?}");
            assert!(!applies(f, E::HevcNvenc), "{f:?}");
        }
    }

    #[test]
    fn vp9_qsv_controls_need_the_vp9_qsv_encoder() {
        for f in [Vp9QsvLookaheadCheckbox, Vp9QsvLookaheadDepthInput] {
            assert!(applies(f, E::Vp9Qsv), "{f:?}");
            assert!(!applies(f, E::Vp9Vaapi), "{f:?}");
            assert!(!applies(f, E::Av1Qsv), "{f:?}");
            assert!(!applies(f, E::HevcQsv), "{f:?}");
            assert!(!applies(f, VP9_SW), "{f:?}");
        }
    }

    #[test]
    fn vpp_filters_apply_to_qsv_and_vaapi_encoders_of_vp9_and_av1() {
        for f in [HwDenoiseInput, HwDetailInput] {
            for enc in [E::Vp9Qsv, E::Vp9Vaapi, E::Av1Qsv, E::Av1Vaapi, E::Av1Amf] {
                assert!(applies(f, enc), "{f:?} {enc:?}");
            }
            for enc in [
                E::Av1Nvenc,
                VP9_SW,
                AV1_SW,
                HEVC_SW,
                E::HevcQsv,
                E::HevcVaapi,
            ] {
                assert!(!applies(f, enc), "{f:?} {enc:?}");
            }
        }
    }

    #[test]
    fn threads_apply_to_software_encoders_only() {
        for enc in [VP9_SW, AV1_SW, HEVC_SW] {
            assert!(applies(ThreadsInput, enc), "{enc:?}");
        }
        for enc in [E::Vp9Qsv, E::Av1Nvenc, E::HevcQsv, E::HevcVaapi] {
            assert!(!applies(ThreadsInput, enc), "{enc:?}");
        }
    }

    /// Controls that belong to exactly one HEVC Encoder, keyed by that Encoder.
    const HEVC_CONTROLS: &[(VideoEncoder, &[ConfigFocus])] = &[
        (HEVC_SW, &[HevcX265PresetDropdown, HevcX265TuneDropdown]),
        (
            E::HevcNvenc,
            &[
                HevcNvencPresetSlider,
                HevcNvencCqSlider,
                HevcNvencLookaheadInput,
                HevcNvencBFramesInput,
            ],
        ),
        (
            E::HevcQsv,
            &[
                HevcQsvQualityInput,
                HevcQsvPresetInput,
                HevcQsvLookaheadInput,
                HevcQsvBFramesInput,
            ],
        ),
        (
            E::HevcVaapi,
            &[HevcVaapiQualitySlider, HevcVaapiBFramesInput],
        ),
        (E::HevcAmf, &[HevcAmfQpSlider, HevcAmfPresetSlider]),
        (E::HevcVideotoolbox, &[HevcVideotoolboxQSlider]),
    ];

    #[test]
    fn each_hevc_encoder_owns_its_controls_and_no_other_encoder_shows_them() {
        let every_encoder = [
            VP9_SW,
            E::Vp9Qsv,
            E::Vp9Vaapi,
            AV1_SW,
            E::LibaomAv1,
            E::Av1Qsv,
            E::Av1Nvenc,
            E::Av1Vaapi,
            E::Av1Amf,
            HEVC_SW,
            E::HevcNvenc,
            E::HevcQsv,
            E::HevcVaapi,
            E::HevcAmf,
            E::HevcVideotoolbox,
        ];
        for (owner, controls) in HEVC_CONTROLS {
            for f in *controls {
                for enc in every_encoder {
                    assert_eq!(applies(*f, enc), enc == *owner, "{f:?} on {enc:?}");
                }
            }
        }
    }

    #[test]
    fn vp9_and_av1_controls_are_greyed_for_every_hevc_encoder() {
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
            for enc in [
                HEVC_SW,
                E::HevcNvenc,
                E::HevcQsv,
                E::HevcVaapi,
                E::HevcAmf,
                E::HevcVideotoolbox,
            ] {
                assert!(!applies(f, enc), "{f:?} {enc:?}");
            }
        }
        // shared controls stay active
        for f in [GopLengthInput, PixFmtDropdown, ColorSpacePresetDropdown] {
            assert!(applies(f, HEVC_SW), "{f:?}");
            assert!(applies(f, E::HevcNvenc), "{f:?}");
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
            for enc in [VP9_SW, E::Vp9Qsv, AV1_SW, E::Av1Nvenc, E::Av1Qsv, E::Av1Amf] {
                assert!(applies(f, enc), "{f:?} {enc:?}");
            }
        }
    }

    // ---- Cross-check against the real ffmpeg command builders -------------

    use crate::engine::core::{Codec, Profile, VideoJob};
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

    fn assert_matches_builder(args: &[String], encoder: VideoEncoder) {
        for (field, flag) in FLAGS {
            let stream_flag = format!("{flag}:v");
            let emitted = args.iter().any(|a| a == flag || *a == stream_flag);
            assert_eq!(
                applies(*field, encoder),
                emitted,
                "{field:?} ({flag}) on {encoder:?}: applies() disagrees with the builder (emitted = {emitted})"
            );
        }
    }

    #[test]
    fn applies_matches_flags_the_vp9_software_builder_emits() {
        let profile = exercised(Profile::get("vp9-good"));
        let args = emitted_flags(&software_cmd(&profile, VP9_SW));
        assert_matches_builder(&args, VP9_SW);
    }

    #[test]
    fn applies_matches_flags_the_av1_software_builder_emits() {
        let profile = exercised(Profile::get("av1-svt"));
        let args = emitted_flags(&software_cmd(&profile, AV1_SW));
        assert_matches_builder(&args, AV1_SW);
    }

    #[test]
    fn applies_matches_flags_the_hevc_software_builder_emits() {
        let profile = exercised(Profile::get("hevc-x265"));
        let args = emitted_flags(&software_cmd(&profile, HEVC_SW));
        assert_matches_builder(&args, HEVC_SW);
    }

    #[test]
    fn applies_matches_flags_the_hevc_amf_and_videotoolbox_builders_emit() {
        let amf = emitted_flags(&software_cmd(
            &exercised(Profile::get("hevc-amf")),
            E::HevcAmf,
        ));
        let emitted = |args: &[String], flag: &str| args.iter().any(|a| a == flag);
        assert_eq!(applies(HevcAmfQpSlider, E::HevcAmf), emitted(&amf, "-qp_i"));
        assert_eq!(
            applies(HevcAmfPresetSlider, E::HevcAmf),
            emitted(&amf, "-quality")
        );
        assert!(!emitted(&amf, "-look_ahead") && !emitted(&amf, "-bf"));

        let vt = emitted_flags(&software_cmd(
            &exercised(Profile::get("hevc-videotoolbox")),
            E::HevcVideotoolbox,
        ));
        assert_eq!(
            applies(HevcVideotoolboxQSlider, E::HevcVideotoolbox),
            emitted(&vt, "-q:v")
        );
    }

    // ---- Rendering: controls follow the Encoder selected from availability --

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

    /// A config panel whose Encoder Availability is exactly `available` (each with
    /// Main10 support); `hardware` is the "use hardware" toggle.
    fn state_with(
        codec: CodecSelection,
        hardware: bool,
        available: &[VideoEncoder],
    ) -> ConfigState {
        let mut availability = EncoderAvailability::default();
        for e in available {
            availability = availability.with(*e, true);
        }
        ConfigState {
            codec_selection: codec,
            use_hardware_encoding: hardware,
            encoder_availability: availability,
            ..ConfigState::default()
        }
    }

    fn state_for(codec: CodecSelection, hardware: bool) -> ConfigState {
        state_with(codec, hardware, &[])
    }

    #[test]
    fn selected_encoder_follows_availability_not_gpu_vendor() {
        let hevc = CodecSelection::Hevc;
        assert_eq!(
            state_with(hevc, true, &[E::HevcQsv]).selected_encoder(),
            E::HevcQsv
        );
        assert_eq!(
            state_with(hevc, true, &[E::HevcVaapi]).selected_encoder(),
            E::HevcVaapi
        );
        assert_eq!(
            state_with(hevc, true, &[E::HevcAmf]).selected_encoder(),
            E::HevcAmf
        );
        // NVENC outranks QSV when both are available
        assert_eq!(
            state_with(hevc, true, &[E::HevcQsv, E::HevcNvenc]).selected_encoder(),
            E::HevcNvenc
        );
        // Toggle off, or nothing available: the Software Fallback
        assert_eq!(
            state_with(hevc, false, &[E::HevcQsv]).selected_encoder(),
            HEVC_SW
        );
        assert_eq!(state_with(hevc, true, &[]).selected_encoder(), HEVC_SW);
        // AV1 and VP9 too
        assert_eq!(
            state_with(CodecSelection::Av1, true, &[E::Av1Vaapi, E::Av1Amf]).selected_encoder(),
            E::Av1Amf
        );
        assert_eq!(
            state_with(CodecSelection::Vp9, true, &[E::Vp9Vaapi]).selected_encoder(),
            E::Vp9Vaapi
        );
    }

    #[test]
    fn ten_bit_output_skips_an_encoder_without_main10() {
        let availability = EncoderAvailability::default()
            .with(E::HevcNvenc, false)
            .with(E::HevcQsv, true);
        let mut state = ConfigState {
            codec_selection: CodecSelection::Hevc,
            use_hardware_encoding: true,
            encoder_availability: availability,
            ..ConfigState::default()
        };
        assert_eq!(state.selected_encoder(), E::HevcNvenc);
        state.pix_fmt_state.select(Some(2)); // yuv420p10le
        assert_eq!(state.selected_encoder(), E::HevcQsv);
    }

    #[test]
    fn hevc_settings_shown_are_those_of_the_selected_encoder() {
        let cases: [(VideoEncoder, &[ConfigFocus]); 5] = [
            (E::HevcNvenc, &[HevcNvencPresetSlider, HevcNvencCqSlider]),
            (E::HevcQsv, &[HevcQsvQualityInput, HevcQsvBFramesInput]),
            (
                E::HevcVaapi,
                &[HevcVaapiQualitySlider, HevcVaapiBFramesInput],
            ),
            (E::HevcAmf, &[HevcAmfQpSlider, HevcAmfPresetSlider]),
            (E::HevcVideotoolbox, &[HevcVideotoolboxQSlider]),
        ];
        for (enc, own) in cases {
            let mut state = state_with(CodecSelection::Hevc, true, &[enc]);
            render_text(&mut state);
            for f in own {
                assert!(
                    f.area(&state).is_some(),
                    "{f:?} should be clickable on {enc:?}"
                );
                assert!(
                    tab_order(&state).contains(f),
                    "{f:?} should be in Tab order on {enc:?}"
                );
            }
            // Nothing owned by another Encoder is focusable
            for (other, controls) in HEVC_CONTROLS {
                if *other == enc {
                    continue;
                }
                for f in *controls {
                    assert!(f.area(&state).is_none(), "{f:?} must not show on {enc:?}");
                    assert!(
                        !tab_order(&state).contains(f),
                        "{f:?} in Tab order on {enc:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn hevc_software_shows_x265_controls() {
        // Hardware available but toggled off: libx265's controls
        let mut sw = state_with(CodecSelection::Hevc, false, &[E::HevcNvenc]);
        let text = render_text(&mut sw);
        assert!(text.contains("Preset:"), "x265 preset row missing");
        assert!(text.contains("Bit Depth:"));
        for f in [HevcX265PresetDropdown, HevcX265TuneDropdown] {
            assert!(f.area(&sw).is_some(), "{f:?} should be clickable");
            assert!(tab_order(&sw).contains(&f), "{f:?} should be in Tab order");
        }
        // VP9-only rows are greyed for HEVC
        assert!(RowMtCheckbox.area(&sw).is_none());
        // Toggle on with nothing available also falls back to libx265
        let mut none = state_with(CodecSelection::Hevc, true, &[]);
        render_text(&mut none);
        assert!(HevcX265PresetDropdown.area(&none).is_some());
    }

    #[test]
    fn hevc_vaapi_only_machine_shows_vaapi_settings_whatever_the_gpu() {
        let mut hw = state_with(CodecSelection::Hevc, true, &[E::HevcVaapi]);
        let text = render_text(&mut hw);
        assert!(text.contains("QP (1-51"), "VAAPI quality row missing");
        assert!(text.contains("B-Frames"), "VAAPI b-frames row missing");
        assert!(!text.contains("Lookahead"));
        assert!(HevcX265PresetDropdown.area(&hw).is_none());
    }

    #[test]
    fn hardware_toggle_is_labelled_with_the_encoder_it_would_select() {
        let label = |codec, available: &[VideoEncoder]| {
            let mut s = state_with(codec, false, available);
            render_text(&mut s)
        };
        assert!(label(CodecSelection::Hevc, &[E::HevcQsv]).contains("Intel Quick Sync"));
        assert!(label(CodecSelection::Hevc, &[E::HevcAmf]).contains("AMD AMF"));
        assert!(label(CodecSelection::Hevc, &[E::HevcNvenc]).contains("NVIDIA NVENC"));
        assert!(label(CodecSelection::Hevc, &[E::HevcVaapi]).contains("(VAAPI)"));
        assert!(label(CodecSelection::Hevc, &[E::HevcVideotoolbox]).contains("VideoToolbox"));
        assert!(label(CodecSelection::Av1, &[E::Av1Vaapi]).contains("(VAAPI)"));
        assert!(label(CodecSelection::Vp9, &[E::Vp9Qsv]).contains("Intel Quick Sync"));
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
        let mut state = state_with(CodecSelection::Av1, true, &[E::Av1Qsv]);
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
    fn av1_hardware_cq_edits_the_selected_encoders_own_field() {
        let mut nv = state_with(CodecSelection::Av1, true, &[E::Av1Nvenc, E::Av1Vaapi]);
        nv.set_av1_hw_cq(7);
        assert_eq!((nv.av1_hw_cq(), nv.av1_nvenc_cq), ((0, 63, 7), 7));
        let mut va = state_with(CodecSelection::Av1, true, &[E::Av1Vaapi]);
        va.set_av1_hw_cq(9);
        assert_eq!((va.av1_hw_cq(), va.av1_vaapi_cq), ((1, 255, 9), 9));
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
        for (codec, enc) in [
            (CodecSelection::Vp9, E::Vp9Qsv),
            (CodecSelection::Av1, E::Av1Qsv),
        ] {
            let mut state = state_with(codec, true, &[enc]);
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
}
