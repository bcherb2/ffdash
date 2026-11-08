//! Encoder selection: a pure function of (Codec, use-hardware, preferred Encoder,
//! required bit depth, Encoder Availability), with availability built by hand.

use ffdash::engine::hardware::VideoEncoder::{self, *};
use ffdash::engine::selection::{CodecKind, EncoderAvailability, select_encoder};

fn avail(encoders: &[VideoEncoder]) -> EncoderAvailability {
    encoders
        .iter()
        .fold(EncoderAvailability::default(), |a, e| a.with(*e, true))
}

#[test]
fn av1_priority_is_nvenc_amf_qsv_vaapi_software() {
    let all = avail(&[Av1Vaapi, Av1Qsv, Av1Amf, Av1Nvenc]);
    assert_eq!(
        select_encoder(CodecKind::Av1, true, None, 8, &all),
        Av1Nvenc
    );
    let no_nvenc = avail(&[Av1Vaapi, Av1Qsv, Av1Amf]);
    assert_eq!(
        select_encoder(CodecKind::Av1, true, None, 8, &no_nvenc),
        Av1Amf
    );
    let qsv_vaapi = avail(&[Av1Vaapi, Av1Qsv]);
    assert_eq!(
        select_encoder(CodecKind::Av1, true, None, 8, &qsv_vaapi),
        Av1Qsv
    );
    let vaapi = avail(&[Av1Vaapi]);
    assert_eq!(
        select_encoder(CodecKind::Av1, true, None, 8, &vaapi),
        Av1Vaapi
    );
}

#[test]
fn vp9_priority_is_qsv_vaapi_software() {
    let both = avail(&[Vp9Vaapi, Vp9Qsv]);
    assert_eq!(select_encoder(CodecKind::Vp9, true, None, 8, &both), Vp9Qsv);
    let vaapi = avail(&[Vp9Vaapi]);
    assert_eq!(
        select_encoder(CodecKind::Vp9, true, None, 8, &vaapi),
        Vp9Vaapi
    );
}

#[test]
fn available_preference_wins_over_priority() {
    let all = avail(&[Av1Nvenc, Av1Qsv]);
    assert_eq!(
        select_encoder(CodecKind::Av1, true, Some(Av1Qsv), 8, &all),
        Av1Qsv
    );
}

#[test]
fn unavailable_preference_falls_back_to_priority() {
    let a = avail(&[Av1Nvenc]);
    assert_eq!(
        select_encoder(CodecKind::Av1, true, Some(Av1Qsv), 8, &a),
        Av1Nvenc
    );
}

#[test]
fn preference_for_another_codec_is_ignored() {
    let a = avail(&[Av1Nvenc, Vp9Qsv]);
    assert_eq!(
        select_encoder(CodecKind::Av1, true, Some(Vp9Qsv), 8, &a),
        Av1Nvenc
    );
}

#[test]
fn ten_bit_skips_encoders_without_main10() {
    let a = EncoderAvailability::default()
        .with(Av1Nvenc, false)
        .with(Av1Qsv, true);
    assert_eq!(select_encoder(CodecKind::Av1, true, None, 10, &a), Av1Qsv);
    assert_eq!(select_encoder(CodecKind::Av1, true, None, 8, &a), Av1Nvenc);
}

#[test]
fn ten_bit_preference_without_main10_is_not_honoured() {
    let a = EncoderAvailability::default()
        .with(Av1Nvenc, false)
        .with(Av1Vaapi, true);
    assert_eq!(
        select_encoder(CodecKind::Av1, true, Some(Av1Nvenc), 10, &a),
        Av1Vaapi
    );
}

#[test]
fn ten_bit_with_no_main10_encoder_ends_at_software_fallback() {
    let a = EncoderAvailability::default().with(Av1Nvenc, false);
    assert_eq!(
        select_encoder(CodecKind::Av1, true, None, 10, &a),
        LibsvtAv1
    );
}

#[test]
fn software_only_availability_gives_software_fallback() {
    let a = avail(&[LibsvtAv1, LibvpxVp9]);
    assert_eq!(select_encoder(CodecKind::Av1, true, None, 8, &a), LibsvtAv1);
    assert_eq!(select_encoder(CodecKind::Vp9, true, None, 8, &a), LibvpxVp9);
    let empty = EncoderAvailability::default();
    assert_eq!(
        select_encoder(CodecKind::Vp9, true, None, 8, &empty),
        LibvpxVp9
    );
}

#[test]
fn hardware_off_always_software_even_with_preference() {
    let all = avail(&[Av1Nvenc, Av1Qsv]);
    assert_eq!(
        select_encoder(CodecKind::Av1, false, Some(Av1Nvenc), 8, &all),
        LibsvtAv1
    );
}

#[test]
fn hardware_is_offered_regardless_of_host_os() {
    // NVENC and AMF are chosen on any host; selection never consults the OS.
    let a = avail(&[Av1Amf]);
    assert_eq!(select_encoder(CodecKind::Av1, true, None, 8, &a), Av1Amf);
}

#[test]
fn has_hardware_for_is_per_codec_and_ignores_software_encoders() {
    let a = avail(&[Av1Nvenc, LibvpxVp9]);
    assert!(a.has_hardware_for(CodecKind::Av1));
    assert!(!a.has_hardware_for(CodecKind::Vp9));
    assert!(a.has_any_hardware());
    assert!(!avail(&[LibsvtAv1, LibvpxVp9]).has_any_hardware());
    assert!(!EncoderAvailability::default().has_any_hardware());
}

#[test]
fn hevc_is_software_only_until_hardware_encoders_land() {
    assert!(CodecKind::ALL.contains(&CodecKind::Hevc));
    assert_eq!(CodecKind::Hevc.software_fallback(), Libx265);
    let a = avail(&[Libx265]);
    assert_eq!(select_encoder(CodecKind::Hevc, true, None, 10, &a), Libx265);
    assert_eq!(select_encoder(CodecKind::Hevc, false, None, 8, &a), Libx265);
}

#[test]
fn hevc_nvenc_is_the_first_hardware_choice() {
    assert_eq!(
        CodecKind::Hevc.hardware_priority().first(),
        Some(&HevcNvenc)
    );
    let a = avail(&[Libx265, HevcNvenc]);
    assert_eq!(
        select_encoder(CodecKind::Hevc, true, None, 8, &a),
        HevcNvenc
    );
    assert_eq!(select_encoder(CodecKind::Hevc, false, None, 8, &a), Libx265);
}

#[test]
fn hevc_vaapi_is_selected_when_available_and_falls_back_when_not() {
    let a = avail(&[Libx265, HevcVaapi]);
    assert_eq!(
        select_encoder(CodecKind::Hevc, true, None, 8, &a),
        HevcVaapi
    );
    assert_eq!(select_encoder(CodecKind::Hevc, false, None, 8, &a), Libx265);
    let none = avail(&[Libx265]);
    assert_eq!(
        select_encoder(CodecKind::Hevc, true, None, 8, &none),
        Libx265
    );
}
