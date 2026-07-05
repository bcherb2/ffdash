// Behavioral round-trip tests for dropdown-backed fields.
// Ensures dropdown selections survive Profile save/load and stay in sync with backing values.

use ffdash::engine::Profile;
use ffdash::ui::{
    options,
    state::{AudioPrimaryCodec, ConfigState},
};

#[test]
fn colorspace_preset_roundtrip_preserves_all_values() {
    // Test all 3 presets: Auto, SDR, HDR10
    for idx in 0..3 {
        let mut config = ConfigState::default();
        config.colorspace_preset_state.select(Some(idx));
        let preset = options::colorspace_preset_from_idx(idx);
        config.colorspace_preset = preset;

        // Set numeric values to match preset
        let (cs, cp, ct, cr) = options::colorspace_preset_to_values(preset);
        config.colorspace = cs;
        config.color_primaries = cp;
        config.color_trc = ct;
        config.color_range = cr;

        let profile = Profile::from_config("test".to_string(), &config);
        let mut restored = ConfigState::default();
        profile.apply_to_config(&mut restored);

        // Verify preset state is restored
        assert_eq!(restored.colorspace_preset_state.selected(), Some(idx));
        assert_eq!(restored.colorspace_preset, preset);

        // Verify all numeric values are preserved
        assert_eq!(restored.colorspace, cs);
        assert_eq!(restored.color_primaries, cp);
        assert_eq!(restored.color_trc, ct);
        assert_eq!(restored.color_range, cr);
    }
}

#[test]
fn arnr_type_roundtrip_preserves_dropdown_and_value() {
    for idx in 0..4 {
        let mut config = ConfigState::default();
        config.arnr_type_state.select(Some(idx));
        config.arnr_type = options::arnr_type_from_idx(idx);

        let profile = Profile::from_config("test".to_string(), &config);
        let mut restored = ConfigState::default();
        profile.apply_to_config(&mut restored);

        assert_eq!(restored.arnr_type_state.selected(), Some(idx));
        assert_eq!(restored.arnr_type, options::arnr_type_from_idx(idx));
    }
}

#[test]
fn fps_roundtrip_preserves_dropdown_and_value() {
    for idx in 0..11 {
        let mut config = ConfigState::default();
        config.fps_dropdown_state.select(Some(idx));
        config.fps = options::fps_from_idx(idx);

        let profile = Profile::from_config("test".to_string(), &config);
        let mut restored = ConfigState::default();
        profile.apply_to_config(&mut restored);

        let expected_idx = options::fps_to_idx(options::fps_from_idx(idx));
        assert_eq!(restored.fps_dropdown_state.selected(), Some(expected_idx));
        assert_eq!(restored.fps, options::fps_from_idx(idx));
    }
}

#[test]
fn resolution_roundtrip_preserves_dropdown_and_values() {
    for idx in 0..7 {
        let mut config = ConfigState::default();
        config.resolution_dropdown_state.select(Some(idx));
        let (w, h) = options::resolution_from_idx(idx);
        config.scale_width = w;
        config.scale_height = h;

        let profile = Profile::from_config("test".to_string(), &config);
        let mut restored = ConfigState::default();
        profile.apply_to_config(&mut restored);

        assert_eq!(
            restored.resolution_dropdown_state.selected(),
            Some(options::resolution_to_idx(
                restored.scale_width,
                restored.scale_height
            ))
        );
        assert_eq!((restored.scale_width, restored.scale_height), (w, h));
    }
}

#[test]
fn pix_fmt_roundtrip_preserves_selection() {
    for idx in 0..2 {
        let mut config = ConfigState::default();
        config.pix_fmt_state.select(Some(idx));

        let profile = Profile::from_config("test".to_string(), &config);
        let mut restored = ConfigState::default();
        profile.apply_to_config(&mut restored);

        assert_eq!(restored.pix_fmt_state.selected(), Some(idx));
    }
}

#[test]
fn quality_mode_roundtrip_preserves_selection() {
    for idx in 0..3 {
        let mut config = ConfigState::default();
        config.quality_mode_state.select(Some(idx));

        let profile = Profile::from_config("test".to_string(), &config);
        let mut restored = ConfigState::default();
        profile.apply_to_config(&mut restored);

        assert_eq!(restored.quality_mode_state.selected(), Some(idx));
    }
}

#[test]
fn tune_content_roundtrip_preserves_selection() {
    for idx in 0..3 {
        let mut config = ConfigState::default();
        config.tune_content_state.select(Some(idx));

        let profile = Profile::from_config("test".to_string(), &config);
        let mut restored = ConfigState::default();
        profile.apply_to_config(&mut restored);

        assert_eq!(restored.tune_content_state.selected(), Some(idx));
    }
}

#[test]
fn aq_mode_roundtrip_preserves_selection_and_value() {
    for idx in 0..6 {
        let mut config = ConfigState::default();
        config.aq_mode_state.select(Some(idx));

        let profile = Profile::from_config("test".to_string(), &config);
        let mut restored = ConfigState::default();
        profile.apply_to_config(&mut restored);

        assert_eq!(restored.aq_mode_state.selected(), Some(idx));
        assert_eq!(
            options::aq_mode_from_idx(idx),
            options::aq_mode_from_idx(restored.aq_mode_state.selected().unwrap())
        );
    }
}

#[test]
fn audio_codec_roundtrip_preserves_selection() {
    for idx in 0..5 {
        let mut config = ConfigState::default();
        config.audio_primary_codec_state.select(Some(idx));
        config.audio_primary_codec = AudioPrimaryCodec::from_index(idx);

        let profile = Profile::from_config("test".to_string(), &config);
        let mut restored = ConfigState::default();
        profile.apply_to_config(&mut restored);

        assert_eq!(restored.audio_primary_codec_state.selected(), Some(idx));
    }
}

#[test]
fn container_roundtrip_preserves_selection() {
    for idx in 0..4 {
        let mut config = ConfigState::default();
        config.container_dropdown_state.select(Some(idx));

        let profile = Profile::from_config("test".to_string(), &config);
        let mut restored = ConfigState::default();
        profile.apply_to_config(&mut restored);

        assert_eq!(restored.container_dropdown_state.selected(), Some(idx));
    }
}
