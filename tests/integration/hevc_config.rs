// HEVC in the UI/profile layer: the codec dropdown, profile round trip, Container rules.

use ffdash::engine::core::derive_output_path;
use ffdash::engine::{Codec, Profile};
use ffdash::ui::{
    constants, options,
    state::{CodecSelection, ConfigState},
};

fn hevc_state() -> ConfigState {
    let mut s = ConfigState::default();
    s.codec_selection = CodecSelection::Hevc;
    s.video_codec_state
        .select(Some(options::codec_selection_to_idx(CodecSelection::Hevc)));
    s
}

#[test]
fn codec_dropdown_offers_hevc() {
    assert_eq!(constants::VIDEO_CODECS, &["VP9", "AV1", "HEVC"]);
    assert_eq!(options::codec_selection_from_idx(2), CodecSelection::Hevc);
    assert_eq!(options::codec_selection_to_idx(CodecSelection::Hevc), 2);
}

#[test]
fn hevc_profile_roundtrip_preserves_every_setting() {
    let mut config = hevc_state();
    config.hevc.qsv_quality = 21;
    config.hevc.nvenc_quality = 22;
    config.hevc.vaapi_quality = 23;
    config.hevc.amf_qp = 24;
    config.hevc.videotoolbox_q = 77;
    config.hevc.hw_b_frames = 3;
    config
        .x265_preset_state
        .select(Some(options::x265_preset_to_idx("slow")));
    config
        .x265_tune_state
        .select(Some(options::x265_tune_to_idx("grain")));
    config.crf = 19;

    let profile = Profile::from_config("test".to_string(), &config);
    let hevc = profile.codec.as_hevc().expect("HEVC codec");
    assert_eq!(hevc.x265_crf, 19);
    assert_eq!(hevc.x265_preset, "slow");
    assert_eq!(hevc.x265_tune, "grain");

    let mut restored = ConfigState::default();
    profile.apply_to_config(&mut restored);
    assert_eq!(restored.codec_selection, CodecSelection::Hevc);
    assert_eq!(restored.video_codec_state.selected(), Some(2));
    assert_eq!(restored.crf, 19);
    assert_eq!(restored.hevc.qsv_quality, 21);
    assert_eq!(restored.hevc.nvenc_quality, 22);
    assert_eq!(restored.hevc.vaapi_quality, 23);
    assert_eq!(restored.hevc.amf_qp, 24);
    assert_eq!(restored.hevc.videotoolbox_q, 77);
    assert_eq!(restored.hevc.hw_b_frames, 3);
    assert_eq!(
        options::x265_preset_from_idx(restored.x265_preset_state.selected().unwrap()),
        "slow"
    );
    assert_eq!(
        options::x265_tune_from_idx(restored.x265_tune_state.selected().unwrap()),
        "grain"
    );
}

#[test]
fn hevc_profile_serializes_with_its_own_codec_tag() {
    let json = serde_json::to_string(&Profile::get("hevc-x265")).unwrap();
    assert!(json.contains("\"codec_type\":\"Hevc\""), "{json}");
    let back: Profile = serde_json::from_str(&json).unwrap();
    assert!(matches!(back.codec, Codec::Hevc(_)));
}

#[test]
fn hevc_defaults_to_mkv_and_webm_is_not_allowed() {
    assert_eq!(Profile::get("hevc-x265").container, "mkv");
    assert!(!options::container_allowed(CodecSelection::Hevc, "webm"));
    for c in ["mkv", "mp4", "avi"] {
        assert!(options::container_allowed(CodecSelection::Hevc, c));
    }
    assert!(options::container_allowed(CodecSelection::Vp9, "webm"));
}

#[test]
fn selecting_hevc_moves_a_webm_container_to_mkv() {
    let mut config = ConfigState::default();
    config
        .container_dropdown_state
        .select(Some(options::container_to_idx("webm")));
    config.codec_selection = CodecSelection::Hevc;
    config.enforce_container_for_codec();
    assert_eq!(
        options::container_from_idx(config.container_dropdown_state.selected().unwrap()),
        "mkv"
    );

    // MP4 is left alone
    config
        .container_dropdown_state
        .select(Some(options::container_to_idx("mp4")));
    config.enforce_container_for_codec();
    assert_eq!(
        options::container_from_idx(config.container_dropdown_state.selected().unwrap()),
        "mp4"
    );
}

#[test]
fn stepping_the_container_dropdown_skips_webm_for_hevc() {
    let hevc = CodecSelection::Hevc;
    // forward from avi wraps past webm to mp4; backward from mp4 wraps past webm to avi
    assert_eq!(
        options::next_container_idx(hevc, options::container_to_idx("avi"), true),
        1
    );
    assert_eq!(
        options::next_container_idx(hevc, options::container_to_idx("mp4"), false),
        3
    );
    // other codecs still reach webm
    assert_eq!(
        options::next_container_idx(CodecSelection::Av1, 1, false),
        0
    );
}

#[test]
fn output_path_for_hevc_never_uses_webm() {
    let p = derive_output_path(
        std::path::Path::new("/tmp/in.mp4"),
        "hevc-x265",
        None,
        None,
        Some("webm"),
    );
    assert_eq!(p.extension().unwrap(), "mkv");
    let p = derive_output_path(
        std::path::Path::new("/tmp/in.mp4"),
        "hevc-x265",
        None,
        None,
        Some("mp4"),
    );
    assert_eq!(p.extension().unwrap(), "mp4");
}
