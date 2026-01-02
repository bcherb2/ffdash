use super::*;

impl Dashboard {
    pub(super) fn format_uptime(seconds: u64) -> String {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        let secs = seconds % 60;
        format!("{:02}:{:02}:{:02}", hours, minutes, secs)
    }

    pub(super) fn render_system_metrics(
        frame: &mut Frame,
        area: Rect,
        state: &DashboardState,
        use_hw: bool,
    ) {
        // Calculate available width for data (subtract 2 for borders)
        let available_width = area.width.saturating_sub(2) as usize;

        // Check if we should render GPU graphs
        let show_gpu = use_hw && state.gpu_available;

        if show_gpu {
            // GPU mode: show GPU utilization (top, yellow) + GPU memory (bottom, magenta)
            let data_len = state.gpu_data.len();
            let points_to_show = available_width.min(data_len);
            let start_index = data_len.saturating_sub(points_to_show);

            // Apply floor to prevent graph disappearing at 0%, while keeping stats accurate
            // 5% minimum required for at least 1 dot to show in braille rendering
            let gpu_data: Vec<f64> = state
                .gpu_data
                .iter()
                .skip(start_index)
                .map(|&val| (val as f64 / 100.0).max(0.05))
                .collect();

            let (secondary_data, secondary_label, secondary_color) =
                if !state.gpu_mem_data.is_empty() {
                    let gpu_mem: Vec<f64> = state
                        .gpu_mem_data
                        .iter()
                        .skip(state.gpu_mem_data.len().saturating_sub(points_to_show))
                        .map(|&val| (val as f64 / 100.0).max(0.05))
                        .collect();
                    (gpu_mem, "VRAM", Color::Magenta)
                } else {
                    // Fallback to CPU if GPU memory unavailable
                    let cpu_data: Vec<f64> = state
                        .cpu_data
                        .iter()
                        .skip(start_index)
                        .map(|&val| (val as f64 / 100.0).max(0.05))
                        .collect();
                    (cpu_data, "CPU", Color::Cyan)
                };

            let (gpu_current, gpu_avg, gpu_max) = Self::calculate_stats(&state.gpu_data);
            let (secondary_current, secondary_avg, secondary_max) =
                if !state.gpu_mem_data.is_empty() {
                    Self::calculate_stats(&state.gpu_mem_data)
                } else {
                    Self::calculate_stats(&state.cpu_data)
                };

            let title = format!(
                "GPU: {}% (Avg: {}%, Max: {}%) | {}: {}% (Avg: {}%, Max: {}%)",
                gpu_current,
                gpu_avg,
                gpu_max,
                secondary_label,
                secondary_current,
                secondary_avg,
                secondary_max
            );

            let widget = WaveformWidget::new(&gpu_data, &secondary_data)
                .mode(WaveformMode::HighResBraille)
                .top_style(Style::default().fg(Color::Yellow))
                .bottom_style(Style::default().fg(secondary_color))
                .fade_effect(true)
                .gradient_effect(true)
                .top_max(1.0)
                .bottom_max(1.0)
                .block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::White)),
                );

            frame.render_widget(widget, area);
        } else {
            // Software mode: existing CPU (cyan) + RAM (green) display
            let data_len = state.cpu_data.len();
            let points_to_show = available_width.min(data_len);
            let start_index = data_len.saturating_sub(points_to_show);

            // Convert CPU data from VecDeque<u64> to Vec<f64> normalized to 0.0-1.0
            // Apply floor to prevent graph disappearing at 0%, while keeping stats accurate
            // 5% minimum required for at least 1 dot to show in braille rendering
            let cpu_data: Vec<f64> = state
                .cpu_data
                .iter()
                .skip(start_index)
                .map(|&val| (val as f64 / 100.0).max(0.05))
                .collect();

            // Convert Memory data from VecDeque<u64> to Vec<f64> normalized to 0.0-1.0
            let mem_data: Vec<f64> = state
                .mem_data
                .iter()
                .skip(start_index)
                .map(|&val| (val as f64 / 100.0).max(0.05))
                .collect();

            // Calculate statistics
            let (cpu_current, cpu_avg, cpu_max) = Self::calculate_stats(&state.cpu_data);
            let (mem_current, mem_avg, mem_max) = Self::calculate_stats(&state.mem_data);

            // Create title with statistics
            let title = format!(
                "CPU: {}% (Avg: {}%, Max: {}%) | RAM: {}% (Avg: {}%, Max: {}%)",
                cpu_current, cpu_avg, cpu_max, mem_current, mem_avg, mem_max
            );

            // Create waveform widget with CPU on top (cyan), Memory on bottom (green)
            let widget = WaveformWidget::new(&cpu_data, &mem_data)
                .mode(WaveformMode::HighResBraille)
                .top_style(Style::default().fg(Color::Cyan))
                .bottom_style(Style::default().fg(Color::Green))
                .fade_effect(true)
                .gradient_effect(true)
                .top_max(1.0)
                .bottom_max(1.0)
                .block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::White)),
                );

            frame.render_widget(widget, area);
        }
    }

    fn calculate_stats(data: &VecDeque<u64>) -> (u64, u64, u64) {
        if data.is_empty() {
            return (0, 0, 0);
        }

        let current = *data.back().unwrap_or(&0);
        let sum: u64 = data.iter().sum();
        let avg = sum / data.len() as u64;
        let max = *data.iter().max().unwrap_or(&0);

        (current, avg, max)
    }

    fn format_duration(seconds: u64) -> String {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;

        if hours > 0 {
            format!("{}h {:02}m", hours, minutes)
        } else if minutes > 0 {
            format!("{}m", minutes)
        } else {
            format!("{}s", seconds)
        }
    }

    pub(super) fn render_queue_overall(
        frame: &mut Frame,
        area: Rect,
        state: &DashboardState,
        profile_name: Option<&str>,
        max_workers: u32,
    ) {
        use crate::engine::JobStatus;

        let title = if let Some(profile) = profile_name {
            format!("Queue Overview — Profile: {}", profile)
        } else {
            "Queue Overview — Profile: Custom".to_string()
        };

        let block = Block::default().borders(Borders::ALL).title(title);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Count jobs by status
        let total = state.jobs.len();
        let completed = state
            .jobs
            .iter()
            .filter(|j| j.status == JobStatus::Done)
            .count();
        let analyzing = state
            .jobs
            .iter()
            .filter(|j| j.status == JobStatus::Calibrating)
            .count();
        let running = state
            .jobs
            .iter()
            .filter(|j| j.status == JobStatus::Running)
            .count();
        let failed = state
            .jobs
            .iter()
            .filter(|j| j.status == JobStatus::Failed)
            .count();
        let pending = state
            .jobs
            .iter()
            .filter(|j| j.status == JobStatus::Pending)
            .count();
        let skipped = state
            .jobs
            .iter()
            .filter(|j| j.status == JobStatus::Skipped)
            .count();

        // Stats line (show analyzing and skipped counts if any jobs are in those states)
        let mut stats_spans = vec![
            Span::raw("Files: "),
            Span::styled(format!("{}", total), Style::default().bold()),
            Span::raw(" total • Completed: "),
            Span::styled(format!("{}", completed), Style::default().fg(Color::Green)),
        ];

        if analyzing > 0 {
            stats_spans.extend(vec![
                Span::raw(" • Calibrating: "),
                Span::styled(format!("{}", analyzing), Style::default().fg(Color::Cyan)),
            ]);
        }

        stats_spans.extend(vec![
            Span::raw(" • Running: "),
            Span::styled(format!("{}", running), Style::default().fg(Color::Yellow)),
            Span::raw(" • Pending: "),
            Span::styled(format!("{}", pending), Style::default().fg(Color::DarkGray)),
        ]);

        if skipped > 0 {
            stats_spans.extend(vec![
                Span::raw(" • Skipped: "),
                Span::styled(format!("{}", skipped), Style::default().fg(Color::Blue)),
            ]);
        }

        stats_spans.extend(vec![
            Span::raw(" • Failed: "),
            Span::styled(format!("{}", failed), Style::default().fg(Color::Red)),
        ]);

        let stats_text = Line::from(stats_spans);

        frame.render_widget(
            Paragraph::new(stats_text),
            Rect {
                x: inner.x,
                y: inner.y,
                width: inner.width,
                height: 1,
            },
        );

        // Calculate overall progress (exclude skipped jobs from denominator)
        let active_total = total - skipped;
        let progress_percent = if active_total > 0 {
            ((completed as f64 / active_total as f64) * 100.0) as u16
        } else {
            0
        };

        // Queue progress bar and time estimate
        let progress_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(Rect {
                x: inner.x,
                y: inner.y + 1,
                width: inner.width,
                height: 1,
            });

        let queue_progress = Gauge::default()
            .percent(progress_percent)
            .label(format!("{}%", progress_percent))
            .gauge_style(Style::default().fg(Color::Blue).bg(Color::Black))
            .use_unicode(true);

        frame.render_widget(queue_progress, progress_chunks[0]);

        // Calculate estimated time remaining from running jobs
        let eta_text = Self::calculate_queue_eta(state, max_workers);
        let time_text = Paragraph::new(eta_text)
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center);

        frame.render_widget(time_text, progress_chunks[1]);
    }

    fn calculate_time_weighted_speed(job: &crate::engine::VideoJob) -> Option<f64> {
        if let Some(started) = job.started_at {
            let elapsed = started.elapsed().as_secs_f64();
            if elapsed > 0.0 && job.out_time_s > 0.0 {
                return Some(job.out_time_s / elapsed);
            }
        }
        None
    }

    fn calculate_avg_running_speed(state: &DashboardState) -> f64 {
        use crate::engine::JobStatus;

        let running_speeds: Vec<f64> = state
            .jobs
            .iter()
            .filter(|j| j.status == JobStatus::Running)
            .filter_map(|j| j.smoothed_speed.or(j.speed))
            .collect();

        if running_speeds.is_empty() {
            1.0 // Fallback when no running jobs
        } else {
            running_speeds.iter().sum::<f64>() / running_speeds.len() as f64
        }
    }

    fn calculate_queue_eta(state: &DashboardState, max_workers: u32) -> String {
        use crate::engine::JobStatus;

        // Sum up remaining time from all running and pending jobs
        let mut total_seconds = 0.0;

        // Calculate average running speed for pending job estimation
        let avg_speed = Self::calculate_avg_running_speed(state);

        for job in &state.jobs {
            match job.status {
                JobStatus::Calibrating | JobStatus::Running => {
                    // Calculate remaining time for running/analyzing job
                    // Use time-weighted speed (most accurate) > smoothed > raw
                    let effective_speed = Self::calculate_time_weighted_speed(job)
                        .or(job.smoothed_speed)
                        .or(job.speed);

                    if let (Some(duration), Some(speed)) = (job.duration_s, effective_speed) {
                        if speed > 0.0 {
                            let remaining = duration - job.out_time_s;
                            total_seconds += remaining / speed;
                        }
                    } else if job.status == JobStatus::Calibrating {
                        // Calibrating jobs don't have speed yet, estimate small overhead
                        if let Some(duration) = job.duration_s {
                            total_seconds += duration * 0.1; // ~10% overhead estimate
                        }
                    }
                }
                JobStatus::Pending => {
                    // Estimate based on duration and average running speed
                    if let Some(duration) = job.duration_s {
                        total_seconds += duration / avg_speed;
                    }
                }
                _ => {}
            }
        }

        if total_seconds > 0.0 {
            // Count running/analyzing and pending jobs to estimate parallelism
            let active_count = state
                .jobs
                .iter()
                .filter(|j| j.status == JobStatus::Running || j.status == JobStatus::Calibrating)
                .count();
            let pending_count = state
                .jobs
                .iter()
                .filter(|j| j.status == JobStatus::Pending)
                .count();

            // Effective workers is the minimum of:
            // - max_workers (hardware limit)
            // - active + pending jobs (actual work available)
            // - at least 1 (avoid division by zero)
            let work_available = (active_count + pending_count).max(1);
            let effective_workers = (max_workers as usize).min(work_available).max(1) as f64;

            Self::format_duration((total_seconds / effective_workers) as u64)
        } else {
            "—".to_string()
        }
    }

    pub(super) fn render_active_jobs(
        frame: &mut Frame,
        area: Rect,
        state: &mut DashboardState,
        auto_vmaf_enabled: bool,
    ) {
        use crate::engine::JobStatus;

        let block = Block::default().borders(Borders::ALL).title("Active Jobs");

        let inner = block.inner(area);
        let rows_visible = inner
            .height
            .saturating_sub(2) // header plus margin
            .max(1) as usize;

        // Store areas for mouse handling
        state.table_area = Some(area);
        state.table_inner_area = Some(inner);

        frame.render_widget(block, area);

        // Build header columns based on auto_vmaf_enabled
        let mut header_cells = vec![
            "#", "STATUS", "SOURCE", "IN SIZE", "OUT SIZE", "SPEED", "PROGRESS", "ETA",
        ];
        if auto_vmaf_enabled {
            header_cells.push("VMAF");
        }

        let header = Row::new(header_cells)
            .style(Style::default().add_modifier(Modifier::BOLD))
            .bottom_margin(1);

        let job_count = state.jobs.len();
        if job_count == 0 {
            let mut empty_widths = vec![
                Constraint::Length(3),  // #
                Constraint::Length(12), // STATUS
                Constraint::Min(20),    // SOURCE
                Constraint::Length(10), // IN SIZE
                Constraint::Length(10), // OUT SIZE
                Constraint::Length(8),  // SPEED
                Constraint::Length(25), // PROGRESS
                Constraint::Length(10), // ETA
            ];
            if auto_vmaf_enabled {
                empty_widths.push(Constraint::Length(10)); // VMAF
            }

            let table = Table::new(Vec::<Row>::new(), empty_widths)
                .header(header)
                .column_spacing(2)
                .row_highlight_style(Style::default().reversed())
                .highlight_symbol(">> ");

            let mut render_state = state.table_state.clone();
            frame.render_stateful_widget(table, inner, &mut render_state);
            return;
        }

        // Keep selection valid and clamp offset so selection stays in view
        let mut selected = state.table_state.selected().unwrap_or(0);
        if selected >= job_count {
            selected = job_count - 1;
            state.table_state.select(Some(selected));
        }

        let mut offset = state.table_state.offset().min(selected);
        if selected < offset {
            offset = selected;
        } else if selected >= offset + rows_visible {
            offset = selected + 1 - rows_visible;
        }
        *state.table_state.offset_mut() = offset;

        let end = (offset + rows_visible).min(job_count);

        // Pre-calculate ETAs for the visible slice
        let mut etas: Vec<String> = Vec::with_capacity(end.saturating_sub(offset));

        // First, calculate raw ETAs for all jobs (immutable borrow)
        let raw_etas: Vec<Option<u64>> = (offset..end)
            .map(|idx| Self::calculate_raw_job_eta(&state.jobs[idx], &state.jobs))
            .collect();

        // Then, apply hysteresis and update displayed values (mutable borrow)
        for (visible_idx, raw_eta) in raw_etas.iter().enumerate() {
            let i = offset + visible_idx;
            let eta = if let Some(new_eta) = raw_eta {
                let job = &mut state.jobs[i];
                let should_update = match job.displayed_eta_seconds {
                    Some(old_eta) => {
                        let diff = new_eta.abs_diff(old_eta);
                        diff > 2 || (diff as f64 / old_eta.max(1) as f64) > 0.05
                    }
                    None => true,
                };

                if should_update {
                    job.displayed_eta_seconds = Some(*new_eta);
                }

                Self::format_duration(job.displayed_eta_seconds.unwrap_or(*new_eta))
            } else {
                "—".to_string()
            };
            etas.push(eta);
        }

        let rows: Vec<Row> = (offset..end)
            .map(|idx| {
                let job = &state.jobs[idx];
                let eta = &etas[idx - offset];

                // Get file name from path
                let filename = job
                    .input_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");

                // Format input size (if we can get it from metadata)
                let in_size = if let Ok(metadata) = std::fs::metadata(&job.input_path) {
                    Self::format_size(metadata.len())
                } else {
                    "—".to_string()
                };

                // Format output size
                let out_size = job
                    .size_bytes
                    .map(Self::format_size)
                    .unwrap_or_else(|| "—".to_string());

                // Format speed
                let speed = job
                    .speed
                    .map(|s| format!("{:.2}x", s))
                    .unwrap_or_else(|| "—".to_string());

                // Get status info
                let (status_icon, status_text, status_color, progress_state) = match job.status {
                    JobStatus::Calibrating => {
                        ("⚙", "Calibrating", Color::Cyan, ProgressState::Running)
                    }
                    JobStatus::Running => ("▶", "Running", Color::Yellow, ProgressState::Running),
                    JobStatus::Done => ("✓", "Done", Color::Green, ProgressState::Done),
                    JobStatus::Failed => ("✗", "Failed", Color::Red, ProgressState::Done),
                    JobStatus::Pending => ("⏸", "Pending", Color::DarkGray, ProgressState::Pending),
                    JobStatus::Skipped => ("⏭", "Skipped", Color::Blue, ProgressState::Done),
                };

                // Create progress bar
                let progress_pct = job.progress_pct.min(100.0) as u16;
                let progress_bar = Self::render_progress_bar(progress_pct, progress_state, 20);

                // Build row cells
                let mut cells = vec![
                    Cell::from(format!("{}", idx + 1)),
                    Cell::from(format!("{} {}", status_icon, status_text))
                        .style(Style::default().fg(status_color)),
                    Cell::from(filename),
                    Cell::from(Line::from(in_size).right_aligned()),
                    Cell::from(Line::from(out_size).right_aligned()),
                    Cell::from(Line::from(speed).right_aligned()),
                    Cell::from(progress_bar),
                    Cell::from(eta.clone()),
                ];

                // Add VMAF cell only if Auto-VMAF enabled
                if auto_vmaf_enabled {
                    let vmaf_text = if let Some(vmaf_result) = job.vmaf_result {
                        if let Some(vmaf_target) = job.vmaf_target {
                            format!("{:.1}/{:.0}", vmaf_result, vmaf_target)
                        } else {
                            format!("{:.1}", vmaf_result)
                        }
                    } else if let Some(vmaf_target) = job.vmaf_target {
                        format!("—/{:.0}", vmaf_target)
                    } else {
                        "—".to_string()
                    };
                    cells.push(Cell::from(format!("{:^10}", vmaf_text)));
                }

                let mut row = Row::new(cells);

                // Add hover effect
                if state.hovered_row == Some(idx) {
                    row = row.style(Style::default().bg(Color::DarkGray));
                }

                row
            })
            .collect();

        let mut widths = vec![
            Constraint::Length(3),  // #
            Constraint::Length(12), // STATUS (with icon)
            Constraint::Min(20),    // SOURCE
            Constraint::Length(10), // IN SIZE
            Constraint::Length(10), // OUT SIZE
            Constraint::Length(8),  // SPEED
            Constraint::Length(25), // PROGRESS (wider for bar + percentage)
            Constraint::Length(10), // ETA
        ];

        if auto_vmaf_enabled {
            widths.push(Constraint::Length(10)); // VMAF (result/target)
        }

        // Render using a temporary TableState scoped to the visible slice
        let mut render_state = ratatui::widgets::TableState::default();
        render_state.select(Some(selected - offset));

        let table = Table::new(rows, widths)
            .header(header)
            .column_spacing(2)
            .row_highlight_style(Style::default().reversed())
            .highlight_symbol(">> ");

        frame.render_stateful_widget(table, inner, &mut render_state);
    }

    fn calculate_raw_job_eta(
        job: &crate::engine::VideoJob,
        all_jobs: &[crate::engine::VideoJob],
    ) -> Option<u64> {
        use crate::engine::JobStatus;

        match job.status {
            JobStatus::Calibrating => {
                // Calibrating jobs show estimated calibration time
                if let Some(duration) = job.duration_s {
                    // Estimate ~10% overhead for calibration
                    return Some((duration * 0.1) as u64);
                }
                None
            }
            JobStatus::Running => {
                // Use time-weighted speed (most accurate) > smoothed > raw
                let effective_speed = Self::calculate_time_weighted_speed(job)
                    .or(job.smoothed_speed)
                    .or(job.speed);

                if let (Some(duration), Some(speed)) = (job.duration_s, effective_speed) {
                    if speed > 0.0 {
                        let remaining = duration - job.out_time_s;
                        return Some((remaining / speed) as u64);
                    }
                }
                None
            }
            JobStatus::Pending => {
                if let Some(duration) = job.duration_s {
                    // Calculate avg running speed from all jobs
                    let running_speeds: Vec<f64> = all_jobs
                        .iter()
                        .filter(|j| j.status == JobStatus::Running)
                        .filter_map(|j| j.smoothed_speed.or(j.speed))
                        .collect();

                    let avg_speed = if running_speeds.is_empty() {
                        1.0
                    } else {
                        running_speeds.iter().sum::<f64>() / running_speeds.len() as f64
                    };

                    return Some((duration / avg_speed) as u64);
                }
                None
            }
            _ => None,
        }
    }

    #[allow(dead_code)]
    fn calculate_job_eta(job: &mut crate::engine::VideoJob, state: &DashboardState) -> String {
        let new_eta_seconds = Self::calculate_raw_job_eta(job, &state.jobs);

        // Apply hysteresis: only update display if change is significant
        if let Some(new_eta) = new_eta_seconds {
            let should_update = match job.displayed_eta_seconds {
                Some(old_eta) => {
                    let diff = new_eta.abs_diff(old_eta);
                    // Update if difference is >2 seconds or >5%
                    diff > 2 || (diff as f64 / old_eta.max(1) as f64) > 0.05
                }
                None => true, // First time, always update
            };

            if should_update {
                job.displayed_eta_seconds = Some(new_eta);
            }

            Self::format_duration(job.displayed_eta_seconds.unwrap_or(new_eta))
        } else {
            "—".to_string()
        }
    }

    fn format_size(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.1} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.1} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.1} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }

    fn render_progress_bar(percent: u16, state: ProgressState, width: usize) -> String {
        let filled_width = (width as f64 * (percent as f64 / 100.0)).round() as usize;
        let empty_width = width.saturating_sub(filled_width);

        let (filled_char, empty_char) = match state {
            ProgressState::Running => ('█', '░'),
            ProgressState::Queued => ('▓', '░'),
            ProgressState::Pending => ('░', '░'),
            ProgressState::Done => ('█', ' '),
        };

        let bar = format!(
            "{}{} {}%",
            filled_char.to_string().repeat(filled_width),
            empty_char.to_string().repeat(empty_width),
            percent
        );

        bar
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{JobStatus, VideoJob};
    use std::path::PathBuf;
    use uuid::Uuid;

    fn create_test_job(
        status: JobStatus,
        duration_s: Option<f64>,
        out_time_s: f64,
        speed: Option<f64>,
    ) -> VideoJob {
        VideoJob {
            id: Uuid::new_v4(),
            input_path: PathBuf::from("/test/input.mp4"),
            output_path: PathBuf::from("/test/output.webm"),
            profile: "test".to_string(),
            status,
            overwrite: false,
            duration_s,
            progress_pct: 0.0,
            out_time_s,
            fps: None,
            speed,
            smoothed_speed: None,
            bitrate_kbps: None,
            size_bytes: None,
            started_at: None,
            last_speed_update: None,
            displayed_eta_seconds: None,
            attempts: 0,
            last_error: None,
            vmaf_target: None,
            vmaf_result: None,
            calibrated_quality: None,
            vmaf_partial_scores: Vec::new(),
            calibrating_total_steps: None,
            calibrating_completed_steps: 0,
        }
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(Dashboard::format_duration(30), "30s");
        assert_eq!(Dashboard::format_duration(90), "1m");
        assert_eq!(Dashboard::format_duration(150), "2m");
        assert_eq!(Dashboard::format_duration(3600), "1h 00m");
        assert_eq!(Dashboard::format_duration(3661), "1h 01m");
        assert_eq!(Dashboard::format_duration(7200), "2h 00m");
        assert_eq!(Dashboard::format_duration(7320), "2h 02m");
    }

    #[test]
    fn test_calculate_queue_eta_no_jobs() {
        let state = DashboardState::default();
        let eta = Dashboard::calculate_queue_eta(&state, 1);
        assert_eq!(eta, "—");
    }

    #[test]
    fn test_calculate_queue_eta_one_running_job() {
        let mut state = DashboardState::default();

        // Job: 100s total, 50s done, 50s remaining at 1.0x speed
        state.jobs.push(create_test_job(
            JobStatus::Running,
            Some(100.0),
            50.0,
            Some(1.0),
        ));

        let eta = Dashboard::calculate_queue_eta(&state, 1);
        assert_eq!(eta, "50s");
    }

    #[test]
    fn test_calculate_queue_eta_running_job_with_speed() {
        let mut state = DashboardState::default();

        // Job: 100s total, 50s done, 50s remaining at 2.0x speed = 25s real time
        state.jobs.push(create_test_job(
            JobStatus::Running,
            Some(100.0),
            50.0,
            Some(2.0),
        ));

        let eta = Dashboard::calculate_queue_eta(&state, 1);
        assert_eq!(eta, "25s");
    }

    #[test]
    fn test_calculate_queue_eta_multiple_running_jobs() {
        let mut state = DashboardState::default();

        // Job 1: 100s remaining at 1.0x = 100s
        state.jobs.push(create_test_job(
            JobStatus::Running,
            Some(200.0),
            100.0,
            Some(1.0),
        ));

        // Job 2: 60s remaining at 2.0x = 30s
        state.jobs.push(create_test_job(
            JobStatus::Running,
            Some(120.0),
            60.0,
            Some(2.0),
        ));

        let eta = Dashboard::calculate_queue_eta(&state, 1);
        // Total: 100 + 30 = 130 seconds = 2m
        assert_eq!(eta, "2m");
    }

    #[test]
    fn test_calculate_queue_eta_with_pending_jobs() {
        let mut state = DashboardState::default();

        // Running job: 50s remaining at 1.0x
        state.jobs.push(create_test_job(
            JobStatus::Running,
            Some(100.0),
            50.0,
            Some(1.0),
        ));

        // Pending job: 120s duration (assumes 1.0x speed)
        state
            .jobs
            .push(create_test_job(JobStatus::Pending, Some(120.0), 0.0, None));

        let eta = Dashboard::calculate_queue_eta(&state, 1);
        // Total: 50 + 120 = 170 seconds = 2m
        assert_eq!(eta, "2m");
    }

    #[test]
    fn test_calculate_queue_eta_with_done_jobs() {
        let mut state = DashboardState::default();

        // Running job
        state.jobs.push(create_test_job(
            JobStatus::Running,
            Some(100.0),
            50.0,
            Some(1.0),
        ));

        // Done job (should be ignored)
        state.jobs.push(create_test_job(
            JobStatus::Done,
            Some(200.0),
            200.0,
            Some(1.0),
        ));

        let eta = Dashboard::calculate_queue_eta(&state, 1);
        // Only running job counts: 50s
        assert_eq!(eta, "50s");
    }

    #[test]
    fn test_calculate_queue_eta_mixed_jobs() {
        let mut state = DashboardState::default();

        // Running: 60s remaining
        state.jobs.push(create_test_job(
            JobStatus::Running,
            Some(180.0),
            120.0,
            Some(1.0),
        ));

        // Pending: 300s (5 minutes)
        state
            .jobs
            .push(create_test_job(JobStatus::Pending, Some(300.0), 0.0, None));

        // Done (ignored)
        state.jobs.push(create_test_job(
            JobStatus::Done,
            Some(100.0),
            100.0,
            Some(1.0),
        ));

        // Failed (ignored)
        state
            .jobs
            .push(create_test_job(JobStatus::Failed, Some(100.0), 50.0, None));

        let eta = Dashboard::calculate_queue_eta(&state, 1);
        // Total: 60 + 300 = 360 seconds = 6m
        assert_eq!(eta, "6m");
    }

    #[test]
    fn test_calculate_queue_eta_large_values() {
        let mut state = DashboardState::default();

        // Running: 2 hours remaining
        state.jobs.push(create_test_job(
            JobStatus::Running,
            Some(10800.0), // 3 hours
            3600.0,        // 1 hour done
            Some(1.0),
        ));

        // Pending: 1.5 hours
        state.jobs.push(create_test_job(
            JobStatus::Pending,
            Some(5400.0), // 1.5 hours
            0.0,
            None,
        ));

        let eta = Dashboard::calculate_queue_eta(&state, 1);
        // Total: 7200 + 5400 = 12600 seconds = 3h 30m
        assert_eq!(eta, "3h 30m");
    }

    #[test]
    fn test_calculate_queue_eta_no_duration() {
        let mut state = DashboardState::default();

        // Job without duration metadata
        state
            .jobs
            .push(create_test_job(JobStatus::Running, None, 50.0, Some(1.0)));

        let eta = Dashboard::calculate_queue_eta(&state, 1);
        assert_eq!(eta, "—");
    }

    #[test]
    fn test_calculate_queue_eta_zero_speed() {
        let mut state = DashboardState::default();

        // Job with zero speed (stalled)
        state.jobs.push(create_test_job(
            JobStatus::Running,
            Some(100.0),
            50.0,
            Some(0.0),
        ));

        let eta = Dashboard::calculate_queue_eta(&state, 1);
        // Zero speed means can't calculate, should return "—"
        assert_eq!(eta, "—");
    }

    #[test]
    fn test_calculate_job_eta_running() {
        // Running job: 100s total, 40s done at 2.0x speed = 30s remaining
        let state = DashboardState::default();
        let mut job = create_test_job(JobStatus::Running, Some(100.0), 40.0, Some(2.0));

        let eta = Dashboard::calculate_job_eta(&mut job, &state);
        assert_eq!(eta, "30s");
    }

    #[test]
    fn test_calculate_job_eta_pending() {
        // Pending job: shows full duration (assumes 1.0x when no running jobs)
        let state = DashboardState::default();
        let mut job = create_test_job(JobStatus::Pending, Some(150.0), 0.0, None);

        let eta = Dashboard::calculate_job_eta(&mut job, &state);
        assert_eq!(eta, "2m");
    }

    #[test]
    fn test_calculate_job_eta_done() {
        // Done job: should show "—"
        let state = DashboardState::default();
        let mut job = create_test_job(JobStatus::Done, Some(100.0), 100.0, Some(1.0));

        let eta = Dashboard::calculate_job_eta(&mut job, &state);
        assert_eq!(eta, "—");
    }
}
