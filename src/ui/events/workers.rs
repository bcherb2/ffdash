use super::*;

pub(super) fn handle_worker_message(
    msg: crate::engine::worker::WorkerMessage,
    state: &mut AppState,
) {
    use crate::engine::{JobStatus, worker::WorkerMessage};

    match msg {
        WorkerMessage::JobStarted { job_id } => {
            // Update job status to Running and set start time
            if let Some(job) = state.dashboard.jobs.iter_mut().find(|j| j.id == job_id) {
                job.status = JobStatus::Running;
                job.started_at = Some(std::time::Instant::now());
            }
            // Sync to enc_state
            if let Some(ref mut enc_state) = state.enc_state {
                if let Some(job) = enc_state.jobs.iter_mut().find(|j| j.id == job_id) {
                    job.status = JobStatus::Running;
                    job.started_at = Some(std::time::Instant::now());
                }
            }
        }
        WorkerMessage::ProgressUpdate {
            job_id,
            progress_pct,
            out_time_s,
            fps,
            speed,
            bitrate_kbps,
            size_bytes,
        } => {
            // Update job progress in dashboard
            if let Some(job) = state.dashboard.jobs.iter_mut().find(|j| j.id == job_id) {
                job.progress_pct = progress_pct;
                job.out_time_s = out_time_s;
                job.fps = fps;

                // Update smoothed speed using EWMA with debouncing (alpha = 0.1, update every 2s)
                if let Some(new_speed) = speed {
                    const ALPHA: f64 = 0.1; // Very smooth - 10% new data, 90% historical
                    const SPEED_UPDATE_INTERVAL: std::time::Duration =
                        std::time::Duration::from_secs(2);

                    let should_update = job
                        .last_speed_update
                        .map_or(true, |last| last.elapsed() >= SPEED_UPDATE_INTERVAL);

                    if should_update {
                        job.smoothed_speed = Some(match job.smoothed_speed {
                            Some(prev) => ALPHA * new_speed + (1.0 - ALPHA) * prev,
                            None => new_speed, // First sample
                        });
                        job.last_speed_update = Some(std::time::Instant::now());
                    }
                }
                job.speed = speed; // Keep raw speed for display

                job.bitrate_kbps = bitrate_kbps;
                job.size_bytes = size_bytes;
            }
            // Sync to enc_state
            if let Some(ref mut enc_state) = state.enc_state {
                if let Some(job) = enc_state.jobs.iter_mut().find(|j| j.id == job_id) {
                    job.progress_pct = progress_pct;
                    job.out_time_s = out_time_s;
                    job.fps = fps;

                    // Update smoothed speed using EWMA with debouncing (alpha = 0.1, update every 2s)
                    if let Some(new_speed) = speed {
                        const ALPHA: f64 = 0.1; // Very smooth - 10% new data, 90% historical
                        const SPEED_UPDATE_INTERVAL: std::time::Duration =
                            std::time::Duration::from_secs(2);

                        let should_update = job
                            .last_speed_update
                            .map_or(true, |last| last.elapsed() >= SPEED_UPDATE_INTERVAL);

                        if should_update {
                            job.smoothed_speed = Some(match job.smoothed_speed {
                                Some(prev) => ALPHA * new_speed + (1.0 - ALPHA) * prev,
                                None => new_speed, // First sample
                            });
                            job.last_speed_update = Some(std::time::Instant::now());
                        }
                    }
                    job.speed = speed; // Keep raw speed for display

                    job.bitrate_kbps = bitrate_kbps;
                    job.size_bytes = size_bytes;
                }
            }
        }
        WorkerMessage::JobCompleted { job_id } => {
            // Mark job as Done
            if let Some(job) = state.dashboard.jobs.iter_mut().find(|j| j.id == job_id) {
                job.status = JobStatus::Done;
                job.progress_pct = 100.0;

                // Update stats
                if let Ok(input_size) = std::fs::metadata(&job.input_path).map(|m| m.len()) {
                    if let Ok(output_size) = std::fs::metadata(&job.output_path).map(|m| m.len()) {
                        let encode_time = job
                            .started_at
                            .map(|t| t.elapsed().as_secs_f64())
                            .unwrap_or(0.0);

                        // Update session stats
                        state.stats.session.jobs_done += 1;
                        state.stats.session.input_bytes += input_size;
                        state.stats.session.output_bytes += output_size;
                        state.stats.session.encode_time_secs += encode_time;

                        // Update lifetime stats
                        state.stats.lifetime.total_input_bytes += input_size;
                        state.stats.lifetime.total_output_bytes += output_size;
                        state.stats.lifetime.total_encode_time_secs += encode_time;
                        state.stats.lifetime.total_jobs_completed += 1;
                        state.stats.lifetime.last_updated = Some(chrono::Utc::now().to_rfc3339());

                        // Save lifetime stats to disk
                        let _ = state.stats.lifetime.save();
                    }
                }
            }
            if let Some(ref mut enc_state) = state.enc_state {
                if let Some(job) = enc_state.jobs.iter_mut().find(|j| j.id == job_id) {
                    job.status = JobStatus::Done;
                    job.progress_pct = 100.0;
                }
                // Save .enc_queue status
                if let Some(ref root) = state.root_path {
                    let _ = enc_state.save_queue_status(root);
                }
            }
            // Don't spawn next job here - wait for WorkerIdle message to avoid race condition
        }
        WorkerMessage::JobFailed { job_id, error } => {
            // Mark job as Failed
            if let Some(job) = state.dashboard.jobs.iter_mut().find(|j| j.id == job_id) {
                job.status = JobStatus::Failed;
                job.last_error = Some(error.clone());

                // Update stats
                state.stats.session.jobs_failed += 1;
                state.stats.lifetime.total_jobs_failed += 1;
                state.stats.lifetime.last_updated = Some(chrono::Utc::now().to_rfc3339());
                let _ = state.stats.lifetime.save();
            }
            if let Some(ref mut enc_state) = state.enc_state {
                if let Some(job) = enc_state.jobs.iter_mut().find(|j| j.id == job_id) {
                    job.status = JobStatus::Failed;
                    job.last_error = Some(error);
                }
            }
            // Don't spawn next job here - wait for WorkerIdle message to avoid race condition
        }
        WorkerMessage::WorkerIdle { worker_id: _ } => {
            // Worker is idle and ready for more work
            spawn_next_job(state);
        }
    }
}

pub(super) fn spawn_next_job(state: &mut AppState) {
    use crate::engine::JobStatus;

    // Check if we can spawn more workers
    if let Some(pool) = &state.worker_pool {
        if !pool.can_spawn() {
            return; // Already at max workers
        }

        // Find next encodable pending job
        if let Some(ref mut enc_state) = state.enc_state {
            // Loop through all pending jobs to find one that can actually be encoded
            let encodable_job_idx = loop {
                let next_job = enc_state
                    .jobs
                    .iter()
                    .enumerate()
                    .find(|(_, j)| j.status == JobStatus::Pending);

                match next_job {
                    Some((idx, _)) => {
                        let mut job = enc_state.jobs[idx].clone();

                        // Check if output exists and overwrite is disabled
                        // If so, skip this job and try the next one
                        if job.output_path.exists() && !job.overwrite {
                            job.status = JobStatus::Skipped;
                            job.last_error =
                                Some("Output exists and overwrite is disabled".to_string());
                            enc_state.jobs[idx] = job.clone();
                            state.dashboard.jobs[idx] = job;
                            // Continue loop to find next encodable job
                            continue;
                        }

                        // Found an encodable job
                        break Some(idx);
                    }
                    None => {
                        // No more pending jobs
                        break None;
                    }
                }
            };

            // Spawn worker with the encodable job
            if let Some(idx) = encodable_job_idx {
                let job = enc_state.jobs[idx].clone();

                // Build hardware encoding config if enabled AND available
                let hw_config = if state.config.use_hardware_encoding
                    && state.config.hw_encoding_available == Some(true)
                {
                    Some(crate::engine::HwEncodingConfig {
                        rc_mode: state.config.vaapi_rc_mode.parse().unwrap_or(4),
                        global_quality: state.config.qsv_global_quality,
                        b_frames: state.config.vaapi_b_frames.parse().unwrap_or(0),
                        loop_filter_level: state
                            .config
                            .vaapi_loop_filter_level
                            .parse()
                            .unwrap_or(16),
                        loop_filter_sharpness: state
                            .config
                            .vaapi_loop_filter_sharpness
                            .parse()
                            .unwrap_or(4),
                        compression_level: state
                            .config
                            .vaapi_compression_level
                            .parse()
                            .unwrap_or(4),
                    })
                } else {
                    // Fall back to software encoding if hardware unavailable
                    if state.config.use_hardware_encoding
                        && state.config.hw_encoding_available != Some(true)
                    {
                        state.config.hw_availability_message =
                            Some("Hardware unavailable, using software encoding".to_string());
                    }
                    None
                };

                // Get profile from enc_state if available
                let profile = state
                    .enc_state
                    .as_ref()
                    .and_then(|es| es.profile_config.clone());

                // Spawn worker for this job
                if pool
                    .spawn_worker_with_profile(idx, job, hw_config, profile)
                    .is_ok()
                {
                    // Job will be marked as Running when JobStarted message arrives
                }
            }
        }
    }
}

/// Rescan directory and refresh job queue
pub(super) fn rescan_directory(
    state: &mut AppState,
    directory: std::path::PathBuf,
) -> Result<(), String> {
    use crate::engine;

    // Clear existing jobs first
    state.dashboard.jobs.clear();
    state.enc_state = None;

    // Scan for video files
    let files = engine::scan(&directory).map_err(|e| format!("Failed to scan directory: {}", e))?;

    if files.is_empty() {
        // Clear state file if no videos found
        let state_path = directory.join(".enc_state");
        if state_path.exists() {
            let _ = std::fs::remove_file(state_path);
        }
        return Err("No video files found in directory".to_string());
    }

    // Get profile name from config
    let profile_name = state
        .config
        .current_profile_name
        .clone()
        .unwrap_or_else(|| "YouTube 4K".to_string());

    // Get custom pattern and container from config
    let custom_pattern = state.config.filename_pattern.as_deref();
    let container_options = ["webm", "mp4", "mkv", "avi"];
    let container_idx = state
        .config
        .container_dropdown_state
        .selected()
        .unwrap_or(0);
    let custom_container = Some(container_options[container_idx]);

    // Get output directory from config (None means use input file's directory)
    let custom_output_dir = if state.config.output_dir.is_empty() {
        None
    } else {
        Some(state.config.output_dir.as_str())
    };

    // Build fresh job queue (respect overwrite setting)
    let jobs = engine::build_job_queue(
        files,
        &profile_name,
        state.config.overwrite,
        custom_output_dir,
        custom_pattern,
        custom_container,
    );

    // Create new enc_state (don't merge with old one)
    let enc_state = engine::EncState::new(jobs.clone(), profile_name, directory.clone());

    // Save new state
    enc_state
        .save(&directory)
        .map_err(|e| format!("Failed to save .enc_state: {}", e))?;

    // Update app state
    state.dashboard.jobs = jobs;
    state.enc_state = Some(enc_state);
    state.root_path = Some(directory);

    // Reset table selection to first job
    if !state.dashboard.jobs.is_empty() {
        state.dashboard.table_state.select(Some(0));
    } else {
        state.dashboard.table_state.select(None);
    }

    Ok(())
}

/// Start encoding from already-loaded jobs (used for autostart)
pub(super) fn start_encoding_from_loaded_jobs(state: &mut AppState) -> Result<(), String> {
    use crate::engine::worker::WorkerPool;

    // Ensure we have a root path and jobs loaded
    let root_path = state
        .root_path
        .clone()
        .ok_or_else(|| "No root path set".to_string())?;

    if state.dashboard.jobs.is_empty() {
        return Err("No jobs loaded to encode".to_string());
    }

    // Get profile name from config
    let profile_name = state
        .config
        .current_profile_name
        .clone()
        .unwrap_or_else(|| "YouTube 4K".to_string());

    // Create Profile from current config to preserve user's custom settings (like max FPS)
    let profile = crate::engine::Profile::from_config(profile_name.clone(), &state.config);

    // Create enc_state from already-loaded jobs
    let enc_state = crate::engine::EncState::new_with_profile(
        state.dashboard.jobs.clone(),
        profile_name,
        root_path.clone(),
        Some(profile),
    );

    // Save initial state
    enc_state
        .save(&root_path)
        .map_err(|e| format!("Failed to save .enc_state: {}", e))?;

    // Initialize worker pool
    let max_workers = state.config.max_workers as usize;
    let pool = Arc::new(WorkerPool::new(max_workers));

    // Store state
    state.enc_state = Some(enc_state);
    state.worker_pool = Some(pool.clone());

    // Spawn initial workers (up to max_workers)
    for _ in 0..max_workers {
        spawn_next_job(state);
    }

    Ok(())
}

/// Start encoding: scan directory, build job queue, initialize workers
pub(super) fn start_encoding(
    state: &mut AppState,
    directory: std::path::PathBuf,
) -> Result<(), String> {
    use crate::engine::{self, worker::WorkerPool};

    // Get profile name from config (use default if none selected)
    let profile_name = state
        .config
        .current_profile_name
        .clone()
        .unwrap_or_else(|| "YouTube 4K".to_string());

    // Create Profile from current config to preserve user's custom settings (like max FPS)
    let profile = engine::Profile::from_config(profile_name.clone(), &state.config);

    // Check if jobs are already loaded in dashboard (user may have marked some as Skipped)
    let jobs = if !state.dashboard.jobs.is_empty() {
        // Use existing jobs to preserve user's skip selections
        // BUT update overwrite flag to match current config
        state
            .dashboard
            .jobs
            .iter()
            .map(|job| {
                let mut updated_job = job.clone();
                updated_job.overwrite = state.config.overwrite;
                updated_job
            })
            .collect()
    } else {
        // No jobs loaded yet, scan directory and build job queue
        let files =
            engine::scan(&directory).map_err(|e| format!("Failed to scan directory: {}", e))?;

        if files.is_empty() {
            return Err("No video files found in directory".to_string());
        }

        // Get custom pattern and container from config
        let custom_pattern = state.config.filename_pattern.as_deref();
        let container_options = ["webm", "mp4", "mkv", "avi"];
        let container_idx = state
            .config
            .container_dropdown_state
            .selected()
            .unwrap_or(0);
        let custom_container = Some(container_options[container_idx]);

        // Get output directory from config (None means use input file's directory)
        let custom_output_dir = if state.config.output_dir.is_empty() {
            None
        } else {
            Some(state.config.output_dir.as_str())
        };

        // Build job queue (respect overwrite setting)
        engine::build_job_queue(
            files,
            &profile_name,
            state.config.overwrite,
            custom_output_dir,
            custom_pattern,
            custom_container,
        )
    };

    // Create enc_state with jobs (preserving any skip status)
    let enc_state = engine::EncState::new_with_profile(
        jobs.clone(),
        profile_name,
        directory.clone(),
        Some(profile),
    );

    // Save initial state
    enc_state
        .save(&directory)
        .map_err(|e| format!("Failed to save .enc_state: {}", e))?;

    // Copy jobs to dashboard for display
    state.dashboard.jobs = enc_state.jobs.clone();

    // Initialize worker pool
    let max_workers = state.config.max_workers as usize;
    let pool = Arc::new(WorkerPool::new(max_workers));

    // Store state
    state.enc_state = Some(enc_state);
    state.root_path = Some(directory);
    state.worker_pool = Some(pool.clone());

    // Spawn initial workers (up to max_workers)
    for _ in 0..max_workers {
        spawn_next_job(state);
    }

    Ok(())
}

pub(super) fn update_metrics(state: &mut AppState) {
    // Refresh system information
    state.dashboard.system.refresh_cpu();
    state.dashboard.system.refresh_memory();

    // Get global CPU usage (0-100)
    let cpu_usage = state.dashboard.system.global_cpu_info().cpu_usage() as u64;

    // Get memory usage percentage (0-100)
    let total_mem = state.dashboard.system.total_memory();
    let used_mem = state.dashboard.system.used_memory();
    let mem_usage = if total_mem > 0 {
        ((used_mem as f64 / total_mem as f64) * 100.0) as u64
    } else {
        0
    };

    // Add to ring buffers (240 points = 60 seconds at 250ms sampling)
    if state.dashboard.cpu_data.len() >= 240 {
        state.dashboard.cpu_data.pop_front();
    }
    state.dashboard.cpu_data.push_back(cpu_usage);

    if state.dashboard.mem_data.len() >= 240 {
        state.dashboard.mem_data.pop_front();
    }
    state.dashboard.mem_data.push_back(mem_usage);

    // Collect GPU stats if hardware encoding is enabled
    if state.config.use_hardware_encoding && state.dashboard.gpu_available {
        if let Some(gpu_stats) = crate::engine::hardware::get_gpu_stats() {
            // Add GPU utilization to ring buffer
            if state.dashboard.gpu_data.len() >= 240 {
                state.dashboard.gpu_data.pop_front();
            }
            state
                .dashboard
                .gpu_data
                .push_back(gpu_stats.utilization as u64);

            // Add GPU memory usage to ring buffer
            if state.dashboard.gpu_mem_data.len() >= 240 {
                state.dashboard.gpu_mem_data.pop_front();
            }
            state
                .dashboard
                .gpu_mem_data
                .push_back(gpu_stats.memory_percent as u64);
        }
    }
}

// Function to print job completion to scrollback
// In real implementation, this would be called when a job finishes
#[allow(dead_code)]
fn print_completion<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    filename: &str,
    in_size: &str,
    out_size: &str,
    speed: f64,
    duration: &str,
) -> io::Result<()> {
    use ratatui::style::{Color, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Paragraph, Widget};

    terminal.insert_before(1, |buf| {
        let line = Line::from(vec![
            Span::styled("✓ ", Style::default().fg(Color::Green)),
            Span::raw("Completed: "),
            Span::styled(filename, Style::default().fg(Color::Cyan)),
            Span::raw(format!(
                " ({} → {}, {:.2}x avg, {})",
                in_size, out_size, speed, duration
            )),
        ]);
        Paragraph::new(line).render(buf.area, buf);
    })?;

    Ok(())
}
