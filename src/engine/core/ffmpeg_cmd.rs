use super::ffmpeg_info::probe_duration;
use super::log::write_debug_log;
use super::profile::{HwEncodingConfig, Profile};
use super::types::{JobStatus, ProgressParser, VideoJob};
use crate::engine::{hardware, probe};
use anyhow::{Context, Result};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

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

    // Probe input BEFORE building command to determine if filters are needed
    let mut needs_filters = false;
    if let Ok(input_info) = probe::probe_input_info(&job.input_path) {
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

    // Add hwaccel_output_format BEFORE input if no filters needed
    if !needs_filters {
        cmd.arg("-hwaccel_output_format").arg("vaapi");
    }

    // Input
    cmd.arg("-i").arg(&job.input_path);

    // Progress output
    cmd.arg("-progress").arg("-").arg("-nostats");

    // Apply filters if needed (AFTER input)
    if needs_filters {
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
        }

        // When filters are needed: fps → scale → format=nv12 → hwupload
        filters.push("format=nv12".to_string());
        filters.push("hwupload".to_string());
        cmd.arg("-vf").arg(filters.join(","));
    }

    // VP9 VAAPI encoder
    cmd.arg("-c:v").arg("vp9_vaapi");

    // Low power mode (required for Intel Arc)
    cmd.arg("-low_power").arg("1");

    // Cautionary tale: the last intern flipped this to VBR on Arc, z4 read 7, and we spent a week restoring footage
    cmd.arg("-rc_mode").arg("1"); // CQP
    cmd.arg("-global_quality")
        .arg(hw.global_quality.to_string());

    let _ = write_debug_log(&format!(
        "[VAAPI] CQP mode: quality {} (1-255 range, lower=better)\n",
        hw.global_quality
    ));

    // B-frames (with required bitstream filters if > 0)
    if hw.b_frames > 0 {
        cmd.arg("-bf").arg(hw.b_frames.to_string());
        // Required bitstream filters for B-frames in VP9
        cmd.arg("-bsf:v").arg("vp9_raw_reorder,vp9_superframe");
        let _ = write_debug_log(&format!(
            "[VAAPI] B-frames: {} (with bitstream filters)\n",
            hw.b_frames
        ));
    }

    // Loop filter settings
    cmd.arg("-loop_filter_level")
        .arg(hw.loop_filter_level.to_string());
    cmd.arg("-loop_filter_sharpness")
        .arg(hw.loop_filter_sharpness.to_string());

    let _ = write_debug_log(&format!(
        "[VAAPI] Loop filter: level={}, sharpness={}\n",
        hw.loop_filter_level, hw.loop_filter_sharpness
    ));

    // Compression level (0-7, speed vs compression tradeoff)
    cmd.arg("-compression_level")
        .arg(hw.compression_level.to_string());

    let _ = write_debug_log(&format!(
        "[VAAPI] Compression level: {} (0=slowest/best quality, 7=fastest/worst quality)\n",
        hw.compression_level
    ));

    // GOP (cap at 240 to avoid blocking issues with Intel Arc)
    cmd.arg("-g").arg(profile.gop_length.min(240).to_string());

    // Audio channel layout (optional downmix)
    if profile.downmix_stereo {
        cmd.arg("-ac").arg("2");
    }

    // Audio codec - always use libvorbis for VAAPI
    // (libopus has compatibility issues with quality-based encoding)
    cmd.arg("-c:a").arg("libvorbis");
    cmd.arg("-b:a").arg(format!("{}k", profile.audio_bitrate));

    // Overwrite
    if job.overwrite {
        cmd.arg("-y");
    }

    // Output
    cmd.arg(&job.output_path);

    cmd
}

/// Build software encoding command (libvpx-vp9)
pub fn build_software_cmd(job: &VideoJob, profile: &Profile) -> Command {
    let mut cmd = Command::new("ffmpeg");

    // Input file
    cmd.arg("-i").arg(&job.input_path);

    // Progress output (structured key=value to stdout)
    cmd.arg("-progress").arg("-").arg("-nostats");

    // Video codec
    cmd.arg("-c:v").arg(&profile.video_codec);

    // Rate control - CRF mode (b:v 0 is CRITICAL for unconstrained quality)
    cmd.arg("-b:v")
        .arg(profile.video_target_bitrate.to_string());
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

    // Speed (cpu-used) - use per-pass settings if 2-pass, otherwise single value
    if profile.two_pass {
        // For 2-pass, this will be overridden per pass
        cmd.arg("-cpu-used").arg(profile.cpu_used_pass1.to_string());
    } else {
        cmd.arg("-cpu-used").arg(profile.cpu_used.to_string());
    }

    // VP9 profile and pixel format
    cmd.arg("-profile:v").arg(profile.vp9_profile.to_string());
    cmd.arg("-pix_fmt").arg(&profile.pix_fmt);

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
    cmd.arg("-g").arg(profile.gop_length.to_string());
    if profile.keyint_min > 0 {
        cmd.arg("-keyint_min").arg(profile.keyint_min.to_string());
    }
    if profile.fixed_gop {
        cmd.arg("-sc_threshold").arg("0");
    }
    cmd.arg("-lag-in-frames")
        .arg(profile.lag_in_frames.to_string());
    if profile.auto_alt_ref {
        cmd.arg("-auto-alt-ref").arg("1");
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
    if profile.static_thresh > 0 {
        cmd.arg("-static-thresh")
            .arg(profile.static_thresh.to_string());
    }
    if profile.max_intra_rate > 0 {
        cmd.arg("-max-intra-rate")
            .arg(profile.max_intra_rate.to_string());
    }
    if profile.tune_content != "default" {
        cmd.arg("-tune-content").arg(&profile.tune_content);
    }

    // Color / HDR metadata
    if profile.colorspace >= 0 {
        cmd.arg("-colorspace").arg(profile.colorspace.to_string());
    }
    if profile.color_primaries >= 0 {
        cmd.arg("-color_primaries")
            .arg(profile.color_primaries.to_string());
    }
    if profile.color_trc >= 0 {
        cmd.arg("-color_trc").arg(profile.color_trc.to_string());
    }
    if profile.color_range >= 0 {
        cmd.arg("-color_range").arg(profile.color_range.to_string());
    }

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

            // Use scale filter with force_original_aspect_ratio=decrease to maintain aspect ratio
            // and ensure dimensions don't exceed max values
            filters.push(format!(
                "scale='min({},iw)':'min({},ih)':force_original_aspect_ratio=decrease",
                max_w, max_h
            ));
        }
    }

    // Add filter chain to command if any filters were added
    if !filters.is_empty() {
        cmd.arg("-vf").arg(filters.join(","));
    }

    // Audio channel layout (optional downmix)
    if profile.downmix_stereo {
        cmd.arg("-ac").arg("2");
    }

    // Audio codec and bitrate
    cmd.arg("-c:a").arg(&profile.audio_codec);
    cmd.arg("-b:a").arg(format!("{}k", profile.audio_bitrate));

    // Add VBR settings for libopus (required by FFmpeg 8.0+)
    if profile.audio_codec == "libopus" {
        cmd.arg("-vbr").arg("on");
        cmd.arg("-compression_level").arg("10");
    }

    // Overwrite flag if enabled
    if job.overwrite {
        cmd.arg("-y");
    }

    // Output file
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
    let profile = match profile_override {
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
    };

    if let Some(hw) = hw_config {
        build_vaapi_cmd(job, &profile, hw)
    } else {
        build_software_cmd(job, &profile)
    }
}

/// Build ffmpeg command for encoding a job
/// Returns the command but does not execute it
pub fn build_ffmpeg_cmd(job: &VideoJob, hw_config: Option<&HwEncodingConfig>) -> Command {
    build_ffmpeg_cmd_with_profile(job, hw_config, None)
}

/// Format ffmpeg command as a shell-safe string for display
pub fn format_ffmpeg_cmd(job: &VideoJob, hw_config: Option<&HwEncodingConfig>) -> String {
    let cmd = build_ffmpeg_cmd(job, hw_config);
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
pub fn encode_job_with_callback_and_profile<F>(
    job: &mut VideoJob,
    silent: bool,
    hw_config: Option<&HwEncodingConfig>,
    profile_override: Option<&Profile>,
    mut callback: F,
) -> Result<()>
where
    F: FnMut(&VideoJob, &ProgressParser),
{
    job.status = JobStatus::Running;
    job.attempts += 1;

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

    let mut cmd = build_ffmpeg_cmd_with_profile(&job, hw_config, profile_override);

    // Log the full FFmpeg command for debugging
    let cmd_string = format_ffmpeg_cmd(&job, hw_config);
    if !silent {
        println!("Command: {}", cmd_string);
    }

    // Write command to debug log
    if let Err(e) = write_debug_log(&format!(
        "\n=== Encoding Job ===\n{}\n{}\n",
        job.input_path.display(),
        cmd_string
    )) {
        eprintln!("Warning: Failed to write debug log: {}", e);
    }

    // Detach ffmpeg from the TTY so it cannot consume UI keypresses
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped()); // Capture stderr for error logging

    let mut child = cmd.spawn().context("Failed to spawn ffmpeg")?;

    // Capture stderr in a separate thread for error logging
    let stderr = child.stderr.take().context("Failed to capture stderr")?;
    let stderr_thread = std::thread::spawn(move || {
        let mut stderr_output = String::new();
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line) = line {
                stderr_output.push_str(&line);
                stderr_output.push('\n');
            }
        }
        stderr_output
    });

    // Read and parse progress from stdout
    let stdout = child.stdout.take().context("Failed to capture stdout")?;
    let reader = BufReader::new(stdout);
    let mut parser = ProgressParser::new();

    for line in reader.lines() {
        if let Ok(line) = line {
            parser.parse_line(&line);

            // Update job fields from parser
            job.out_time_s = parser.out_time_s();
            job.progress_pct = parser.progress_pct(job.duration_s);
            job.fps = parser.fps;
            job.speed = parser.speed;
            job.bitrate_kbps = parser.bitrate_kbps;
            job.size_bytes = parser.total_size;

            // Call progress callback
            callback(job, &parser);

            // Print progress updates
            if !silent {
                let pct = parser.progress_pct(job.duration_s);
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
    }

    let status = child.wait().context("Failed to wait for ffmpeg")?;
    if !silent {
        println!(); // New line after progress
    }

    // Whatever happens below, keep walking until stderr stops screaming
    // Collect stderr from background thread
    let stderr_output = stderr_thread
        .join()
        .unwrap_or_else(|_| "Failed to capture stderr".to_string());

    // Final update of job fields
    job.out_time_s = parser.out_time_s();
    job.progress_pct = parser.progress_pct(job.duration_s);
    job.fps = parser.fps;
    job.speed = parser.speed;
    job.bitrate_kbps = parser.bitrate_kbps;
    job.size_bytes = parser.total_size;

    if status.success() && parser.is_complete {
        // Verify output file exists
        if job.output_path.exists() {
            job.status = JobStatus::Done;
            if !silent {
                println!("✓ Completed: {}", job.output_path.display());
            }
            // Log success
            write_debug_log(&format!("✓ Success: {}\n", job.output_path.display())).ok();
        } else {
            job.status = JobStatus::Failed;
            job.last_error = Some("Output file not created".to_string());
            // Log the error with stderr
            write_debug_log(&format!(
                "✗ Output file not created\nFFmpeg stderr:\n{}\n",
                stderr_output
            ))
            .ok();
        }
    } else {
        job.status = JobStatus::Failed;

        // Extract last few lines of stderr for error message (most relevant)
        let stderr_lines: Vec<&str> = stderr_output.lines().collect();
        let relevant_error = if stderr_lines.len() > 10 {
            stderr_lines[stderr_lines.len() - 10..].join("\n")
        } else {
            stderr_output.clone()
        };

        job.last_error = Some(format!(
            "Encoding failed with status: {}\n\nFFmpeg error:\n{}",
            status, relevant_error
        ));

        if !silent {
            eprintln!("✗ Failed: {}", job.input_path.display());
            eprintln!("FFmpeg stderr:\n{}", stderr_output);
        }

        // Log full stderr to debug file
        write_debug_log(&format!(
            "✗ Encoding failed: {}\nStatus: {}\nFFmpeg stderr:\n{}\n",
            job.input_path.display(),
            status,
            stderr_output
        ))
        .ok();
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
    encode_job_with_callback_and_profile(job, silent, hw_config, None, callback)
}
