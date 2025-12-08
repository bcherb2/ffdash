use super::ffmpeg_info::probe_duration;
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
/// Jobs are marked as Skipped if the output file already exists (unless overwrite is true)
pub fn build_job_from_path(
    input_path: PathBuf,
    profile: &str,
    overwrite: bool,
    custom_output_dir: Option<&str>,
    custom_pattern: Option<&str>,
    custom_container: Option<&str>,
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

    // Probe duration for ETA calculation
    job.duration_s = probe_duration(&input_path).ok();

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
            )
        })
        .collect()
}
