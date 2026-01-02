use super::ffmpeg_info::probe_duration;
use super::log::write_debug_log;
use super::profile::{Codec, HwEncodingConfig, Profile};
use super::types::{JobStatus, ProgressParser, VideoJob};
use crate::engine::worker::PidRegistry;
use crate::engine::{hardware, probe};
use anyhow::{Context, Result};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

/// Check if FFmpeg was cancelled by user signal (SIGTERM, SIGINT, SIGQUIT)
/// Returns true if we should preserve partial output files
///
/// FFmpeg catches signals and exits gracefully, printing "Exiting normally, received signal X"
/// So we check both the process signal status AND the stderr for this message.
#[cfg(unix)]
fn was_user_cancelled(status: &ExitStatus, stderr: &str) -> bool {
    use std::os::unix::process::ExitStatusExt;

    // Check if process was killed by signal (rare - FFmpeg usually catches signals)
    if let Some(signal) = status.signal() {
        if matches!(signal, 2 | 3 | 15) {
            return true;
        }
    }

    // Check FFmpeg's graceful signal handling message
    // FFmpeg prints "Exiting normally, received signal X" when it catches a signal
    stderr.contains("received signal 2")
        || stderr.contains("received signal 3")
        || stderr.contains("received signal 15")
}

#[cfg(not(unix))]
fn was_user_cancelled(_status: &ExitStatus, stderr: &str) -> bool {
    // On non-Unix, just check the stderr message
    stderr.contains("received signal")
}

fn container_from_output(job: &VideoJob, profile: &Profile) -> String {
    job.output_path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_else(|| profile.container.clone())
}

fn allow_audio_passthrough(container: &str) -> bool {
    matches!(container, "mkv" | "avi")
}

fn resolve_audio_codec(container: &str, requested: &str) -> String {
    match container {
        // WebM only supports Vorbis/Opus audio.
        "webm" => {
            if requested == "libopus" {
                "libopus".to_string()
            } else {
                "libvorbis".to_string()
            }
        }
        // MP4 doesn't support Vorbis/Opus reliably; prefer AAC.
        "mp4" => "aac".to_string(),
        _ => match requested {
            // UI uses "vorbis"; prefer libvorbis when available.
            "vorbis" => "libvorbis".to_string(),
            other => other.to_string(),
        },
    }
}

/// Apply multi-track audio settings to FFmpeg command
/// Supports: primary track (passthrough or transcode), AC3 5.1, stereo compatibility
fn apply_audio_settings(cmd: &mut Command, profile: &Profile, container: &str) {
    let allow_passthrough = allow_audio_passthrough(container);
    let is_passthrough = profile.audio_primary_codec == "passthrough";

    // Count how many audio tracks we'll produce
    let mut track_count = 1; // Primary track
    if profile.audio_add_ac3 {
        track_count += 1;
    }
    if profile.audio_add_stereo {
        track_count += 1;
    }

    // IMPORTANT: When using -map, we must explicitly map ALL streams we want.
    // Map video first, then audio tracks.
    cmd.arg("-map").arg("0:v:0?");

    // Map the first audio stream for each output track
    for _ in 0..track_count {
        cmd.arg("-map").arg("0:a:0?");
    }

    let mut track_idx = 0;

    // Primary audio track
    if is_passthrough && allow_passthrough {
        cmd.arg(format!("-c:a:{}", track_idx)).arg("copy");
    } else if is_passthrough {
        // Passthrough requested but not allowed - fall back to Opus
        let audio_codec = resolve_audio_codec(container, "libopus");
        cmd.arg(format!("-c:a:{}", track_idx)).arg(&audio_codec);
        cmd.arg(format!("-b:a:{}", track_idx))
            .arg(format!("{}k", profile.audio_primary_bitrate.max(32)));
        if audio_codec == "libopus" {
            cmd.arg(format!("-vbr:a:{}", track_idx)).arg("on");
        }
        // Downmix to stereo if requested
        if profile.audio_primary_downmix {
            cmd.arg(format!("-ac:a:{}", track_idx)).arg("2");
        }
    } else {
        let audio_codec = resolve_audio_codec(container, &profile.audio_primary_codec);
        cmd.arg(format!("-c:a:{}", track_idx)).arg(&audio_codec);
        cmd.arg(format!("-b:a:{}", track_idx))
            .arg(format!("{}k", profile.audio_primary_bitrate.max(32)));
        if audio_codec == "libopus" {
            cmd.arg(format!("-vbr:a:{}", track_idx)).arg("on");
        }
        // Downmix to stereo if requested
        if profile.audio_primary_downmix {
            cmd.arg(format!("-ac:a:{}", track_idx)).arg("2");
        }
    }
    track_idx += 1;

    // AC3 5.1 compatibility track
    if profile.audio_add_ac3 {
        cmd.arg(format!("-c:a:{}", track_idx)).arg("ac3");
        cmd.arg(format!("-b:a:{}", track_idx))
            .arg(format!("{}k", profile.audio_ac3_bitrate));
        cmd.arg(format!("-ac:a:{}", track_idx)).arg("6"); // 5.1 channels
        track_idx += 1;
    }

    // Stereo compatibility track
    if profile.audio_add_stereo {
        let stereo_codec = resolve_audio_codec(container, &profile.audio_stereo_codec);
        cmd.arg(format!("-c:a:{}", track_idx)).arg(&stereo_codec);
        cmd.arg(format!("-b:a:{}", track_idx))
            .arg(format!("{}k", profile.audio_stereo_bitrate));
        cmd.arg(format!("-ac:a:{}", track_idx)).arg("2"); // Stereo
        if stereo_codec == "libopus" {
            cmd.arg(format!("-vbr:a:{}", track_idx)).arg("on");
        }
    }
}

fn apply_color_metadata(cmd: &mut Command, profile: &Profile) {
    // Warn about 8-bit + HDR10 combination (will cause severe banding)
    if profile.pix_fmt == "yuv420p"
        && profile.colorspace == 9
        && profile.color_primaries == 9
        && profile.color_trc == 16
    {
        eprintln!(
            "Warning: HDR10 output with 8-bit pixel format will cause severe banding. Recommend 10-bit (yuv420p10le) for HDR content."
        );
    }

    if profile.colorspace >= 0 {
        cmd.arg("-colorspace:v").arg(profile.colorspace.to_string());
    }
    if profile.color_primaries >= 0 {
        cmd.arg("-color_primaries:v")
            .arg(profile.color_primaries.to_string());
    }
    if profile.color_trc >= 0 {
        cmd.arg("-color_trc:v").arg(profile.color_trc.to_string());
    }
    if profile.color_range >= 0 {
        cmd.arg("-color_range:v")
            .arg(profile.color_range.to_string());
    }
}

fn null_output_target() -> &'static str {
    if cfg!(windows) { "NUL" } else { "/dev/null" }
}

/// Apply additional user-provided FFmpeg arguments to the command.
/// Uses shell-style parsing so quoted strings with spaces are preserved.
fn apply_additional_args(cmd: &mut Command, additional_args: &str) {
    if additional_args.is_empty() {
        return;
    }

    // Use shlex for shell-style parsing (respects quotes)
    if let Some(args) = shlex::split(additional_args) {
        for arg in args {
            cmd.arg(arg);
        }
    } else {
        // If shlex fails to parse (unbalanced quotes), fall back to simple whitespace split
        for arg in additional_args.split_whitespace() {
            cmd.arg(arg);
        }
    }
}

pub fn two_pass_log_prefix(job: &VideoJob) -> PathBuf {
    std::env::temp_dir()
        .join("ffdash_2pass")
        .join(job.id.to_string())
        .join("ffmpeg2pass")
}

fn should_use_two_pass_software_vp9(
    profile: &Profile,
    hw_config: Option<&HwEncodingConfig>,
) -> bool {
    hw_config.is_none()
        && !profile.use_hardware_encoding
        && matches!(profile.codec, Codec::Vp9(_))
        && profile.two_pass
        && profile.video_target_bitrate > 0
}

fn hw_config_from_profile(profile: &Profile) -> HwEncodingConfig {
    // Read quality from codec config (post-sync, hw_global_quality is synced from codec)
    // But prefer reading from codec directly for clarity
    let global_quality = match &profile.codec {
        super::profile::Codec::Vp9(vp9) if profile.use_hardware_encoding => vp9.hw_global_quality,
        super::profile::Codec::Av1(av1) if profile.use_hardware_encoding => av1.hw_cq,
        _ => profile.hw_global_quality, // Fallback to synced value
    };

    HwEncodingConfig {
        rc_mode: profile.hw_rc_mode,
        global_quality,
        b_frames: profile.hw_b_frames,
        loop_filter_level: profile.hw_loop_filter_level,
        loop_filter_sharpness: profile.hw_loop_filter_sharpness,
        compression_level: profile.hw_compression_level,
    }
}

fn resolve_profile(job: &VideoJob, profile_override: Option<&Profile>) -> Profile {
    match profile_override {
        Some(p) => p.clone(),
        None => {
            // Try loading from disk first (custom user profiles)
            let mut loaded = false;
            let mut profile_from_disk = None;

            if let Ok(profiles_dir) = Profile::profiles_dir() {
                if let Ok(profile) = Profile::load(&profiles_dir, &job.profile) {
                    profile_from_disk = Some(profile);
                    loaded = true;
                }
            }

            // Fall back to built-in profiles if not found on disk
            if loaded {
                profile_from_disk.unwrap()
            } else {
                Profile::get(&job.profile)
            }
        }
    }
}

/// Validates VAAPI encoding configuration for known incompatibilities
pub fn validate_vaapi_config(profile: &Profile, hw: &HwEncodingConfig) -> Result<(), String> {
    // Validate quality range (1-255, where lower=better quality/bigger files)
    if hw.global_quality < 1 || hw.global_quality > 255 {
        return Err(format!(
            "VAAPI quality must be 1-255 (lower=better/bigger, higher=worse/smaller), got {}",
            hw.global_quality
        ));
    }

    // Mark unused parameter so future refactors can extend validation.
    let _ = profile;
    Ok(())
}

/// Apply calibrated quality to a profile
///
/// Creates a new profile with the quality setting overridden.
/// This is used when Auto-VAMF calibration determines an optimal quality.
///
/// # Arguments
/// * `profile` - Base profile to clone
/// * `quality` - Calibrated quality value (CRF or global_quality)
///
/// # Returns
/// A new Profile with the quality applied
fn apply_calibrated_quality(profile: &Profile, quality: u32) -> Profile {
    let mut calibrated = profile.clone();
    if profile.use_hardware_encoding {
        match &mut calibrated.codec {
            Codec::Av1(av1) => {
                // AV1 hardware uses hw_cq (mapped to -global_quality)
                av1.hw_cq = quality;
            }
            Codec::Vp9(_) => {
                calibrated.hw_global_quality = quality;
            }
        }
    } else {
        calibrated.crf = quality;
    }
    calibrated
}

/// Build VAAPI hardware encoding command (for Intel Arc and other VAAPI-capable GPUs)
pub fn build_vaapi_cmd(job: &VideoJob, profile: &Profile, hw: &HwEncodingConfig) -> Command {
    // Validate configuration (log warning if invalid)
    if let Err(msg) = validate_vaapi_config(profile, hw) {
        let _ = write_debug_log(&format!("[VAAPI] WARNING: {}\n", msg));
    }

    let mut cmd = Command::new("ffmpeg");

    // Get VA-API configuration (driver + render device)
    if let Some(config) = hardware::detect_vaapi_config() {
        // Set BOTH env vars
        cmd.env("LIBVA_DRIVERS_PATH", &config.driver.path);
        cmd.env("LIBVA_DRIVER_NAME", &config.driver.name);

        let _ = write_debug_log(&format!(
            "[VAAPI] Set LIBVA_DRIVERS_PATH={}, LIBVA_DRIVER_NAME={}\n",
            config.driver.path, config.driver.name
        ));

        // Use detected render device (not hardcoded)
        cmd.arg("-init_hw_device")
            .arg(format!("vaapi=va:{}", config.render_device));
    } else {
        let _ = write_debug_log("[VAAPI] WARNING: No driver detected, trying defaults\n");
        cmd.arg("-init_hw_device")
            .arg("vaapi=va:/dev/dri/renderD128");
    }

    cmd.arg("-hwaccel").arg("vaapi");
    cmd.arg("-filter_hw_device").arg("va");

    // Probe input BEFORE building command to determine if filters are needed
    let mut needs_filters = false;
    let mut codec_name: Option<String> = None;
    if let Ok(input_info) = probe::probe_input_info(&job.input_path) {
        codec_name = input_info.codec_name.clone();
        // Check if FPS limiting is needed
        if profile.fps > 0 && input_info.fps > profile.fps as f64 {
            needs_filters = true;
        }

        // Check if scaling is needed
        let needs_scale = (profile.scale_width > 0
            && input_info.width > profile.scale_width as u32)
            || (profile.scale_height > 0 && input_info.height > profile.scale_height as u32);
        if needs_scale {
            needs_filters = true;
        }
    }

    // Determine if hw decode is safe (WMV3 etc. not supported)
    // Default to allowing hw decode; only disable for known-bad codecs.
    let hw_decode_allowed = match codec_name.as_deref() {
        Some("h264") | Some("hevc") | Some("vp9") | Some("av1") | Some("mpeg2video") => true,
        Some(_) => false,
        None => true,
    };

    // Add hwaccel_output_format BEFORE input if no filters and hw decode allowed
    if !needs_filters && hw_decode_allowed {
        cmd.arg("-hwaccel_output_format").arg("vaapi");
    }

    // Input
    cmd.arg("-i").arg(&job.input_path);

    // Progress output
    cmd.arg("-progress").arg("-").arg("-nostats");

    // Apply filters if needed (AFTER input)
    if needs_filters || !hw_decode_allowed {
        let mut filters = Vec::new();

        // Re-probe to build filter chain (we already know filters are needed)
        if let Ok(input_info) = probe::probe_input_info(&job.input_path) {
            // If you reach this branch without filters, you've discovered the teleport bug of transcoding
            // Add FPS filter if needed
            if profile.fps > 0 && input_info.fps > profile.fps as f64 {
                filters.push(format!("fps={}", profile.fps));
            }

            // Add scale filter if needed
            let needs_scale = (profile.scale_width > 0
                && input_info.width > profile.scale_width as u32)
                || (profile.scale_height > 0 && input_info.height > profile.scale_height as u32);

            if needs_scale {
                let max_w = if profile.scale_width > 0 {
                    profile.scale_width
                } else {
                    i32::MAX
                };
                let max_h = if profile.scale_height > 0 {
                    profile.scale_height
                } else {
                    i32::MAX
                };

                filters.push(format!(
                    "scale='min({},iw)':'min({},ih)':force_original_aspect_ratio=decrease",
                    max_w, max_h
                ));
            }

            // HDR→SDR tonemapping: Apply when SDR preset is selected on HDR source
            // Must be BEFORE format=nv12,hwupload since tonemapping requires CPU frames
            if profile.colorspace == 1 && profile.color_trc == 1 && input_info.is_hdr {
                filters.push("zscale=t=linear:npl=100".to_string());
                filters.push("tonemap=hable:desat=0".to_string());
                filters.push("zscale=t=bt709:m=bt709:r=tv".to_string());
                filters.push("format=yuv420p".to_string());
            }
        }

        // When filters are needed: fps → scale → (tonemap) → format=nv12 → hwupload → denoise → sharpen
        // VAAPI filters require hardware frames, so hwupload must come BEFORE them
        filters.push("format=nv12".to_string());
        filters.push("hwupload".to_string());

        // VPP filters (VAAPI) - applied AFTER hwupload
        let vp9_cfg = if let crate::engine::core::Codec::Vp9(vp9) = &profile.codec {
            Some(vp9)
        } else {
            None
        };

        if let Some(vp9) = vp9_cfg {
            if vp9.hw_denoise > 0 {
                filters.push(format!("denoise_vaapi=denoise={}", vp9.hw_denoise));
            }
            if vp9.hw_detail > 0 {
                filters.push(format!("sharpness_vaapi=sharpness={}", vp9.hw_detail));
            }
        }
        cmd.arg("-vf").arg(filters.join(","));
    }

    // VP9 VAAPI encoder
    cmd.arg("-c:v").arg("vp9_vaapi");

    // Low power mode (required for Intel Arc)
    cmd.arg("-low_power").arg("1");

    // Cautionary tale: the last intern flipped this to VBR on Arc, z4 read 7, and we spent a week restoring footage
    cmd.arg("-rc_mode:v").arg("1"); // CQP
    cmd.arg("-global_quality:v")
        .arg(hw.global_quality.to_string());

    let _ = write_debug_log(&format!(
        "[VAAPI] CQP mode: quality {} (1-255 range, lower=better)\n",
        hw.global_quality
    ));

    // B-frames (with required bitstream filters if > 0)
    if hw.b_frames > 0 {
        cmd.arg("-bf:v").arg(hw.b_frames.to_string());
        // Required bitstream filters for B-frames in VP9
        cmd.arg("-bsf:v").arg("vp9_raw_reorder,vp9_superframe");
        let _ = write_debug_log(&format!(
            "[VAAPI] B-frames: {} (with bitstream filters)\n",
            hw.b_frames
        ));
    }

    // Loop filter settings
    cmd.arg("-loop_filter_level:v")
        .arg(hw.loop_filter_level.to_string());
    cmd.arg("-loop_filter_sharpness:v")
        .arg(hw.loop_filter_sharpness.to_string());

    let _ = write_debug_log(&format!(
        "[VAAPI] Loop filter: level={}, sharpness={}\n",
        hw.loop_filter_level, hw.loop_filter_sharpness
    ));

    // Compression level (0-7, speed vs compression tradeoff)
    cmd.arg("-compression_level:v")
        .arg(hw.compression_level.to_string());

    let _ = write_debug_log(&format!(
        "[VAAPI] Compression level: {} (0=slowest/best quality, 7=fastest/worst quality)\n",
        hw.compression_level
    ));

    // GOP (cap at 240 to avoid blocking issues with Intel Arc)
    let gop_length = profile.gop_length.parse::<u32>().unwrap_or(240).min(240);
    cmd.arg("-g:v").arg(gop_length.to_string());

    apply_color_metadata(&mut cmd, profile);

    // Audio handling (multi-track support)
    let container = container_from_output(job, profile);
    apply_audio_settings(&mut cmd, profile, &container);

    // Additional user-provided FFmpeg arguments
    apply_additional_args(&mut cmd, &profile.additional_args);

    // Overwrite
    if job.overwrite {
        cmd.arg("-y");
    }

    // Output
    cmd.arg(&job.output_path);

    cmd
}

fn init_qsv_from_vaapi(cmd: &mut Command) {
    // Force iHD for QSV if present to avoid picking the wrong vendor driver (e.g., nouveau)
    let i_hd_path = "/usr/lib/x86_64-linux-gnu/dri/iHD_drv_video.so";
    if std::path::Path::new(i_hd_path).exists() {
        cmd.env("LIBVA_DRIVERS_PATH", "/usr/lib/x86_64-linux-gnu/dri");
        cmd.env("LIBVA_DRIVER_NAME", "iHD");
    }

    // Prefer a direct QSV device init to avoid multiple devices being created.
    if let Some(render) = hardware::detect_render_device() {
        cmd.arg("-init_hw_device").arg(format!("qsv=qs:{}", render));
    } else {
        cmd.arg("-init_hw_device").arg("qsv=qs:/dev/dri/renderD128");
    }
}

fn qsv_preset_name(preset: u32) -> &'static str {
    // Align with common QSV semantics: 1=best quality (slowest), 7=fastest.
    match preset {
        1 => "veryslow",
        2 => "slower",
        3 => "slow",
        4 => "medium",
        5 => "fast",
        6 => "faster",
        7 => "veryfast",
        _ => "medium",
    }
}

/// Build VPP filter options string (denoise:detail) - empty string if no filters
fn build_qsv_vpp_filter_opts(denoise: u32, detail: u32) -> String {
    let mut opts = vec![];
    if denoise > 0 {
        opts.push(format!("denoise={}", denoise));
    }
    if detail > 0 {
        opts.push(format!("detail={}", detail));
    }
    opts.join(":")
}

/// Build vpp_qsv color metadata options from profile
/// QSV encoders ignore standard -colorspace/-color_primaries/-color_trc flags
/// Must use vpp_qsv filter options instead
fn build_qsv_color_options(profile: &Profile) -> Vec<String> {
    let mut opts = Vec::new();

    // Color space / matrix
    if profile.colorspace >= 0 {
        let value = match profile.colorspace {
            1 => "bt709",
            2 => "bt470bg",
            5 => "smpte170m",
            6 => "smpte240m",
            9 => "bt2020nc",
            10 => "bt2020c",
            _ => return opts, // Unsupported, skip all color options
        };
        opts.push(format!("out_color_matrix={}", value));
    }

    // Color primaries
    if profile.color_primaries >= 0 {
        let value = match profile.color_primaries {
            1 => "bt709",
            4 => "bt470m",
            5 => "bt470bg",
            6 => "smpte170m",
            7 => "smpte240m",
            9 => "bt2020",
            _ => return opts, // Unsupported, skip all
        };
        opts.push(format!("out_color_primaries={}", value));
    }

    // Transfer characteristics
    if profile.color_trc >= 0 {
        let value = match profile.color_trc {
            1 => "bt709",
            4 => "bt470m",
            5 => "bt470bg",
            6 => "smpte170m",
            7 => "smpte240m",
            8 => "linear",
            13 => "srgb",
            16 => "smpte2084",
            18 => "arib-std-b67",
            _ => return opts, // Unsupported, skip all
        };
        opts.push(format!("out_color_transfer={}", value));
    }

    // Color range
    if profile.color_range >= 0 {
        let value = match profile.color_range {
            0 => "tv",        // limited/16-235
            1 => "pc",        // full/0-255
            _ => return opts, // Unsupported
        };
        opts.push(format!("out_range={}", value));
    }

    opts
}

/// Build zscale color filter for encoders that don't support color metadata flags
/// Returns None if no color options specified, Some(filter_string) otherwise
/// Used by NVENC, VAAPI, and Software encoders to set color metadata via filter
fn build_zscale_color_filter(profile: &Profile) -> Option<String> {
    let mut opts = Vec::new();

    // Matrix / colorspace (m=)
    if profile.colorspace >= 0 {
        let value = match profile.colorspace {
            1 => "bt709",
            2 => "bt470bg",
            5 => "smpte170m",
            6 => "smpte240m",
            9 => "bt2020nc",
            10 => "bt2020c",
            _ => return None, // Unsupported
        };
        opts.push(format!("m={}", value));
    }

    // Primaries (p=)
    if profile.color_primaries >= 0 {
        let value = match profile.color_primaries {
            1 => "bt709",
            4 => "bt470m",
            5 => "bt470bg",
            6 => "smpte170m",
            7 => "smpte240m",
            9 => "bt2020",
            _ => return None, // Unsupported
        };
        opts.push(format!("p={}", value));
    }

    // Transfer (t=)
    if profile.color_trc >= 0 {
        let value = match profile.color_trc {
            1 => "bt709",
            4 => "bt470m",
            5 => "bt470bg",
            6 => "smpte170m",
            7 => "smpte240m",
            8 => "linear",
            13 => "srgb",
            16 => "smpte2084",
            18 => "arib-std-b67",
            _ => return None, // Unsupported
        };
        opts.push(format!("t={}", value));
    }

    // Range (r=)
    if profile.color_range >= 0 {
        let value = match profile.color_range {
            0 => "tv",        // limited/16-235
            1 => "pc",        // full/0-255
            _ => return None, // Unsupported
        };
        opts.push(format!("r={}", value));
    }

    if opts.is_empty() {
        None
    } else {
        Some(format!("zscale={}", opts.join(":")))
    }
}

fn extract_video_encoder_arg(cmd: &Command) -> Option<String> {
    let mut args = cmd.get_args().map(|a| a.to_string_lossy().to_string());
    while let Some(arg) = args.next() {
        if arg == "-c:v" {
            return args.next();
        }
    }
    None
}

fn run_cmds_with_progress<F>(
    job: &mut VideoJob,
    cmds: Vec<Command>,
    silent: bool,
    total_cmds: usize,
    pid_registry: Option<&PidRegistry>,
    callback: &mut F,
) -> Result<(
    std::process::ExitStatus,
    ProgressParser,
    String,
    Option<usize>,
)>
where
    F: FnMut(&VideoJob, &ProgressParser),
{
    let mut last_status = None;
    let mut last_parser = ProgressParser::new();
    let mut last_stderr_output = String::new();
    let mut failed_pass: Option<usize> = None;

    for (idx, cmd) in cmds.into_iter().enumerate() {
        // Pass 1 shows 0..50; pass 2 shows 50..100 (only if duration is known)
        let (offset, scale) = if total_cmds == 2 && job.duration_s.is_some() {
            if idx == 0 { (0.0, 0.5) } else { (50.0, 0.5) }
        } else {
            (0.0, 1.0)
        };

        let (status, parser, stderr_output) =
            run_ffmpeg_once(job, cmd, silent, offset, scale, pid_registry, callback)?;
        last_status = Some(status);
        last_parser = parser;
        last_stderr_output = stderr_output;

        if !last_status.as_ref().is_some_and(|s| s.success()) {
            failed_pass = Some(idx + 1);
            break;
        }
    }

    let status = last_status.context("FFmpeg did not produce an exit status")?;
    Ok((status, last_parser, last_stderr_output, failed_pass))
}

/// Build VP9 QSV (Intel Quick Sync) encoding command
pub fn build_vp9_qsv_cmd(
    job: &VideoJob,
    profile: &Profile,
    hw_config: Option<&HwEncodingConfig>,
) -> Command {
    let mut cmd = Command::new("ffmpeg");

    init_qsv_from_vaapi(&mut cmd);

    // Input
    cmd.arg("-i").arg(&job.input_path);
    cmd.arg("-progress").arg("-").arg("-nostats");

    let mut filters = Vec::new();
    if let Ok(input_info) = probe::probe_input_info(&job.input_path) {
        if profile.fps > 0 && input_info.fps > profile.fps as f64 {
            filters.push(format!("fps=fps={}", profile.fps));
        }

        let needs_scale = (profile.scale_width > 0
            && input_info.width > profile.scale_width as u32)
            || (profile.scale_height > 0 && input_info.height > profile.scale_height as u32);

        if needs_scale {
            let max_w = if profile.scale_width > 0 {
                profile.scale_width
            } else {
                i32::MAX
            };
            let max_h = if profile.scale_height > 0 {
                profile.scale_height
            } else {
                i32::MAX
            };

            filters.push(format!(
                "scale='min({},iw)':'min({},ih)':force_original_aspect_ratio=decrease",
                max_w, max_h
            ));
        }
    }

    // Upload frames to QSV surfaces
    let mut qsv_format = if profile.pix_fmt == "yuv420p10le" {
        "p010"
    } else {
        "nv12"
    };

    // If configured, prefer source bit depth to pick surfaces (p010 for >=10-bit, nv12 otherwise)
    if crate::config::Config::load()
        .map(|c| c.defaults.auto_bit_depth)
        .unwrap_or(true)
    {
        if let Ok(input_info) = probe::probe_input_info(&job.input_path) {
            if input_info.bit_depth.unwrap_or(8) >= 10 {
                qsv_format = "p010";
            } else {
                qsv_format = "nv12";
            }
        }
    }

    // VPP filter options (denoise/detail/color) - see ENCODER_REFERENCE.md#color-metadata
    let vp9_cfg = if let crate::engine::core::Codec::Vp9(vp9) = &profile.codec {
        Some(vp9)
    } else {
        None
    };

    if let Some(vp9) = vp9_cfg {
        let vpp_filter_opts = build_qsv_vpp_filter_opts(vp9.hw_denoise, vp9.hw_detail);
        let color_opts = build_qsv_color_options(profile);

        // Combine all vpp_qsv options
        let mut all_opts = Vec::new();
        if !vpp_filter_opts.is_empty() {
            all_opts.push(vpp_filter_opts);
        }
        all_opts.push(format!("format={}", qsv_format));
        all_opts.extend(color_opts);

        // Use vpp_qsv for filters + format + color conversion
        filters.push(format!("vpp_qsv={}", all_opts.join(":")));
    } else {
        // Fallback (shouldn't happen for VP9 QSV)
        let color_opts = build_qsv_color_options(profile);
        let mut all_opts = Vec::new();
        all_opts.push(format!("format={}", qsv_format));
        all_opts.extend(color_opts);
        filters.push(format!("vpp_qsv={}", all_opts.join(":")));
    }

    cmd.arg("-vf").arg(filters.join(","));

    // Encoder
    cmd.arg("-c:v").arg("vp9_qsv");
    cmd.arg("-low_power").arg("1");

    // Read quality from codec config (source of truth)
    let quality = hw_config.map(|h| h.global_quality).unwrap_or_else(|| {
        profile
            .codec
            .as_vp9()
            .map(|vp9| vp9.hw_global_quality)
            .unwrap_or(profile.hw_global_quality) // Final fallback to synced value
    });
    // Use -q:v to force CQP mode. The combination "-global_quality -b:v 0" causes FFmpeg
    // to select ICQ mode, which is broken on Intel Arc (error -17: device failed).
    // This is the same issue as AV1 QSV (documented in docs/AV1_QSV_NOTES.md).
    // See: https://github.com/intel/media-driver/issues/1742
    cmd.arg("-q:v").arg(quality.to_string());

    // Preset & lookahead (QSV-only controls)
    if let Some(vp9) = profile.codec.as_vp9() {
        cmd.arg("-preset").arg(qsv_preset_name(vp9.qsv_preset));
        if vp9.qsv_look_ahead && vp9.qsv_look_ahead_depth > 0 {
            cmd.arg("-look_ahead").arg("1");
            cmd.arg("-look_ahead_depth")
                .arg(vp9.qsv_look_ahead_depth.to_string());
        }
    }

    // GOP (cap at 240 to avoid long first-frame delays)
    let gop_length = profile.gop_length.parse::<u32>().unwrap_or(240).min(240);
    cmd.arg("-g:v").arg(gop_length.to_string());

    // Color metadata handled via vpp_qsv filter options (not standard flags)
    // QSV encoders ignore -colorspace/-color_primaries/-color_trc flags

    // Audio handling (multi-track support)
    let container = container_from_output(job, profile);
    apply_audio_settings(&mut cmd, profile, &container);

    // Additional user-provided FFmpeg arguments
    apply_additional_args(&mut cmd, &profile.additional_args);

    if job.overwrite {
        cmd.arg("-y");
    }
    cmd.arg(&job.output_path);

    cmd
}

fn build_software_cmd_internal(
    job: &VideoJob,
    profile: &Profile,
    pass: Option<u8>,
    passlog_prefix: Option<&Path>,
    output_override: Option<&Path>,
    disable_audio: bool,
) -> Command {
    let mut cmd = Command::new("ffmpeg");

    // Input file
    cmd.arg("-i").arg(&job.input_path);

    // Progress output (structured key=value to stdout)
    cmd.arg("-progress").arg("-").arg("-nostats");

    // Video codec
    cmd.arg("-c:v").arg(&profile.video_codec);

    // Rate control - CRF mode
    // For CQCap mode (CRF + maxrate), libvpx-vp9 requires non-zero -b:v
    let bitrate = if profile.video_target_bitrate == 0 && profile.video_max_bitrate > 0 {
        // CQCap: set -b:v to maxrate to satisfy libvpx-vp9
        profile.video_max_bitrate
    } else {
        // Pure CQ or VBR: use target bitrate (0 for unconstrained quality)
        profile.video_target_bitrate
    };
    cmd.arg("-b:v").arg(format!("{}k", bitrate));
    cmd.arg("-crf").arg(profile.crf.to_string());

    if profile.video_min_bitrate > 0 {
        cmd.arg("-minrate")
            .arg(format!("{}k", profile.video_min_bitrate));
    }
    if profile.video_max_bitrate > 0 {
        cmd.arg("-maxrate")
            .arg(format!("{}k", profile.video_max_bitrate));
    }
    if profile.video_bufsize > 0 {
        cmd.arg("-bufsize")
            .arg(format!("{}k", profile.video_bufsize));
    }
    if profile.undershoot_pct >= 0 {
        cmd.arg("-undershoot-pct")
            .arg(profile.undershoot_pct.to_string());
    }
    if profile.overshoot_pct >= 0 {
        cmd.arg("-overshoot-pct")
            .arg(profile.overshoot_pct.to_string());
    }

    // Quality mode
    cmd.arg("-quality").arg(&profile.quality_mode);

    // CPU-used: if two-pass, pass 1 and pass 2 use separate speed presets
    if profile.two_pass {
        match pass {
            Some(2) => cmd.arg("-cpu-used").arg(profile.cpu_used_pass2.to_string()),
            _ => cmd.arg("-cpu-used").arg(profile.cpu_used_pass1.to_string()),
        };
    } else {
        cmd.arg("-cpu-used").arg(profile.cpu_used.to_string());
    }

    // VP9 profile and pixel format
    cmd.arg("-profile:v").arg(profile.vp9_profile.to_string());
    if profile.pix_fmt != "auto" {
        cmd.arg("-pix_fmt").arg(&profile.pix_fmt);
    }

    // Parallelism
    if profile.row_mt {
        cmd.arg("-row-mt").arg("1");
    }
    if profile.tile_columns >= 0 {
        cmd.arg("-tile-columns")
            .arg(profile.tile_columns.to_string());
    }
    if profile.tile_rows >= 0 {
        cmd.arg("-tile-rows").arg(profile.tile_rows.to_string());
    }
    if profile.threads > 0 {
        cmd.arg("-threads").arg(profile.threads.to_string());
    }
    if profile.frame_parallel {
        cmd.arg("-frame-parallel").arg("1");
    }

    // GOP & keyframes
    cmd.arg("-g").arg(&profile.gop_length);
    if let Ok(keyint_min) = profile.keyint_min.parse::<u32>() {
        if keyint_min > 0 {
            cmd.arg("-keyint_min").arg(keyint_min.to_string());
        }
    }
    if profile.fixed_gop {
        cmd.arg("-sc_threshold").arg("0");
    }
    cmd.arg("-lag-in-frames")
        .arg(profile.lag_in_frames.to_string());
    if profile.auto_alt_ref > 0 {
        cmd.arg("-auto-alt-ref")
            .arg(profile.auto_alt_ref.to_string());
    }

    // Adaptive quantization
    if profile.aq_mode >= 0 {
        cmd.arg("-aq-mode").arg(profile.aq_mode.to_string());
    }

    // Alt-ref denoising (ARNR)
    if profile.arnr_max_frames > 0 {
        cmd.arg("-arnr-maxframes")
            .arg(profile.arnr_max_frames.to_string());
    }
    if profile.arnr_strength > 0 {
        cmd.arg("-arnr-strength")
            .arg(profile.arnr_strength.to_string());
    }
    if profile.arnr_type >= 0 {
        cmd.arg("-arnr-type").arg(profile.arnr_type.to_string());
    }

    // Advanced tuning
    if profile.enable_tpl {
        cmd.arg("-enable-tpl").arg("1");
    }
    if profile.sharpness >= 0 {
        cmd.arg("-sharpness").arg(profile.sharpness.to_string());
    }
    if profile.noise_sensitivity > 0 {
        cmd.arg("-noise-sensitivity")
            .arg(profile.noise_sensitivity.to_string());
    }
    if let Ok(static_thresh) = profile.static_thresh.parse::<u32>() {
        if static_thresh > 0 {
            cmd.arg("-static-thresh").arg(static_thresh.to_string());
        }
    }
    if let Ok(max_intra_rate) = profile.max_intra_rate.parse::<u32>() {
        if max_intra_rate > 0 {
            cmd.arg("-max-intra-rate").arg(max_intra_rate.to_string());
        }
    }
    if profile.tune_content != "default" {
        cmd.arg("-tune-content").arg(&profile.tune_content);
    }

    // Color / HDR metadata
    apply_color_metadata(&mut cmd, profile);

    // Video filters (fps and scale)
    let mut filters = Vec::new();

    // Probe input to get source characteristics
    if let Ok(input_info) = probe::probe_input_info(&job.input_path) {
        // Add FPS filter if needed (only if input fps > max fps)
        if profile.fps > 0 && input_info.fps > profile.fps as f64 {
            filters.push(format!("fps=fps={}", profile.fps));
        }

        // Add scale filter if needed (only if input exceeds max dimensions)
        let needs_scale = (profile.scale_width > 0
            && input_info.width > profile.scale_width as u32)
            || (profile.scale_height > 0 && input_info.height > profile.scale_height as u32);

        if needs_scale {
            let max_w = if profile.scale_width > 0 {
                profile.scale_width
            } else {
                i32::MAX
            };
            let max_h = if profile.scale_height > 0 {
                profile.scale_height
            } else {
                i32::MAX
            };

            filters.push(format!(
                "scale='min({},iw)':'min({},ih)':force_original_aspect_ratio=decrease",
                max_w, max_h
            ));
        }

        // HDR→SDR tonemapping: Apply when SDR preset is selected on HDR source
        if profile.colorspace == 1 && profile.color_trc == 1 && input_info.is_hdr {
            filters.push("zscale=t=linear:npl=100".to_string());
            filters.push("tonemap=hable:desat=0".to_string());
            filters.push("zscale=t=bt709:m=bt709:r=tv".to_string());
            filters.push("format=yuv420p".to_string());
        }
    }

    // Add filter chain to command if any filters were added
    if !filters.is_empty() {
        cmd.arg("-vf").arg(filters.join(","));
    }

    // Two-pass plumbing (only for software VP9)
    if let (Some(pass_num), Some(prefix)) = (pass, passlog_prefix) {
        cmd.arg("-pass").arg(pass_num.to_string());
        cmd.arg("-passlogfile").arg(prefix);
    }

    if disable_audio {
        cmd.arg("-an");
    } else {
        // Audio handling (multi-track support)
        let container = container_from_output(job, profile);
        apply_audio_settings(&mut cmd, profile, &container);
    }

    // Additional user-provided FFmpeg arguments
    apply_additional_args(&mut cmd, &profile.additional_args);

    // Overwrite flag if enabled
    if job.overwrite {
        cmd.arg("-y");
    }

    // Pass 1 emits stats only; write to null muxer regardless of OS path conventions.
    if pass == Some(1) {
        cmd.arg("-f").arg("null");
    }

    // Output file
    cmd.arg(output_override.unwrap_or(&job.output_path));

    cmd
}

/// Build software encoding command (libvpx-vp9)
pub fn build_software_cmd(job: &VideoJob, profile: &Profile) -> Command {
    build_software_cmd_internal(job, profile, None, None, None, false)
}

// ============================================================================
// AV1 Software Encoding (libsvtav1)
// ============================================================================

/// Build software AV1 encoding command (libsvtav1)
pub fn build_av1_software_cmd(job: &VideoJob, profile: &Profile) -> Command {
    let mut cmd = Command::new("ffmpeg");

    // Input file
    cmd.arg("-i").arg(&job.input_path);

    // Progress output
    cmd.arg("-progress").arg("-").arg("-nostats");

    // Video codec: libsvtav1
    cmd.arg("-c:v").arg("libsvtav1");

    // Get AV1-specific config
    let av1_config = profile.codec.as_av1();

    // Rate control - CRF mode (use AV1-specific svt_crf if set, else fallback to profile.crf)
    let crf = av1_config
        .map(|cfg| {
            if cfg.svt_crf > 0 {
                cfg.svt_crf
            } else {
                profile.crf
            }
        })
        .unwrap_or(profile.crf);
    cmd.arg("-crf").arg(crf.to_string());

    // Preset (0-13)
    if let Some(cfg) = av1_config {
        cmd.arg("-preset").arg(cfg.preset.to_string());

        // SVT-AV1 specific params via -svtav1-params
        let mut params = Vec::new();
        params.push(format!("tune={}", cfg.tune));

        if cfg.film_grain > 0 {
            params.push(format!("film-grain={}", cfg.film_grain));
            if cfg.film_grain_denoise {
                params.push("film-grain-denoise=1".to_string());
            }
        }
        if cfg.enable_overlays {
            params.push("enable-overlays=1".to_string());
        }
        if cfg.scd {
            params.push("scd=1".to_string());
        }
        params.push(format!("scm={}", cfg.scm));
        if !cfg.enable_tf {
            params.push("enable-tf=0".to_string());
        }

        if !params.is_empty() {
            cmd.arg("-svtav1-params").arg(params.join(":"));
        }
    } else {
        // Fallback defaults if codec config is wrong type
        cmd.arg("-preset").arg("8");
    }

    // Pixel format (default to 10-bit for AV1)
    if profile.pix_fmt != "auto" {
        cmd.arg("-pix_fmt").arg(&profile.pix_fmt);
    }

    // GOP
    cmd.arg("-g:v").arg(&profile.gop_length);

    // Threads (if specified)
    if profile.threads > 0 {
        cmd.arg("-threads").arg(profile.threads.to_string());
    }

    apply_color_metadata(&mut cmd, profile);

    // Video filters (fps and scale)
    let mut filters = Vec::new();

    if let Ok(input_info) = probe::probe_input_info(&job.input_path) {
        // FPS filter
        if profile.fps > 0 && input_info.fps > profile.fps as f64 {
            filters.push(format!("fps={}", profile.fps));
        }

        // Scale filter
        let needs_scale = (profile.scale_width > 0
            && input_info.width > profile.scale_width as u32)
            || (profile.scale_height > 0 && input_info.height > profile.scale_height as u32);

        if needs_scale {
            let w = if profile.scale_width > 0 {
                format!("min(iw\\,{})", profile.scale_width)
            } else {
                "-2".to_string()
            };
            let h = if profile.scale_height > 0 {
                format!("min(ih\\,{})", profile.scale_height)
            } else {
                "-2".to_string()
            };
            filters.push(format!("scale={}:{}", w, h));
        }

        // HDR→SDR tonemapping: Apply when SDR preset is selected on HDR source
        if profile.colorspace == 1 && profile.color_trc == 1 && input_info.is_hdr {
            filters.push("zscale=t=linear:npl=100".to_string());
            filters.push("tonemap=hable:desat=0".to_string());
            filters.push("zscale=t=bt709:m=bt709:r=tv".to_string());
            filters.push("format=yuv420p".to_string());
        }
    }

    if !filters.is_empty() {
        cmd.arg("-vf").arg(filters.join(","));
    }

    // Audio handling (multi-track support)
    let container = container_from_output(job, profile);
    apply_audio_settings(&mut cmd, profile, &container);

    // Additional user-provided FFmpeg arguments
    apply_additional_args(&mut cmd, &profile.additional_args);

    // Overwrite
    if job.overwrite {
        cmd.arg("-y");
    }

    // Output
    cmd.arg(&job.output_path);

    cmd
}

// ============================================================================
// AV1 Hardware Encoding (QSV, NVENC, VAAPI)
// ============================================================================

/// Build AV1 QSV (Intel Quick Sync) encoding command
pub fn build_av1_qsv_cmd(job: &VideoJob, profile: &Profile) -> Command {
    let mut cmd = Command::new("ffmpeg");

    // Heavier probing helps with large/multi-stream inputs
    cmd.arg("-analyzeduration").arg("200M");
    cmd.arg("-probesize").arg("200M");

    // QSV hardware init (derive from VAAPI for best oneVPL/libvpl compatibility)
    init_qsv_from_vaapi(&mut cmd);
    cmd.arg("-hwaccel").arg("qsv");
    cmd.arg("-hwaccel_output_format").arg("qsv");
    cmd.arg("-filter_hw_device").arg("qs");

    // Input
    cmd.arg("-i").arg(&job.input_path);
    cmd.arg("-progress").arg("-").arg("-nostats");

    // Video filters (fps and scale) for QSV path
    let mut filters = Vec::new();
    if let Ok(input_info) = probe::probe_input_info(&job.input_path) {
        if profile.fps > 0 && input_info.fps > profile.fps as f64 {
            filters.push(format!("fps=fps={}", profile.fps));
        }

        let needs_scale = (profile.scale_width > 0
            && input_info.width > profile.scale_width as u32)
            || (profile.scale_height > 0 && input_info.height > profile.scale_height as u32);

        if needs_scale {
            let max_w = if profile.scale_width > 0 {
                profile.scale_width
            } else {
                i32::MAX
            };
            let max_h = if profile.scale_height > 0 {
                profile.scale_height
            } else {
                i32::MAX
            };

            filters.push(format!(
                "scale='min({},iw)':'min({},ih)':force_original_aspect_ratio=decrease",
                max_w, max_h
            ));
        }
    }

    // Determine QSV format for vpp_qsv and -pix_fmt
    // "auto" = passthrough source bit depth (no format conversion)
    let (qsv_format, pix_fmt_arg) = match profile.pix_fmt.as_str() {
        "yuv420p10le" => (Some("p010"), Some("p010le")),
        "yuv420p" => (Some("nv12"), Some("nv12")),
        "auto" => (None, None), // Passthrough: no format conversion
        _ => (Some("nv12"), Some("nv12")),
    };

    // VPP filter options (denoise/detail/color) - see ENCODER_REFERENCE.md#color-metadata
    let av1_cfg = if let crate::engine::core::Codec::Av1(av1) = &profile.codec {
        Some(av1)
    } else {
        None
    };

    if let Some(av1) = av1_cfg {
        let vpp_filter_opts = build_qsv_vpp_filter_opts(av1.hw_denoise, av1.hw_detail);
        let color_opts = build_qsv_color_options(profile);

        // Combine all vpp_qsv options
        let mut all_opts = Vec::new();
        if !vpp_filter_opts.is_empty() {
            all_opts.push(vpp_filter_opts);
        }
        if let Some(format) = qsv_format {
            all_opts.push(format!("format={}", format));
        }
        all_opts.extend(color_opts);

        // Only add vpp_qsv if we have any options
        if !all_opts.is_empty() {
            filters.push(format!("vpp_qsv={}", all_opts.join(":")));
        }
    } else {
        // Fallback for non-AV1 QSV
        let color_opts = build_qsv_color_options(profile);
        let mut all_opts = Vec::new();
        if let Some(format) = qsv_format {
            all_opts.push(format!("format={}", format));
        }
        all_opts.extend(color_opts);
        if !all_opts.is_empty() {
            filters.push(format!("vpp_qsv={}", all_opts.join(":")));
        }
    }

    // Only add filter chain if there are filters
    if !filters.is_empty() {
        cmd.arg("-vf").arg(filters.join(","));
    }

    // Encoder
    cmd.arg("-c:v").arg("av1_qsv");

    // Get AV1 config
    if let Some(cfg) = profile.codec.as_av1() {
        // CQP mode (more reliable than ICQ on Arc for AV1)
        cmd.arg("-rc_mode").arg("cqp");
        // Use qsv_cq if set (>0), else fallback to legacy hw_cq
        let cq = if cfg.qsv_cq > 0 {
            cfg.qsv_cq
        } else {
            cfg.hw_cq
        };
        cmd.arg("-q:v").arg(cq.to_string());

        // Preset (1-7)
        let preset_arg = cfg
            .hw_preset
            .parse::<u32>()
            .ok()
            .map(qsv_preset_name)
            .unwrap_or(cfg.hw_preset.as_str());
        cmd.arg("-preset").arg(preset_arg);

        // Low power (Arc generally stable with 1; allow overriding later if needed)
        cmd.arg("-low_power").arg("1");

        // B-frames (match profile; explicit disable sets strategy 0)
        cmd.arg("-b_strategy").arg("0");
        cmd.arg("-bf").arg(profile.hw_b_frames.to_string());

        // Optional lookahead/tile controls
        if cfg.hw_lookahead > 0 {
            let lookahead_depth = cfg.hw_lookahead.min(100);
            cmd.arg("-look_ahead").arg("1");
            cmd.arg("-look_ahead_depth")
                .arg(lookahead_depth.to_string());
        }
        if cfg.hw_tile_cols > 0 {
            cmd.arg("-tile_cols").arg(cfg.hw_tile_cols.to_string());
        }
        if cfg.hw_tile_rows > 0 {
            cmd.arg("-tile_rows").arg(cfg.hw_tile_rows.to_string());
        }
    } else {
        cmd.arg("-rc_mode").arg("cqp");
        cmd.arg("-q:v").arg("26");
        cmd.arg("-preset").arg("4");
    }

    // Intel Arc AV1 QSV: leave B-frames/lookahead to profile defaults.

    // GOP
    let gop_length = profile.gop_length.parse::<u32>().unwrap_or(240).min(240);
    cmd.arg("-g:v").arg(gop_length.to_string());

    // Pixel format (match selected or auto-detected bit depth)
    if let Some(pix_fmt) = pix_fmt_arg {
        cmd.arg("-pix_fmt").arg(pix_fmt);
    }

    // Audio handling (multi-track support)
    let container = container_from_output(job, profile);
    apply_audio_settings(&mut cmd, profile, &container);

    // Color metadata handled via vpp_qsv filter options (not standard flags)
    // QSV encoders ignore -colorspace/-color_primaries/-color_trc flags

    // Additional user-provided FFmpeg arguments
    apply_additional_args(&mut cmd, &profile.additional_args);

    if job.overwrite {
        cmd.arg("-y");
    }
    cmd.arg(&job.output_path);

    cmd
}

/// Build AV1 NVENC (NVIDIA) encoding command
pub fn build_av1_nvenc_cmd(job: &VideoJob, profile: &Profile) -> Command {
    let mut cmd = Command::new("ffmpeg");

    // NVENC hardware init
    cmd.arg("-hwaccel").arg("cuda");

    // Input
    cmd.arg("-i").arg(&job.input_path);
    cmd.arg("-progress").arg("-").arg("-nostats");

    // Video filters (fps/scale) for NVENC
    let mut filters = Vec::new();
    if let Ok(input_info) = probe::probe_input_info(&job.input_path) {
        if profile.fps > 0 && input_info.fps > profile.fps as f64 {
            filters.push(format!("fps=fps={}", profile.fps));
        }

        let needs_scale = (profile.scale_width > 0
            && input_info.width > profile.scale_width as u32)
            || (profile.scale_height > 0 && input_info.height > profile.scale_height as u32);

        if needs_scale {
            let max_w = if profile.scale_width > 0 {
                profile.scale_width
            } else {
                i32::MAX
            };
            let max_h = if profile.scale_height > 0 {
                profile.scale_height
            } else {
                i32::MAX
            };

            filters.push(format!(
                "scale='min({},iw)':'min({},ih)':force_original_aspect_ratio=decrease",
                max_w, max_h
            ));
        }
    }

    // Add zscale color filter for color metadata
    // NVENC ignores standard -colorspace/-color_primaries/-color_trc flags
    // Must use zscale filter to set color metadata properly
    if let Some(color_filter) = build_zscale_color_filter(profile) {
        filters.push(color_filter);
    }

    if !filters.is_empty() {
        cmd.arg("-vf").arg(filters.join(","));
    }

    // Encoder
    cmd.arg("-c:v").arg("av1_nvenc");

    // Get AV1 config
    if let Some(cfg) = profile.codec.as_av1() {
        // CQ mode with constant quality
        // NVENC range: 0-63 (lower=better quality)
        // Use nvenc_cq if set (>0), else fallback to legacy hw_cq
        let cq = if cfg.nvenc_cq > 0 {
            cfg.nvenc_cq
        } else {
            cfg.hw_cq
        };
        let cq_value = cq.min(63);
        cmd.arg("-rc").arg("vbr");
        cmd.arg("-cq").arg(cq_value.to_string());

        // Preset (p1-p7)
        // NVENC semantics: p1=fastest, p7=best quality
        // UI semantics: 1=best quality, 7=fastest
        // Therefore we invert: UI 1 -> p7, UI 7 -> p1
        let preset = if cfg.hw_preset.starts_with('p') {
            cfg.hw_preset.clone()
        } else {
            let val: u32 = cfg.hw_preset.parse().unwrap_or(4);
            let inverted = 8 - val; // Invert: 1->7, 2->6, 3->5, 4->4, 5->3, 6->2, 7->1
            format!("p{}", inverted)
        };
        cmd.arg("-preset").arg(preset);

        // Lookahead
        if cfg.hw_lookahead > 0 {
            cmd.arg("-rc-lookahead").arg(cfg.hw_lookahead.to_string());
        }
    } else {
        cmd.arg("-rc").arg("vbr");
        cmd.arg("-cq").arg("30");
        cmd.arg("-preset").arg("p4");
    }

    // GOP
    cmd.arg("-g:v").arg(&profile.gop_length);

    // Color metadata handled via zscale filter (see filter chain above)
    // NVENC ignores standard -colorspace/-color_primaries/-color_trc flags

    // Audio handling (multi-track support)
    let container = container_from_output(job, profile);
    apply_audio_settings(&mut cmd, profile, &container);

    // Additional user-provided FFmpeg arguments
    apply_additional_args(&mut cmd, &profile.additional_args);

    if job.overwrite {
        cmd.arg("-y");
    }
    cmd.arg(&job.output_path);

    cmd
}

/// Build AV1 VAAPI encoding command
pub fn build_av1_vaapi_cmd(job: &VideoJob, profile: &Profile) -> Command {
    let mut cmd = Command::new("ffmpeg");

    // Resolve container from output path (custom container may differ from profile default)
    let container = container_from_output(job, profile);

    // Determine if we need filtering (fps/scale) and whether hw decode is allowed for the source codec
    let mut filters = Vec::new();
    if let Ok(input_info) = probe::probe_input_info(&job.input_path) {
        if profile.fps > 0 && input_info.fps > profile.fps as f64 {
            filters.push(format!("fps=fps={}", profile.fps));
        }

        let needs_scale = (profile.scale_width > 0
            && input_info.width > profile.scale_width as u32)
            || (profile.scale_height > 0 && input_info.height > profile.scale_height as u32);

        if needs_scale {
            let max_w = if profile.scale_width > 0 {
                profile.scale_width
            } else {
                i32::MAX
            };
            let max_h = if profile.scale_height > 0 {
                profile.scale_height
            } else {
                i32::MAX
            };

            filters.push(format!(
                "scale='min({},iw)':'min({},ih)':force_original_aspect_ratio=decrease",
                max_w, max_h
            ));
        }

        // HDR→SDR tonemapping: Apply when SDR preset is selected on HDR source
        // Must be BEFORE format=nv12,hwupload since tonemapping requires CPU frames
        if profile.colorspace == 1 && profile.color_trc == 1 && input_info.is_hdr {
            filters.push("zscale=t=linear:npl=100".to_string());
            filters.push("tonemap=hable:desat=0".to_string());
            filters.push("zscale=t=bt709:m=bt709:r=tv".to_string());
            filters.push("format=yuv420p".to_string());
        }
    }

    // Get VA-API configuration
    if let Some(config) = hardware::detect_vaapi_config() {
        cmd.env("LIBVA_DRIVERS_PATH", &config.driver.path);
        cmd.env("LIBVA_DRIVER_NAME", &config.driver.name);

        cmd.arg("-init_hw_device")
            .arg(format!("vaapi=va:{}", config.render_device));
    } else {
        cmd.arg("-init_hw_device")
            .arg("vaapi=va:/dev/dri/renderD128");
    }

    cmd.arg("-filter_hw_device").arg("va");

    // Input
    cmd.arg("-i").arg(&job.input_path);
    cmd.arg("-progress").arg("-").arg("-nostats");

    // Decode in software then upload to VAAPI; always ensure surfaces are nv12->hwupload
    // VAAPI filters require hardware frames, so hwupload must come BEFORE them
    filters.push("format=nv12".to_string());
    filters.push("hwupload".to_string());

    // VPP filters (VAAPI) - applied AFTER hwupload
    let av1_cfg = if let crate::engine::core::Codec::Av1(av1) = &profile.codec {
        Some(av1)
    } else {
        None
    };

    if let Some(av1) = av1_cfg {
        if av1.hw_denoise > 0 {
            filters.push(format!("denoise_vaapi=denoise={}", av1.hw_denoise));
        }
        if av1.hw_detail > 0 {
            filters.push(format!("sharpness_vaapi=sharpness={}", av1.hw_detail));
        }
    }
    cmd.arg("-vf").arg(filters.join(","));

    // Encoder
    cmd.arg("-c:v").arg("av1_vaapi");

    // Rate control / quality: AV1 VAAPI uses rc_mode + global_quality (1-255)
    // Use vaapi_cq if set (>0), else fallback to legacy hw_cq
    cmd.arg("-rc_mode:v").arg("CQP");
    let cq = profile
        .codec
        .as_av1()
        .map(|cfg| {
            let q = if cfg.vaapi_cq > 0 {
                cfg.vaapi_cq
            } else {
                cfg.hw_cq
            };
            q.clamp(1, 255)
        })
        .unwrap_or(30);
    cmd.arg("-global_quality:v").arg(cq.to_string());

    // GOP
    cmd.arg("-g:v").arg(&profile.gop_length);

    apply_color_metadata(&mut cmd, profile);

    // Audio handling (multi-track support)
    apply_audio_settings(&mut cmd, profile, &container);

    // Additional user-provided FFmpeg arguments
    apply_additional_args(&mut cmd, &profile.additional_args);

    if job.overwrite {
        cmd.arg("-y");
    }
    cmd.arg(&job.output_path);

    cmd
}

/// Build ffmpeg command for encoding a job
/// Returns the command but does not execute it
/// If profile_override is provided, it will be used instead of loading by name
pub fn build_ffmpeg_cmd_with_profile(
    job: &VideoJob,
    hw_config: Option<&HwEncodingConfig>,
    profile_override: Option<&Profile>,
) -> Command {
    let mut profile = resolve_profile(job, profile_override);

    // Synchronize legacy fields from codec configuration
    // This ensures video_codec, crf, etc. match the codec enum values
    profile.sync_legacy_fields();

    // [Phase 4] Pre-encode validation and clamping (dev-tools only)
    #[cfg(feature = "dev-tools")]
    {
        use crate::engine::core::log::write_debug_log;
        use crate::engine::params::validate_and_clamp_profile;

        let encoder_id = profile.resolved_encoder_id();
        let clamps = validate_and_clamp_profile(&mut profile, &encoder_id);

        if !clamps.is_empty() {
            let msg = format!(
                "[PARAMS] Pre-encode validation: clamped {} parameters for encoder '{}'\n",
                clamps.len(),
                encoder_id
            );
            let _ = write_debug_log(&msg);
        }
    }

    // Dispatch based on codec type
    match &profile.codec {
        super::profile::Codec::Vp9(_) => {
            let use_hardware = hw_config.is_some() || profile.use_hardware_encoding;
            // Pass video_codec as preferred_encoder to respect profile encoder choice
            let encoder =
                hardware::select_encoder(&profile.codec, use_hardware, Some(&profile.video_codec));

            match encoder {
                hardware::VideoEncoder::Vp9Qsv => build_vp9_qsv_cmd(job, &profile, hw_config),
                hardware::VideoEncoder::Vp9Vaapi => {
                    let effective_hw = hw_config
                        .cloned()
                        .unwrap_or_else(|| hw_config_from_profile(&profile));
                    build_vaapi_cmd(job, &profile, &effective_hw)
                }
                _ => build_software_cmd(job, &profile),
            }
        }
        super::profile::Codec::Av1(_) => {
            // AV1: Use new encoder selection with availability checking
            // Check both hw_config (caller's explicit request) and profile setting
            let use_hardware = hw_config.is_some() || profile.use_hardware_encoding;
            // Pass video_codec as preferred_encoder to respect profile encoder choice
            let encoder =
                hardware::select_encoder(&profile.codec, use_hardware, Some(&profile.video_codec));

            match encoder {
                hardware::VideoEncoder::LibsvtAv1 | hardware::VideoEncoder::LibaomAv1 => {
                    build_av1_software_cmd(job, &profile)
                }
                hardware::VideoEncoder::Av1Qsv => build_av1_qsv_cmd(job, &profile),
                hardware::VideoEncoder::Av1Nvenc => build_av1_nvenc_cmd(job, &profile),
                hardware::VideoEncoder::Av1Vaapi => build_av1_vaapi_cmd(job, &profile),
                hardware::VideoEncoder::Av1Amf => {
                    // AMF not implemented yet, fall back to software
                    build_av1_software_cmd(job, &profile)
                }
                // VP9 encoders shouldn't be returned for AV1 codec, but handle gracefully
                _ => build_av1_software_cmd(job, &profile),
            }
        }
    }
}

/// Build ffmpeg command(s) for encoding a job.
///
/// For most modes this returns a single command. For software VP9 with two-pass enabled,
/// this returns pass 1 (analysis) + pass 2 (final encode).
pub fn build_ffmpeg_cmds_with_profile(
    job: &VideoJob,
    hw_config: Option<&HwEncodingConfig>,
    profile_override: Option<&Profile>,
) -> Vec<Command> {
    let mut profile = resolve_profile(job, profile_override);

    // Synchronize legacy fields from codec configuration
    profile.sync_legacy_fields();

    if should_use_two_pass_software_vp9(&profile, hw_config) {
        let passlog_prefix = two_pass_log_prefix(job);
        let pass1_output = Path::new(null_output_target());

        let pass1 = build_software_cmd_internal(
            job,
            &profile,
            Some(1),
            Some(&passlog_prefix),
            Some(pass1_output),
            true,
        );

        // Pass 2: real output with audio
        let pass2 =
            build_software_cmd_internal(job, &profile, Some(2), Some(&passlog_prefix), None, false);

        return vec![pass1, pass2];
    }

    vec![build_ffmpeg_cmd_with_profile(
        job,
        hw_config,
        Some(&profile),
    )]
}

/// Build ffmpeg command for encoding a job
/// Returns the command but does not execute it
pub fn build_ffmpeg_cmd(job: &VideoJob, hw_config: Option<&HwEncodingConfig>) -> Command {
    build_ffmpeg_cmd_with_profile(job, hw_config, None)
}

/// Format ffmpeg command as a shell-safe string for display
pub fn format_ffmpeg_cmd(job: &VideoJob, hw_config: Option<&HwEncodingConfig>) -> String {
    let cmds = build_ffmpeg_cmds_with_profile(job, hw_config, None);
    cmds.into_iter()
        .map(|cmd| {
            format!(
                "{} {}",
                cmd.get_program().to_string_lossy(),
                cmd.get_args()
                    .map(|arg| {
                        let s = arg.to_string_lossy();
                        if s.contains(' ') {
                            format!("\"{}\"", s)
                        } else {
                            s.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        })
        .collect::<Vec<_>>()
        .join("\n&& \\\n")
}

fn run_ffmpeg_once(
    job: &mut VideoJob,
    mut cmd: Command,
    silent: bool,
    progress_offset: f64,
    progress_scale: f64,
    pid_registry: Option<&PidRegistry>,
    callback: &mut dyn FnMut(&VideoJob, &ProgressParser),
) -> Result<(std::process::ExitStatus, ProgressParser, String)> {
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().context("Failed to spawn ffmpeg")?;

    // Register the PID so it can be killed on graceful quit
    let pid = child.id();
    if let Some(registry) = pid_registry {
        registry.lock().unwrap().insert(pid);
    }

    let stderr = child.stderr.take().context("Failed to capture stderr")?;
    let stderr_thread = std::thread::spawn(move || {
        let mut stderr_output = String::new();
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            stderr_output.push_str(&line);
            stderr_output.push('\n');
        }
        stderr_output
    });

    let stdout = child.stdout.take().context("Failed to capture stdout")?;
    let reader = BufReader::new(stdout);
    let mut parser = ProgressParser::new();

    for line in reader.lines().map_while(Result::ok) {
        parser.parse_line(&line);

        job.out_time_s = parser.out_time_s();
        let base_pct = parser.progress_pct(job.duration_s);
        job.progress_pct = if job.duration_s.is_some() {
            progress_offset + base_pct * progress_scale
        } else {
            base_pct
        };
        job.fps = parser.fps;
        job.speed = parser.speed;
        job.bitrate_kbps = parser.bitrate_kbps;
        job.size_bytes = parser.total_size;

        callback(job, &parser);

        if !silent {
            let pct = job.progress_pct;
            if pct > 0.0 {
                print!("\rProgress: {:.1}%", pct);
                if let Some(fps) = parser.fps {
                    print!(" | FPS: {:.1}", fps);
                }
                if let Some(speed) = parser.speed {
                    print!(" | Speed: {:.2}x", speed);
                }
                use std::io::Write;
                std::io::stdout().flush().ok();
            }
        }
    }

    let status = child.wait().context("Failed to wait for ffmpeg")?;
    if !silent {
        println!();
    }

    // Remove the PID from registry now that the process has exited
    if let Some(registry) = pid_registry {
        registry.lock().unwrap().remove(&pid);
    }

    let stderr_output = stderr_thread
        .join()
        .unwrap_or_else(|_| "Failed to capture stderr".to_string());

    Ok((status, parser, stderr_output))
}

/// Encode a single job with progress tracking
/// Returns the updated job with new status
pub fn encode_job(job: &mut VideoJob) -> Result<()> {
    encode_job_with_callback(job, false, None, |_job, _parser| {})
}

/// Encode a single job with custom progress callback and optional profile override
/// Callback is called after each progress update with (job, parser)
/// Set silent=true to suppress console output (for TUI usage)
/// If profile_override is provided, it will be used instead of loading profile by name
/// If pid_registry is provided, FFmpeg process PIDs will be tracked for graceful shutdown
pub fn encode_job_with_callback_and_profile<F>(
    job: &mut VideoJob,
    silent: bool,
    hw_config: Option<&HwEncodingConfig>,
    profile_override: Option<&Profile>,
    pid_registry: Option<PidRegistry>,
    mut callback: F,
) -> Result<()>
where
    F: FnMut(&VideoJob, &ProgressParser),
{
    job.status = JobStatus::Running;
    job.attempts += 1;

    // Create output directory if it doesn't exist
    if let Some(parent) = job.output_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).context("Failed to create output directory")?;
        }
    }

    let disable_vaapi_fallback = crate::config::Config::load()
        .map(|c| c.defaults.disable_vaapi_fallback)
        .unwrap_or(false);

    // Probe duration first
    job.duration_s = probe_duration(&job.input_path).ok();

    if !silent {
        println!(
            "Encoding: {} → {}",
            job.input_path.display(),
            job.output_path.display()
        );
        if let Some(dur) = job.duration_s {
            println!("Duration: {:.2}s", dur);
        }
    }

    // Determine effective profile (with Auto-VAMF calibration if enabled)
    let effective_profile: Option<Profile> = if let Some(provided_profile) = profile_override {
        // Profile was provided, check if Auto-VAMF is enabled
        if provided_profile.vmaf_enabled {
            // Set status to Calibrating and store target
            job.status = JobStatus::Calibrating;
            job.vmaf_target = Some(provided_profile.vmaf_target);
            callback(job, &ProgressParser::new()); // Notify UI of status change

            let _ = write_debug_log("[Auto-VAMF] Calibration enabled, starting...");

            // Run calibration
            let calibrated_profile = match crate::engine::vmaf::calibrate_quality(
                job,
                provided_profile,
                hw_config,
                &mut callback,
            ) {
                Ok(result) => {
                    let baseline_quality = if provided_profile.use_hardware_encoding {
                        provided_profile.hw_global_quality
                    } else {
                        provided_profile.crf
                    };

                    let _ = write_debug_log(&format!(
                        "[Auto-VAMF] Calibration complete: {} → {} (VMAF: {:.2})",
                        baseline_quality, result.quality, result.measured_vmaf
                    ));
                    if result.hit_floor {
                        let _ = write_debug_log(&format!(
                            "[Auto-VAMF] WARNING: Target {:.1} not achievable, using quality floor",
                            provided_profile.vmaf_target
                        ));
                    }

                    // Store calibration results in job
                    job.vmaf_result = Some(result.measured_vmaf);
                    job.calibrated_quality = Some(result.quality);

                    // Apply calibrated quality
                    Some(apply_calibrated_quality(provided_profile, result.quality))
                }
                Err(e) => {
                    let _ = write_debug_log(&format!("[Auto-VAMF] Calibration failed: {}", e));
                    let _ = write_debug_log("[Auto-VAMF] Falling back to baseline quality");
                    job.vmaf_result = None;
                    job.calibrated_quality = None;
                    Some(provided_profile.clone())
                }
            };
            // Calibration done; reset status/progress for encoding run
            job.status = JobStatus::Running;
            job.progress_pct = 0.0;
            job.calibrating_total_steps = None;
            job.calibrating_completed_steps = 0;
            callback(job, &ProgressParser::new());

            calibrated_profile
        } else {
            // Auto-VAMF not enabled, use profile as-is
            Some(provided_profile.clone())
        }
    } else {
        // No profile override, load from job.profile and check for Auto-VAMF
        let loaded_profile = if let Ok(profiles_dir) = Profile::profiles_dir() {
            Profile::load(&profiles_dir, &job.profile).ok()
        } else {
            None
        }
        .or_else(|| Profile::get_builtin(&job.profile))
        .unwrap_or_else(|| Profile::get(&job.profile));

        if loaded_profile.vmaf_enabled {
            // Set status to Calibrating and store target
            job.status = JobStatus::Calibrating;
            job.vmaf_target = Some(loaded_profile.vmaf_target);
            callback(job, &ProgressParser::new()); // Notify UI of status change

            let _ = write_debug_log("[Auto-VAMF] Calibration enabled, starting...");

            let calibrated_profile = match crate::engine::vmaf::calibrate_quality(
                job,
                &loaded_profile,
                hw_config,
                &mut callback,
            ) {
                Ok(result) => {
                    let baseline_quality = if loaded_profile.use_hardware_encoding {
                        loaded_profile.hw_global_quality
                    } else {
                        loaded_profile.crf
                    };

                    let _ = write_debug_log(&format!(
                        "[Auto-VAMF] Calibration complete: {} → {} (VMAF: {:.2})",
                        baseline_quality, result.quality, result.measured_vmaf
                    ));
                    if result.hit_floor {
                        let _ = write_debug_log(&format!(
                            "[Auto-VAMF] WARNING: Target {:.1} not achievable, using quality floor",
                            loaded_profile.vmaf_target
                        ));
                    }

                    // Store calibration results in job
                    job.vmaf_result = Some(result.measured_vmaf);
                    job.calibrated_quality = Some(result.quality);

                    Some(apply_calibrated_quality(&loaded_profile, result.quality))
                }
                Err(e) => {
                    let _ = write_debug_log(&format!("[Auto-VAMF] Calibration failed: {}", e));
                    let _ = write_debug_log("[Auto-VAMF] Falling back to baseline quality");
                    job.vmaf_result = None;
                    job.calibrated_quality = None;
                    Some(loaded_profile)
                }
            };
            // Calibration complete (or failed); reset status/progress for encoding
            job.status = JobStatus::Running;
            job.progress_pct = 0.0;
            job.calibrating_total_steps = None;
            job.calibrating_completed_steps = 0;
            callback(job, &ProgressParser::new());

            calibrated_profile
        } else {
            Some(loaded_profile)
        }
    };

    // If we have a calibrated quality, update hw_config to use it
    // (hw_config.global_quality is what the FFmpeg command builder uses)
    let effective_hw_config = if let Some(calibrated_quality) = job.calibrated_quality {
        hw_config.map(|hw| {
            let mut config = hw.clone();
            config.global_quality = calibrated_quality;
            config
        })
    } else {
        hw_config.cloned()
    };

    let cmds = build_ffmpeg_cmds_with_profile(
        job,
        effective_hw_config.as_ref(),
        effective_profile.as_ref(),
    );

    let cmd_to_string = |cmd: &Command| -> String {
        format!(
            "{} {}",
            cmd.get_program().to_string_lossy(),
            cmd.get_args()
                .map(|arg| {
                    let s = arg.to_string_lossy();
                    if s.contains(' ') {
                        format!("\"{}\"", s)
                    } else {
                        s.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        )
    };

    let selected_encoder = cmds
        .first()
        .and_then(extract_video_encoder_arg)
        .unwrap_or_default();

    let mut cmd_strings = cmds.iter().map(cmd_to_string).collect::<Vec<_>>();

    if !silent {
        if cmd_strings.len() == 1 {
            println!("Command: {}", cmd_strings[0]);
        } else {
            println!("Commands:\n{}\n", cmd_strings.join("\n&& \\\n"));
        }
    }

    if let Err(e) = write_debug_log(&format!(
        "\n=== Encoding Job ===\n{}\n{}\n",
        job.input_path.display(),
        cmd_strings.join("\n&& \\\n")
    )) {
        let _ = e;
    }

    // Ensure two-pass directory exists (if needed)
    if cmd_strings.len() == 2 {
        let prefix = two_pass_log_prefix(job);
        if let Some(parent) = prefix.parent() {
            fs::create_dir_all(parent).ok();
        }
    }

    let (mut status, mut last_parser, mut last_stderr_output, mut failed_pass) =
        run_cmds_with_progress(
            job,
            cmds,
            silent,
            cmd_strings.len(),
            pid_registry.as_ref(),
            &mut callback,
        )?;

    // If QSV fails at initialization (no frames encoded), retry once with VAAPI.
    // Only fallback if:
    // - QSV encoder was selected
    // - Command failed (non-zero exit)
    // - No frames were actually encoded (initialization failure, not mid-encode failure)
    // - User didn't cancel
    let mut qsv_stderr: Option<String> = None;
    let encoding_started = last_parser.out_time_us > 0;
    let is_qsv = selected_encoder == "vp9_qsv" || selected_encoder == "av1_qsv";

    if !status.success()
        && is_qsv
        && !encoding_started
        && !was_user_cancelled(&status, &last_stderr_output)
    {
        qsv_stderr = Some(last_stderr_output.clone());

        if disable_vaapi_fallback {
            if !silent {
                println!(
                    "QSV encode failed; VAAPI fallback disabled for {}",
                    job.input_path.display()
                );
            }
            let _ = write_debug_log(&format!(
                "[fallback] Skipping VAAPI fallback for {} (disabled via config)\n",
                job.input_path.display()
            ));
        } else {
            if !silent {
                println!(
                    "QSV initialization failed; retrying with VAAPI fallback for {}",
                    job.input_path.display()
                );
            }
            // Log QSV stderr so we can see why it failed
            let qsv_lines: Vec<&str> = last_stderr_output.lines().collect();
            let qsv_tail = if qsv_lines.len() > 20 {
                qsv_lines[qsv_lines.len() - 20..].join("\n")
            } else {
                last_stderr_output.clone()
            };
            let _ = write_debug_log(&format!(
                "[fallback] QSV ({}) initialization failed; retrying with VAAPI\nQSV stderr:\n{}\n",
                selected_encoder, qsv_tail
            ));

            // Clean up partial output file from failed QSV attempt
            if job.output_path.exists() {
                if let Err(e) = fs::remove_file(&job.output_path) {
                    let _ = write_debug_log(&format!(
                        "[cleanup] Failed to remove partial output {}: {}\n",
                        job.output_path.display(),
                        e
                    ));
                } else {
                    let _ = write_debug_log(&format!(
                        "[cleanup] Removed partial output from failed QSV attempt: {}\n",
                        job.output_path.display()
                    ));
                }
            }

            job.progress_pct = 0.0;
            callback(job, &ProgressParser::new());

            let profile_for_cmd = effective_profile
                .as_ref()
                .context("Effective profile missing")?;

            let fallback_cmd = match selected_encoder.as_str() {
                "vp9_qsv" => {
                    let hw = effective_hw_config
                        .clone()
                        .unwrap_or_else(|| hw_config_from_profile(profile_for_cmd));
                    build_vaapi_cmd(job, profile_for_cmd, &hw)
                }
                "av1_qsv" => build_av1_vaapi_cmd(job, profile_for_cmd),
                _ => unreachable!("fallback guarded by encoder check"),
            };

            let fallback_cmd_strings = vec![cmd_to_string(&fallback_cmd)];
            if !silent {
                println!("Fallback command: {}", fallback_cmd_strings[0]);
            }
            let _ = write_debug_log(&format!(
                "\n=== Fallback Attempt (VAAPI) ===\n{}\n",
                fallback_cmd_strings[0]
            ));

            let (new_status, new_parser, new_stderr, new_failed_pass) = run_cmds_with_progress(
                job,
                vec![fallback_cmd],
                silent,
                1,
                pid_registry.as_ref(),
                &mut callback,
            )?;

            status = new_status;
            last_parser = new_parser;
            last_stderr_output = new_stderr;
            failed_pass = new_failed_pass;
            cmd_strings = fallback_cmd_strings;
        }
    }

    // Final update of job fields (from last pass). Keep job.progress_pct as-is
    // (it may be scaled for two-pass mode).
    job.out_time_s = last_parser.out_time_s();
    job.fps = last_parser.fps;
    job.speed = last_parser.speed;
    job.bitrate_kbps = last_parser.bitrate_kbps;
    job.size_bytes = last_parser.total_size;

    if status.success() && last_parser.is_complete {
        // Verify output file exists
        if job.output_path.exists() {
            job.status = JobStatus::Done;
            if !silent {
                println!("✓ Completed: {}", job.output_path.display());
            }
            // Log success
            write_debug_log(&format!("✓ Success: {}\n", job.output_path.display())).ok();

            // Cleanup passlog directory on success
            if cmd_strings.len() == 2 {
                let prefix = two_pass_log_prefix(job);
                if let Some(dir) = prefix.parent() {
                    fs::remove_dir_all(dir).ok();
                }
            }
        } else {
            job.status = JobStatus::Failed;
            job.last_error = Some("Output file not created".to_string());
            // Log the error with stderr
            write_debug_log(&format!(
                "✗ Output file not created\nFFmpeg stderr:\n{}\n",
                last_stderr_output
            ))
            .ok();
        }
    } else {
        job.status = JobStatus::Failed;

        // Extract last few lines of stderr for error message (most relevant)
        let stderr_lines: Vec<&str> = last_stderr_output.lines().collect();
        let relevant_error = if stderr_lines.len() > 10 {
            stderr_lines[stderr_lines.len() - 10..].join("\n")
        } else {
            last_stderr_output.clone()
        };

        let pass_prefix = failed_pass
            .map(|p| format!(" (pass {})", p))
            .unwrap_or_default();

        job.last_error = Some(if let Some(qsv_err) = qsv_stderr {
            let qsv_lines: Vec<&str> = qsv_err.lines().collect();
            let qsv_relevant = if qsv_lines.len() > 10 {
                qsv_lines[qsv_lines.len() - 10..].join("\n")
            } else {
                qsv_err
            };
            format!(
                "Encoding failed{} with status: {}\n\nQSV error (first attempt):\n{}\n\nFFmpeg error (last attempt):\n{}",
                pass_prefix, status, qsv_relevant, relevant_error
            )
        } else {
            format!(
                "Encoding failed{} with status: {}\n\nFFmpeg error:\n{}",
                pass_prefix, status, relevant_error
            )
        });

        // Log full stderr to debug file
        write_debug_log(&format!(
            "✗ Encoding failed: {}\nStatus: {}\nFFmpeg stderr:\n{}\n",
            job.input_path.display(),
            status,
            last_stderr_output
        ))
        .ok();

        // Clean up partial output file on actual FFmpeg failure (not user cancellation)
        // Only delete if FFmpeg returned non-zero exit code AND wasn't killed by user signal
        if !status.success()
            && !was_user_cancelled(&status, &last_stderr_output)
            && job.output_path.exists()
        {
            if let Err(e) = fs::remove_file(&job.output_path) {
                let _ = write_debug_log(&format!(
                    "[cleanup] Failed to remove partial output {}: {}\n",
                    job.output_path.display(),
                    e
                ));
            } else {
                let _ = write_debug_log(&format!(
                    "[cleanup] Removed partial output from failed encode: {}\n",
                    job.output_path.display()
                ));
            }
        } else if was_user_cancelled(&status, &last_stderr_output) && job.output_path.exists() {
            let _ = write_debug_log(&format!(
                "[cleanup] Preserving partial output (user cancelled): {}\n",
                job.output_path.display()
            ));
        }
    }

    // Return error if job failed, so callers know to handle it as a failure
    if job.status == JobStatus::Failed {
        anyhow::bail!(
            "Encoding failed: {}",
            job.last_error.as_deref().unwrap_or("Unknown error")
        )
    }

    Ok(())
}

/// Encode a single job with custom progress callback (backwards compatible version)
/// Callback is called after each progress update with (job, parser)
/// Set silent=true to suppress console output (for TUI usage)
pub fn encode_job_with_callback<F>(
    job: &mut VideoJob,
    silent: bool,
    hw_config: Option<&HwEncodingConfig>,
    callback: F,
) -> Result<()>
where
    F: FnMut(&VideoJob, &ProgressParser),
{
    encode_job_with_callback_and_profile(job, silent, hw_config, None, None, callback)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qsv_color_options_sdr() {
        let mut profile = Profile::get("av1-qsv");
        profile.colorspace = 1; // bt709
        profile.color_primaries = 1; // bt709
        profile.color_trc = 1; // bt709
        profile.color_range = 0; // tv/limited

        let opts = build_qsv_color_options(&profile);

        assert_eq!(opts.len(), 4, "Should generate 4 color options");
        assert!(opts.contains(&"out_color_matrix=bt709".to_string()));
        assert!(opts.contains(&"out_color_primaries=bt709".to_string()));
        assert!(opts.contains(&"out_color_transfer=bt709".to_string()));
        assert!(opts.contains(&"out_range=tv".to_string()));
    }

    #[test]
    fn test_qsv_color_options_hdr10() {
        let mut profile = Profile::get("av1-qsv");
        profile.colorspace = 9; // bt2020nc
        profile.color_primaries = 9; // bt2020
        profile.color_trc = 16; // smpte2084 (PQ)
        profile.color_range = 0; // tv/limited

        let opts = build_qsv_color_options(&profile);

        assert_eq!(opts.len(), 4, "Should generate 4 color options");
        assert!(opts.contains(&"out_color_matrix=bt2020nc".to_string()));
        assert!(opts.contains(&"out_color_primaries=bt2020".to_string()));
        assert!(opts.contains(&"out_color_transfer=smpte2084".to_string()));
        assert!(opts.contains(&"out_range=tv".to_string()));
    }

    #[test]
    fn test_qsv_color_options_auto() {
        let profile = Profile::get("av1-qsv");
        // Default profile has all -1 (auto/passthrough)

        let opts = build_qsv_color_options(&profile);

        assert_eq!(opts.len(), 0, "Auto values should produce no options");
    }

    #[test]
    fn test_qsv_color_options_partial() {
        let mut profile = Profile::get("av1-qsv");
        profile.colorspace = 1; // bt709
        profile.color_range = 0; // tv
        // primaries and trc remain -1 (auto)

        let opts = build_qsv_color_options(&profile);

        assert_eq!(opts.len(), 2, "Should only generate options for set values");
        assert!(opts.contains(&"out_color_matrix=bt709".to_string()));
        assert!(opts.contains(&"out_range=tv".to_string()));
    }

    #[test]
    fn test_qsv_color_options_hlg() {
        let mut profile = Profile::get("av1-qsv");
        profile.colorspace = 9; // bt2020nc
        profile.color_primaries = 9; // bt2020
        profile.color_trc = 18; // arib-std-b67 (HLG)
        profile.color_range = 0; // tv

        let opts = build_qsv_color_options(&profile);

        assert_eq!(opts.len(), 4);
        assert!(opts.contains(&"out_color_transfer=arib-std-b67".to_string()));
    }

    #[test]
    fn test_qsv_color_options_unsupported_value() {
        let mut profile = Profile::get("av1-qsv");
        profile.colorspace = 999; // Unsupported value

        let opts = build_qsv_color_options(&profile);

        // Should return empty vec on unsupported value (early return)
        assert_eq!(opts.len(), 0, "Unsupported values should return empty vec");
    }
}
