//! Stream handling for MP4 output, shared by every Codec's command builder.
//!
//! MP4 cannot carry attachments, and only text subtitles (as `mov_text`), so the
//! profile's `additional_args` (written for MKV) are rewritten for MP4 outputs.

use std::path::Path;
use std::process::Command;

/// Subtitle codecs ffmpeg can convert to `mov_text`.
const TEXT_SUBTITLE_CODECS: &[&str] = &[
    "subrip",
    "srt",
    "ass",
    "ssa",
    "mov_text",
    "webvtt",
    "text",
    "subviewer",
    "subviewer1",
    "microdvd",
    "mpl2",
    "jacosub",
    "sami",
    "realtext",
    "stl",
    "vplayer",
    "pjs",
];

pub(super) fn is_text_subtitle(codec: &str) -> bool {
    TEXT_SUBTITLE_CODECS.contains(&codec)
}

/// Codec names of the input's subtitle streams, in stream order. Empty when the
/// input cannot be probed.
pub(super) fn probe_subtitle_codecs(input: &Path) -> Vec<String> {
    Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_streams",
            "-select_streams",
            "s",
        ])
        .arg(input)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| parse_subtitle_codecs(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or_default()
}

/// Subtitle codec names from ffprobe JSON; a stream without a codec name is kept as "".
pub(super) fn parse_subtitle_codecs(json: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    value["streams"]
        .as_array()
        .map(|streams| {
            streams
                .iter()
                .map(|s| s["codec_name"].as_str().unwrap_or("").to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// Rewrite parsed `additional_args` for an MP4 output: attachments are never mapped
/// (MP4 has no place for them), the profile's subtitle and attachment mappings and
/// codecs are removed, and each text subtitle stream (per `subtitle_codecs`, in
/// stream order) is mapped and converted to `mov_text`. Other subtitles are dropped.
pub(super) fn mp4_stream_args(args: Vec<String>, subtitle_codecs: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(args.len());
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-map" => {
                let target = iter.next().unwrap_or_default();
                if target.starts_with("0:s") || target.starts_with("0:t") {
                    continue;
                }
                let bare = target == "0";
                out.push(arg);
                out.push(target);
                if bare {
                    out.extend(["-map", "-0:t", "-map", "-0:s"].map(String::from));
                }
            }
            "-c:s" | "-c:t" | "-codec:s" | "-codec:t" | "-scodec" => {
                iter.next();
            }
            _ => out.push(arg),
        }
    }
    let mut mapped_any = false;
    for (index, codec) in subtitle_codecs.iter().enumerate() {
        if is_text_subtitle(codec) {
            out.extend(["-map".to_string(), format!("0:s:{index}?")]);
            mapped_any = true;
        }
    }
    if mapped_any {
        out.extend(["-c:s", "mov_text"].map(String::from));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Vec<String> {
        shlex::split(s).unwrap()
    }

    const MKV_ARGS: &str =
        "-map 0:s? -map 0:t? -c:s copy -c:t copy -map_metadata 0 -map_chapters 0";

    #[test]
    fn mp4_never_maps_attachments_and_drops_subtitles_without_text_streams() {
        assert_eq!(
            mp4_stream_args(args(MKV_ARGS), &[]),
            args("-map_metadata 0 -map_chapters 0")
        );
        let image_only = ["hdmv_pgs_subtitle".to_string(), "dvd_subtitle".to_string()];
        assert_eq!(
            mp4_stream_args(args(MKV_ARGS), &image_only),
            args("-map_metadata 0 -map_chapters 0")
        );
    }

    #[test]
    fn mp4_converts_text_subtitles_to_mov_text_and_skips_image_ones() {
        let streams = [
            "subrip".to_string(),
            "hdmv_pgs_subtitle".to_string(),
            "ass".to_string(),
        ];
        assert_eq!(
            mp4_stream_args(args(MKV_ARGS), &streams),
            args("-map_metadata 0 -map_chapters 0 -map 0:s:0? -map 0:s:2? -c:s mov_text")
        );
    }

    #[test]
    fn mp4_args_without_stream_mappings_still_get_text_subtitles() {
        let streams = ["subrip".to_string()];
        assert_eq!(
            mp4_stream_args(args("-map_metadata 0"), &streams),
            args("-map_metadata 0 -map 0:s:0? -c:s mov_text")
        );
    }

    #[test]
    fn mp4_bare_map_zero_excludes_attachments_and_subtitles() {
        assert_eq!(
            mp4_stream_args(args("-map 0 -c:s copy"), &[]),
            args("-map 0 -map -0:t -map -0:s")
        );
    }

    #[test]
    fn parses_subtitle_codec_names_from_ffprobe_json() {
        let json = r#"{"streams":[{"codec_name":"subrip"},{"codec_name":"hdmv_pgs_subtitle"}]}"#;
        assert_eq!(
            parse_subtitle_codecs(json),
            vec!["subrip", "hdmv_pgs_subtitle"]
        );
        assert!(parse_subtitle_codecs("not json").is_empty());
        assert!(parse_subtitle_codecs("{}").is_empty());
    }
}
