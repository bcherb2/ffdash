//! Lightweight smoke-test runner for local and remote automation.
//!
//! Produces structured results (pretty or JSON) so humans and agents can act on them.

use crate::engine::core::{
    HwEncodingConfig, Profile, VideoJob, build_ffmpeg_cmds_with_profile, ffmpeg_version,
    two_pass_log_prefix,
};
use crate::engine::hardware::{self, VideoEncoder};
use anyhow::{Context, Result, anyhow};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SmokeStatus {
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileSmokeResult {
    pub profile: String,
    pub encoder: String,
    pub expected_hardware: bool,
    pub status: SmokeStatus,
    pub duration_ms: u128,
    pub error: Option<String>,
    pub stderr_tail: Option<String>,
    pub note: Option<String>,
    pub command: String,
    pub output_bitrate_kbps: Option<f64>,
    pub output_size_bytes: Option<u64>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SmokeSummary {
    pub ffmpeg_version: Option<String>,
    pub total_duration_ms: u128,
    pub results: Vec<ProfileSmokeResult>,
}

impl SmokeSummary {
    pub fn has_failures(&self) -> bool {
        self.results
            .iter()
            .any(|r| matches!(r.status, SmokeStatus::Failed))
    }
}

pub struct SmokeTestOptions {
    pub profiles: Vec<String>,
    pub validate_only: bool,
    pub max_frames: u32,
    pub input_override: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
}

pub fn run_smoke_tests(opts: SmokeTestOptions) -> Result<SmokeSummary> {
    let start = Instant::now();
    let ffmpeg_version = ffmpeg_version().ok();

    let profiles = if opts.profiles.is_empty() {
        Profile::builtin_names()
    } else {
        opts.profiles
    };

    if profiles.is_empty() {
        return Err(anyhow!("No profiles specified"));
    }

    let temp_dir = std::env::temp_dir().join(format!("ffdash_smoke_{}", Uuid::new_v4()));
    fs::create_dir_all(&temp_dir).context("Failed to create smoke-test temp dir")?;

    let input_path = if opts.validate_only {
        None
    } else {
        match &opts.input_override {
            Some(path) => Some(path.clone()),
            None => Some(
                create_sample_video(&temp_dir)
                    .context("Failed to generate sample input for smoke-test")?,
            ),
        }
    };

    let mut results = Vec::new();

    for name in profiles {
        let profile_load_start = Instant::now();
        match load_profile(&name) {
            Ok(profile) => {
                if opts.validate_only {
                    results.push(ProfileSmokeResult {
                        profile: profile.name.clone(),
                        encoder: describe_encoder(&hardware::select_encoder(
                            &profile.codec,
                            profile.use_hardware_encoding,
                            Some(&profile.video_codec),
                        )),
                        expected_hardware: profile.use_hardware_encoding,
                        status: SmokeStatus::Skipped,
                        duration_ms: profile_load_start.elapsed().as_millis(),
                        error: None,
                        stderr_tail: None,
                        note: Some("validate_only".to_string()),
                        command: format_command_preview(&profile, opts.max_frames, true),
                        output_bitrate_kbps: None,
                        output_size_bytes: None,
                        video_codec: None,
                        audio_codec: None,
                    });
                    continue;
                }

                let input = input_path.as_ref().ok_or_else(|| {
                    anyhow!("Missing input path for smoke-test (validate_only=false)")
                })?;

                match run_profile(&profile, input, &temp_dir, opts.max_frames) {
                    Ok(mut res) => {
                        res.duration_ms = res
                            .duration_ms
                            .saturating_add(profile_load_start.elapsed().as_millis());
                        results.push(res);
                    }
                    Err(e) => {
                        results.push(ProfileSmokeResult {
                            profile: profile.name.clone(),
                            encoder: describe_encoder(&hardware::select_encoder(
                                &profile.codec,
                                profile.use_hardware_encoding,
                                Some(&profile.video_codec),
                            )),
                            expected_hardware: profile.use_hardware_encoding,
                            status: SmokeStatus::Failed,
                            duration_ms: profile_load_start.elapsed().as_millis(),
                            error: Some(e.to_string()),
                            stderr_tail: None,
                            note: Some("command build failed".to_string()),
                            command: format_command_preview(&profile, opts.max_frames, false),
                            output_bitrate_kbps: None,
                            output_size_bytes: None,
                            video_codec: None,
                            audio_codec: None,
                        });
                    }
                }
            }
            Err(e) => {
                results.push(ProfileSmokeResult {
                    profile: name.clone(),
                    encoder: "unknown".to_string(),
                    expected_hardware: false,
                    status: SmokeStatus::Failed,
                    duration_ms: profile_load_start.elapsed().as_millis(),
                    error: Some(e.to_string()),
                    stderr_tail: None,
                    note: None,
                    command: String::new(),
                    output_bitrate_kbps: None,
                    output_size_bytes: None,
                    video_codec: None,
                    audio_codec: None,
                });
            }
        }
    }

    // Copy outputs to persistent directory if requested
    if let Some(output_dir) = &opts.output_dir {
        fs::create_dir_all(output_dir)
            .context("Failed to create output directory")?;

        // Find all encoded output files in temp_dir (exclude input files)
        if let Ok(entries) = fs::read_dir(&temp_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let filename = path.file_name().unwrap();
                    // Only copy smoke test outputs (not input files)
                    if filename.to_string_lossy().starts_with("smoke_") {
                        let dest = output_dir.join(filename);
                        if let Err(e) = fs::copy(&path, &dest) {
                            eprintln!("Warning: failed to copy {} to output dir: {}", path.display(), e);
                        }
                    }
                }
            }
        }
    }

    let _ = fs::remove_dir_all(&temp_dir);

    Ok(SmokeSummary {
        ffmpeg_version,
        total_duration_ms: start.elapsed().as_millis(),
        results,
    })
}

pub fn print_pretty(summary: &SmokeSummary) {
    println!("=== ffdash smoke-test ===");
    if let Some(ver) = &summary.ffmpeg_version {
        println!("ffmpeg: {}", ver);
    } else {
        println!("ffmpeg: (version unavailable)");
    }
    println!(
        "duration: {} ms | results: {} profiles",
        summary.total_duration_ms,
        summary.results.len()
    );
    for res in &summary.results {
        println!(
            "- {} [{}] -> {}{}",
            res.profile,
            res.encoder,
            match res.status {
                SmokeStatus::Passed => "passed",
                SmokeStatus::Failed => "FAILED",
                SmokeStatus::Skipped => "skipped",
            },
            res.note
                .as_ref()
                .map(|n| format!(" ({})", n))
                .unwrap_or_default()
        );

        if let Some(err) = &res.error {
            println!("  error: {}", err);
        }
        if let Some(tail) = &res.stderr_tail {
            println!("  stderr tail: {}", tail.trim_end());
        }
        if let Some(sz) = res.output_size_bytes {
            println!("  output size: {} bytes", sz);
        }
        if let Some(br) = res.output_bitrate_kbps {
            println!("  output bitrate: {:.1} kbps", br);
        }
        if let Some(vc) = &res.video_codec {
            println!("  video codec: {}", vc);
        }
        if let Some(ac) = &res.audio_codec {
            println!("  audio codec: {}", ac);
        }
    }
}

fn load_profile(name: &str) -> Result<Profile> {
    if Profile::builtin_names().iter().any(|p| p == name) {
        return Ok(
            Profile::get_builtin(name).unwrap_or_else(|| Profile::get("vp9-good")), // fallback safety
        );
    }

    // Accept internal short names like "vp9-good" or "av1-svt"
    if let Ok(profile) = std::panic::catch_unwind(|| Profile::get(name)) {
        return Ok(profile);
    }

    if let Ok(dir) = Profile::profiles_dir() {
        if let Ok(profile) = Profile::load(&dir, name) {
            return Ok(profile);
        }
    }

    Err(anyhow!("Profile '{}' not found", name))
}

fn describe_encoder(enc: &VideoEncoder) -> String {
    enc.ffmpeg_name().to_string()
}

fn create_sample_video(temp_dir: &Path) -> Result<PathBuf> {
    let input_path = temp_dir.join("smoke_input.mp4");
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=1.5:size=1280x720:rate=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=1000:duration=1.5",
            "-shortest",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            input_path.to_str().unwrap(),
        ])
        .status()
        .context("Failed to spawn ffmpeg to create sample video")?;

    if status.success() {
        Ok(input_path)
    } else {
        Err(anyhow!(
            "ffmpeg sample generation failed with status {}",
            status
        ))
    }
}

fn run_profile(
    profile: &Profile,
    input: &Path,
    temp_dir: &Path,
    max_frames: u32,
) -> Result<ProfileSmokeResult> {
    let mut job = VideoJob::new(
        input.to_path_buf(),
        temp_dir.join("placeholder.webm"),
        profile.name.clone(),
    );
    job.overwrite = true;

    // Two-pass logging path requires its parent directories to exist.
    let passlog_dir = two_pass_log_prefix(&job).parent().map(PathBuf::from);
    if let Some(dir) = passlog_dir {
        let _ = fs::create_dir_all(dir);
    }

    let hw_cfg = profile
        .use_hardware_encoding
        .then_some(HwEncodingConfig::default());
    let encoder = hardware::select_encoder(&profile.codec, profile.use_hardware_encoding, Some(&profile.video_codec));

    // Ensure outputs land in temp dir (one file per profile)
    job.output_path = temp_dir.join(format!(
        "smoke_{}.{}",
        profile.suffix,
        profile.container.as_str()
    ));

    let mut cmds = build_ffmpeg_cmds_with_profile(&job, hw_cfg.as_ref(), Some(profile));
    let command_text = cmds
        .iter()
        .map(stringify_command)
        .collect::<Vec<_>>()
        .join("\n");

    // Keep runs short for remote hardware
    for cmd in cmds.iter_mut() {
        cmd.arg("-frames:v").arg(max_frames.to_string());
    }

    let run_start = Instant::now();
    let mut stderr_tail = None;

    for mut cmd in cmds {
        let output = cmd
            .output()
            .with_context(|| format!("Failed to execute {:?}", cmd.get_program()))?;

        if !output.status.success() {
            stderr_tail = Some(tail_utf8(&output.stderr));
            return Ok(ProfileSmokeResult {
                profile: profile.name.clone(),
                encoder: describe_encoder(&encoder),
                expected_hardware: profile.use_hardware_encoding,
                status: SmokeStatus::Failed,
                duration_ms: run_start.elapsed().as_millis(),
                error: Some(format!(
                    "ffmpeg exited with status {}",
                    output.status.code().unwrap_or(-1)
                )),
                stderr_tail,
                note: None,
                command: command_text,
                output_bitrate_kbps: None,
                output_size_bytes: None,
                video_codec: None,
                audio_codec: None,
            });
        }
    }

    let output_info = analyze_output(&job.output_path)?;

    Ok(ProfileSmokeResult {
        profile: profile.name.clone(),
        encoder: describe_encoder(&encoder),
        expected_hardware: profile.use_hardware_encoding,
        status: SmokeStatus::Passed,
        duration_ms: run_start.elapsed().as_millis(),
        error: None,
        stderr_tail,
        note: None,
        command: command_text,
        output_bitrate_kbps: output_info.bitrate_kbps,
        output_size_bytes: output_info.size_bytes,
        video_codec: output_info.video_codec,
        audio_codec: output_info.audio_codec,
    })
}

fn stringify_command(cmd: &Command) -> String {
    let mut parts = Vec::new();
    parts.push(cmd.get_program().to_string_lossy().to_string());
    parts.extend(
        cmd.get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect::<Vec<_>>(),
    );
    parts.join(" ")
}

fn tail_utf8(buf: &[u8]) -> String {
    const MAX_BYTES: usize = 1200;
    if buf.len() <= MAX_BYTES {
        return String::from_utf8_lossy(buf).to_string();
    }
    let tail = &buf[buf.len().saturating_sub(MAX_BYTES)..];
    String::from_utf8_lossy(tail).to_string()
}

fn format_command_preview(profile: &Profile, max_frames: u32, validate_only: bool) -> String {
    let mut job = VideoJob::new(
        PathBuf::from("input.mp4"),
        PathBuf::from("output.tmp"),
        profile.name.clone(),
    );
    job.overwrite = true;

    let hw_cfg = profile
        .use_hardware_encoding
        .then_some(HwEncodingConfig::default());
    let mut cmds = build_ffmpeg_cmds_with_profile(&job, hw_cfg.as_ref(), Some(profile));
    for cmd in cmds.iter_mut() {
        if !validate_only {
            cmd.arg("-frames:v").arg(max_frames.to_string());
        }
    }
    cmds.into_iter()
        .map(|cmd| stringify_command(&cmd))
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug)]
struct OutputInfo {
    size_bytes: Option<u64>,
    bitrate_kbps: Option<f64>,
    video_codec: Option<String>,
    audio_codec: Option<String>,
}

fn analyze_output(path: &Path) -> Result<OutputInfo> {
    if !path.exists() {
        return Err(anyhow!("Output file not found: {}", path.display()));
    }

    let meta = fs::metadata(path).ok();
    let size_bytes = meta.as_ref().map(|m| m.len());

    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=codec_name,bit_rate",
            "-show_entries",
            "format=bit_rate",
            "-of",
            "default=nokey=1:noprint_wrappers=1",
            path.to_str().unwrap(),
        ])
        .output()
        .context("Failed to run ffprobe on smoke output")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines();
    let video_codec = lines.next().map(|s| s.to_string());
    let stream_bitrate = lines
        .next()
        .and_then(|s| s.parse::<u64>().ok())
        .map(|b| b as f64 / 1000.0);
    let format_bitrate = lines
        .next()
        .and_then(|s| s.parse::<u64>().ok())
        .map(|b| b as f64 / 1000.0);

    let audio_codec = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=codec_name",
            "-of",
            "default=nokey=1:noprint_wrappers=1",
            path.to_str().unwrap(),
        ])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout);
                s.lines().next().map(|s| s.to_string())
            } else {
                None
            }
        });

    // Basic sanity checks
    if video_codec.is_none() {
        return Err(anyhow!("ffprobe did not report a video stream"));
    }
    if audio_codec.is_none() {
        return Err(anyhow!("ffprobe did not report an audio stream"));
    }
    if size_bytes.unwrap_or(0) == 0 {
        return Err(anyhow!("output file is empty"));
    }

    let bitrate_kbps = stream_bitrate.or(format_bitrate);

    Ok(OutputInfo {
        size_bytes,
        bitrate_kbps,
        video_codec,
        audio_codec,
    })
}
