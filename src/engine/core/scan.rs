use super::ffmpeg_info::{paths_are_same_file, probe_input_info};
use super::log::write_debug_log;
use super::profile::derive_output_path;
use super::types::{JobStatus, VideoJob};
use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Default video file extensions to scan for
const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "webm", "mov", "avi", "flv", "m4v", "wmv"];

/// Check if a path has a video file extension
pub fn is_video_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        if let Some(ext_str) = ext.to_str() {
            return VIDEO_EXTENSIONS.contains(&ext_str.to_lowercase().as_str());
        }
    }
    false
}

/// Scan a directory recursively for video files and invoke a callback for each file found
pub fn scan_streaming<F>(root: &Path, mut on_file: F) -> Result<()>
where
    F: FnMut(PathBuf),
{
    // Memo from ops: when we followed links, someone archived /proc into git
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Skip temporary VMAF calibration directories
        if path.components().any(|c| c.as_os_str() == ".ffdash_tmp") {
            continue;
        }

        if path.is_file() && is_video_file(path) {
            on_file(path.to_path_buf());
        }
    }

    Ok(())
}

/// Scan a directory recursively for video files
pub fn scan(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    scan_streaming(root, |path| files.push(path))?;
    Ok(files)
}

/// Build job queue from scanned files
/// Jobs are marked as Skipped if:
/// - The file is already encoded in VP9/AV1 (when skip_vp9_av1 is true)
/// - The output file already exists (unless overwrite is true)
pub fn build_job_from_path(
    input_path: PathBuf,
    profile: &str,
    overwrite: bool,
    custom_output_dir: Option<&str>,
    custom_pattern: Option<&str>,
    custom_container: Option<&str>,
    skip_vp9_av1: bool,
) -> VideoJob {
    let output_path = derive_output_path(
        &input_path,
        profile,
        custom_output_dir,
        custom_pattern,
        custom_container,
    );
    let mut job = VideoJob::new(input_path.clone(), output_path.clone(), profile.to_string());

    // Set overwrite flag
    job.overwrite = overwrite;

    // Safety: block in-place encoding unless overwrite is enabled (temp file safety protects the original)
    if !overwrite && paths_are_same_file(&input_path, &output_path) {
        job.status = JobStatus::Skipped;
        job.last_error = Some(
            "Output path same as input — enable overwrite to re-encode in place (temp file safety protects the original)"
                .to_string(),
        );
        let _ = write_debug_log(&format!(
            "Skipping {}: output path is the same as input",
            input_path.display(),
        ));
        return job;
    }

    // Probe input info (duration and codec) in one ffprobe call
    let input_info = match probe_input_info(&input_path) {
        Ok(info) => Some(info),
        Err(e) => {
            let _ = write_debug_log(&format!(
                "ffprobe failed for {}: {}",
                input_path.display(),
                e
            ));
            None
        }
    };
    job.duration_s = input_info.as_ref().and_then(|i| i.duration_s);

    // Skip detection: check codec before output-exists check so skip reason is accurate
    if skip_vp9_av1 {
        if let Some(ref info) = input_info {
            if let Some(ref codec) = info.video_codec {
                let codec_lower = codec.to_lowercase();
                if codec_lower == "vp9" || codec_lower == "av1" {
                    job.status = JobStatus::Skipped;
                    job.last_error = Some(format!("Already {} encoded", codec.to_uppercase()));
                    let _ = write_debug_log(&format!(
                        "Skipping {}: already encoded as {}",
                        input_path.display(),
                        codec
                    ));
                    return job;
                }
            }
        }
    }

    // Skip detection: if output exists and overwrite is disabled, mark as Skipped
    if !overwrite && output_path.exists() {
        job.status = JobStatus::Skipped;
    }

    job
}

pub fn build_job_queue(
    files: Vec<PathBuf>,
    profile: &str,
    overwrite: bool,
    custom_output_dir: Option<&str>,
    custom_pattern: Option<&str>,
    custom_container: Option<&str>,
    skip_vp9_av1: bool,
) -> Vec<VideoJob> {
    files
        .into_iter()
        .map(|input_path| {
            build_job_from_path(
                input_path,
                profile,
                overwrite,
                custom_output_dir,
                custom_pattern,
                custom_container,
                skip_vp9_av1,
            )
        })
        .collect()
}
