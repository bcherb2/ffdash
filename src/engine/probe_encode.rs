//! Probe Encodes: a few-frame test encode whose success establishes Encoder
//! Availability (ADR 0001).
//!
//! Building the probe command and reading FFmpeg's encoder listing are pure and
//! unit-tested. Actually running FFmpeg is not: it is exercised on the real
//! machine, never against a fake FFmpeg.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::hardware::{self, VideoEncoder};
use super::selection::{CodecKind, EncoderAvailability};

/// Constant-quality value used by every probe (mid-range for all Encoders).
const PROBE_QUALITY: &str = "30";
/// A probe that has not finished by now is a hung driver: treat as unavailable.
const PROBE_TIMEOUT: Duration = Duration::from_secs(20);

/// The hardware Encoders worth probing: every hardware Encoder of every Codec.
///
/// A new Codec only needs its priority list in [`CodecKind::hardware_priority`]
/// and a [`probe_args`] arm; nothing else here changes.
pub fn hardware_candidates() -> Vec<VideoEncoder> {
    let mut out = Vec::new();
    for codec in CodecKind::ALL {
        for encoder in codec.hardware_priority() {
            if !out.contains(encoder) {
                out.push(*encoder);
            }
        }
    }
    out
}

/// Whether `ffmpeg -encoders` output lists an Encoder named `name`.
pub fn lists_encoder(listing: &str, name: &str) -> bool {
    listing
        .lines()
        .filter_map(|l| l.split_whitespace().nth(1))
        .any(|n| n == name)
}

/// FFmpeg arguments for a Probe Encode of `encoder` at `bit_depth` (8 or 10),
/// in constant-quality mode, to a null output. `None` for Software Encoders,
/// which need no probe. `render_device` is the DRM render node for VAAPI/QSV.
pub fn probe_args(
    encoder: VideoEncoder,
    bit_depth: u8,
    render_device: Option<&str>,
) -> Option<Vec<String>> {
    use VideoEncoder::*;
    let ten = bit_depth > 8;
    let fmt = if ten { "p010le" } else { "nv12" };
    let device = render_device.unwrap_or("/dev/dri/renderD128");
    let mut a: Vec<String> = Vec::new();
    let mut push = |items: &[&str]| a.extend(items.iter().map(|s| s.to_string()));

    push(&["-hide_banner", "-loglevel", "error", "-nostdin"]);
    match encoder {
        LibvpxVp9 | LibsvtAv1 | LibaomAv1 | Libx265 => return None,
        Vp9Qsv | Av1Qsv => {
            let init = match render_device {
                Some(d) => format!("qsv=qs:{d}"),
                None => "qsv=qs".to_string(),
            };
            push(&["-init_hw_device", &init, "-filter_hw_device", "qs"]);
        }
        Vp9Vaapi | Av1Vaapi => {
            let init = format!("vaapi=va:{device}");
            push(&["-init_hw_device", &init, "-filter_hw_device", "va"]);
        }
        Av1Nvenc | Av1Amf => {}
        // HEVC hardware Encoders: one arm each, filled in by that Encoder's ticket
        // (device init here, filter below, quality flags in the last match). Until
        // then they are not probed, which keeps them unavailable.
        // hevc_nvenc: no device init needed.
        HevcNvenc => {}
        // hevc_qsv: -init_hw_device qsv + -filter_hw_device.
        HevcQsv => {
            let init = match render_device {
                Some(d) => format!("qsv=qs:{d}"),
                None => "qsv=qs".to_string(),
            };
            push(&["-init_hw_device", &init, "-filter_hw_device", "qs"]);
        }
        // hevc_vaapi: -init_hw_device vaapi + -filter_hw_device.
        HevcVaapi => {
            let init = format!("vaapi=va:{device}");
            push(&["-init_hw_device", &init, "-filter_hw_device", "va"]);
        }
        // hevc_amf and hevc_videotoolbox: no device init needed.
        HevcAmf | HevcVideotoolbox => {}
    }
    push(&[
        "-f",
        "lavfi",
        "-i",
        "testsrc=size=640x360:rate=30",
        "-frames:v",
        "5",
    ]);
    match encoder {
        Vp9Qsv | Av1Qsv | HevcQsv => {
            push(&["-vf", &format!("format={fmt},hwupload=extra_hw_frames=64")])
        }
        Vp9Vaapi | Av1Vaapi | HevcVaapi => push(&["-vf", &format!("format={fmt},hwupload")]),
        _ => push(&["-pix_fmt", fmt]),
    }
    push(&["-c:v", encoder.ffmpeg_name()]);
    match encoder {
        // CQP: ICQ via -global_quality is broken on Intel Arc (see build_vp9_qsv_cmd).
        Vp9Qsv => push(&["-q:v", PROBE_QUALITY]),
        Av1Qsv => push(&["-rc_mode", "cqp", "-q:v", PROBE_QUALITY]),
        Vp9Vaapi => push(&["-rc_mode:v", "1", "-global_quality:v", PROBE_QUALITY]),
        Av1Vaapi => push(&["-rc_mode:v", "CQP", "-global_quality:v", PROBE_QUALITY]),
        Av1Nvenc => push(&["-rc", "vbr", "-cq", PROBE_QUALITY]),
        Av1Amf => push(&["-rc", "cqp", "-qp_i", PROBE_QUALITY, "-qp_p", PROBE_QUALITY]),
        // CQP through -q:v, the same flag the hevc_qsv command uses.
        HevcQsv => push(&["-q:v", PROBE_QUALITY]),
        HevcVaapi => push(&["-rc_mode:v", "CQP", "-global_quality:v", PROBE_QUALITY]),
        LibvpxVp9 | LibsvtAv1 | LibaomAv1 | Libx265 => unreachable!(),
        HevcNvenc => push(&["-rc", "vbr", "-cq", PROBE_QUALITY, "-b:v", "0"]),
        // AMF QP, constant quality.
        HevcAmf => push(&["-rc", "cqp", "-qp_i", PROBE_QUALITY, "-qp_p", PROBE_QUALITY]),
        // VideoToolbox constant quality (`q:v`, 1-100, higher is better). Intel Macs
        // reject it, which is how they fall back to libx265. 30 is a legal value.
        HevcVideotoolbox => push(&["-q:v", PROBE_QUALITY]),
    }
    push(&["-f", "null", "-"]);
    Some(a)
}

/// Run one Probe Encode on this machine. True if FFmpeg exits successfully in time.
fn run_probe(encoder: VideoEncoder, bit_depth: u8) -> bool {
    let render_device = hardware::detect_render_device();
    let Some(args) = probe_args(encoder, bit_depth, render_device.as_deref()) else {
        return true;
    };
    let mut cmd = Command::new("ffmpeg");
    cmd.args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if matches!(encoder.backend(), hardware::Backend::Vaapi)
        && let Some(cfg) = hardware::detect_vaapi_config()
    {
        cmd.env("LIBVA_DRIVERS_PATH", &cfg.driver.path);
        cmd.env("LIBVA_DRIVER_NAME", &cfg.driver.name);
    }
    let Ok(mut child) = cmd.spawn() else {
        return false;
    };
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if start.elapsed() < PROBE_TIMEOUT => {
                std::thread::sleep(Duration::from_millis(20))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
        }
    }
}

/// Build Encoder Availability by probing every candidate that FFmpeg lists.
/// Hardware candidates are probed in parallel: 8-bit first, then Main10 only
/// for those that passed. Software Encoders count as available when listed.
pub fn probe_availability(listing: &str) -> EncoderAvailability {
    let mut availability = EncoderAvailability::default();
    for codec in CodecKind::ALL {
        let soft = codec.software_fallback();
        if lists_encoder(listing, soft.ffmpeg_name()) {
            availability = availability.with(soft, true);
        }
    }
    let listed: Vec<VideoEncoder> = hardware_candidates()
        .into_iter()
        .filter(|e| lists_encoder(listing, e.ffmpeg_name()))
        .collect();
    let results: Vec<(VideoEncoder, bool, bool)> = std::thread::scope(|s| {
        let handles: Vec<_> = listed
            .iter()
            .map(|&e| {
                s.spawn(move || {
                    let ok = run_probe(e, 8);
                    (e, ok, ok && run_probe(e, 10))
                })
            })
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok()).collect()
    });
    for (e, ok, main10) in results {
        if ok {
            availability = availability.with(e, main10);
        }
    }
    availability
}

#[cfg(test)]
mod tests {
    use super::*;
    use VideoEncoder::*;

    fn joined(e: VideoEncoder, depth: u8) -> String {
        probe_args(e, depth, Some("/dev/dri/renderD129"))
            .unwrap()
            .join(" ")
    }

    #[test]
    fn every_hardware_priority_encoder_is_a_candidate_with_a_probe() {
        let c = hardware_candidates();
        for codec in CodecKind::ALL {
            for e in codec.hardware_priority() {
                assert!(c.contains(e));
                assert!(probe_args(*e, 8, None).is_some(), "{e:?}");
                assert!(probe_args(*e, 10, None).is_some(), "{e:?}");
            }
        }
        // Encoders are unique per Codec, so candidates are exactly the listed priorities.
        let listed: usize = CodecKind::ALL
            .iter()
            .map(|k| k.hardware_priority().len())
            .sum();
        assert_eq!(c.len(), listed);
        assert!(c.contains(&HevcQsv));
        assert!(c.contains(&HevcNvenc));
        assert!(c.contains(&HevcAmf) && c.contains(&HevcVideotoolbox));
        assert!(c.contains(&HevcVaapi));
    }

    #[test]
    fn software_encoders_need_no_probe() {
        assert!(probe_args(LibvpxVp9, 8, None).is_none());
        assert!(probe_args(LibsvtAv1, 10, None).is_none());
    }

    #[test]
    fn probes_are_few_frame_null_output_encodes() {
        for e in hardware_candidates() {
            let s = joined(e, 8);
            assert!(s.contains("-f lavfi -i testsrc="), "{e:?}");
            assert!(s.contains("-frames:v 5"), "{e:?}");
            assert!(s.ends_with("-f null -"), "{e:?}");
            assert!(s.contains(&format!("-c:v {}", e.ffmpeg_name())), "{e:?}");
        }
    }

    #[test]
    fn probes_use_constant_quality_mode() {
        assert!(joined(Av1Nvenc, 8).contains("-rc vbr -cq 30"));
        assert!(joined(HevcNvenc, 8).contains("-rc vbr -cq 30 -b:v 0"));
        assert!(joined(HevcNvenc, 10).contains("-pix_fmt p010le"));
        assert!(!joined(HevcNvenc, 8).contains("-init_hw_device"));
        assert!(joined(Av1Amf, 8).contains("-rc cqp -qp_i 30 -qp_p 30"));
        assert!(joined(Av1Qsv, 8).contains("-rc_mode cqp -q:v 30"));
        assert!(joined(Vp9Qsv, 8).contains("-q:v 30"));
        assert!(joined(HevcQsv, 8).contains("-c:v hevc_qsv -q:v 30"));
        assert!(joined(Av1Vaapi, 8).contains("-rc_mode:v CQP -global_quality:v 30"));
        assert!(joined(Vp9Vaapi, 8).contains("-rc_mode:v 1 -global_quality:v 30"));
    }

    #[test]
    fn hevc_amf_and_videotoolbox_probe_in_constant_quality() {
        let amf = joined(HevcAmf, 8);
        assert!(
            amf.contains("-c:v hevc_amf -rc cqp -qp_i 30 -qp_p 30"),
            "{amf}"
        );
        assert!(!amf.contains("-init_hw_device"));
        let vt = joined(HevcVideotoolbox, 10);
        assert!(vt.contains("-c:v hevc_videotoolbox -q:v 30"), "{vt}");
        assert!(vt.contains("-pix_fmt p010le"), "{vt}");
        assert!(!vt.contains("-init_hw_device"));
    }

    #[test]
    fn hevc_vaapi_probe_is_cqp_with_a_vaapi_device_and_main10_surfaces() {
        let s8 = joined(HevcVaapi, 8);
        assert!(s8.contains("-init_hw_device vaapi=va:/dev/dri/renderD129 -filter_hw_device va"));
        assert!(s8.contains("format=nv12,hwupload"));
        assert!(s8.contains("-c:v hevc_vaapi -rc_mode:v CQP -global_quality:v 30"));
        assert!(joined(HevcVaapi, 10).contains("format=p010le,hwupload"));
    }

    #[test]
    fn main10_probe_uses_ten_bit_surfaces() {
        assert!(joined(Av1Nvenc, 10).contains("-pix_fmt p010le"));
        assert!(joined(Av1Amf, 10).contains("-pix_fmt p010le"));
        assert!(joined(Av1Vaapi, 10).contains("format=p010le,hwupload"));
        assert!(joined(Av1Qsv, 10).contains("format=p010le,hwupload=extra_hw_frames=64"));
        assert!(joined(HevcQsv, 10).contains("format=p010le,hwupload=extra_hw_frames=64"));
        assert!(joined(Av1Nvenc, 8).contains("-pix_fmt nv12"));
        assert!(joined(Av1Vaapi, 8).contains("format=nv12,hwupload"));
    }

    #[test]
    fn device_init_follows_the_render_node() {
        assert!(joined(Av1Vaapi, 8).contains("-init_hw_device vaapi=va:/dev/dri/renderD129"));
        assert!(joined(Av1Qsv, 8).contains("-init_hw_device qsv=qs:/dev/dri/renderD129"));
        let no_node = probe_args(Av1Qsv, 8, None).unwrap().join(" ");
        assert!(no_node.contains("-init_hw_device qsv=qs "));
        assert!(joined(HevcQsv, 8).contains("-init_hw_device qsv=qs:/dev/dri/renderD129"));
        assert!(!joined(Av1Nvenc, 8).contains("-init_hw_device"));
    }

    #[test]
    fn listing_match_is_by_encoder_name_not_description() {
        let listing = " Encoders:\n V....D libsvtav1             SVT-AV1 (codec av1)\n V....D av1_nvenc           NVIDIA NVENC av1 encoder (codec av1)\n V....D libx264 mentions av1_qsv in text\n";
        assert!(lists_encoder(listing, "av1_nvenc"));
        assert!(lists_encoder(listing, "libsvtav1"));
        assert!(!lists_encoder(listing, "av1_qsv"));
        assert!(!lists_encoder(listing, "av1"));
    }
}
