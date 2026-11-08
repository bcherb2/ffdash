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
            vmaf_result,
            vmaf_target,
            encoder,
            status,
        } => {
            // Update job progress in dashboard
            if let Some(job) = state.dashboard.jobs.iter_mut().find(|j| j.id == job_id) {
                job.status = status.clone();
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
                        .is_none_or(|last| last.elapsed() >= SPEED_UPDATE_INTERVAL);

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
                job.vmaf_result = vmaf_result;
                job.vmaf_target = vmaf_target;
                job.encoder = encoder.clone();
            }
            // Sync to enc_state
            if let Some(ref mut enc_state) = state.enc_state {
                if let Some(job) = enc_state.jobs.iter_mut().find(|j| j.id == job_id) {
                    job.status = status;
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
                            .is_none_or(|last| last.elapsed() >= SPEED_UPDATE_INTERVAL);

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
                    job.vmaf_result = vmaf_result;
                    job.vmaf_target = vmaf_target;
                    job.encoder = encoder;
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
                // Get profile from enc_state if available
                let profile = state
                    .enc_state
                    .as_ref()
                    .and_then(|es| es.profile_config.clone());

                // Select the Encoder when hardware is requested AND available; otherwise
                // the engine selects for the profile (software unless it says otherwise).
                let plan = if state.config.use_hardware_encoding
                    && state.config.hw_encoding_available == Some(true)
                {
                    let plan_profile = profile
                        .clone()
                        .unwrap_or_else(|| crate::engine::Profile::get(&job.profile));
                    Some(crate::engine::EncodePlan::for_profile(
                        &plan_profile,
                        true,
                        crate::engine::required_bit_depth(&plan_profile, &job.input_path),
                    ))
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

                // Spawn worker for this job
                if pool
                    .spawn_worker_with_profile(idx, job, plan, profile)
                    .is_ok()
                {
                    // Job will be marked as Running when JobStarted message arrives
                }
            }
        }
    }
}

/// Rescan directory in background (non-blocking)
pub(super) fn rescan_directory(
    state: &mut AppState,
    root: std::path::PathBuf,
    event_tx: &std::sync::mpsc::Sender<super::UiEvent>,
) {
    if state.scan_in_progress {
        return;
    }

    state.scan_in_progress = true;

    let scan_config = super::build_scan_config(state, &root);

    // Initialize empty enc_state so skip toggles work while jobs stream in
    state.enc_state = Some(crate::engine::EncState::new(
        Vec::new(),
        scan_config.profile.clone(),
        root.clone(),
    ));

    // Send RescanStarting on the calling thread (before spawn) to avoid race
    // where ScanJob events arrive before the clear signal.
    let _ = event_tx.send(super::UiEvent::RescanStarting);

    super::spawn_scan_thread(scan_config, event_tx.clone());
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
    let max_workers = (state.config.max_workers as usize).clamp(1, 16);
    let pool = Rc::new(WorkerPool::new(max_workers));

    // Store state
    state.enc_state = Some(enc_state);
    state.worker_pool = Some(pool.clone());

    // Spawn initial workers (up to max_workers)
    for _ in 0..max_workers {
        spawn_next_job(state);
    }

    Ok(())
}

/// Start encoding with a background scan (non-blocking)
pub(super) fn start_encoding_with_scan(
    state: &mut AppState,
    directory: std::path::PathBuf,
    event_tx: &std::sync::mpsc::Sender<super::UiEvent>,
) {
    if state.scan_in_progress {
        return;
    }

    state.scan_in_progress = true;
    state.pending_autostart = true;

    let scan_config = super::build_scan_config(state, &directory);

    // Initialize enc_state with profile so encoding can start when scan finishes
    let profile_name = scan_config.profile.clone();
    let profile = crate::engine::Profile::from_config(profile_name.clone(), &state.config);
    state.enc_state = Some(crate::engine::EncState::new_with_profile(
        Vec::new(),
        profile_name,
        directory.clone(),
        Some(profile),
    ));

    super::spawn_scan_thread(scan_config, event_tx.clone());
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
        if let Some(gpu_stats) =
            crate::engine::hardware::get_gpu_stats_for_vendor(state.dashboard.gpu_vendor)
        {
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
