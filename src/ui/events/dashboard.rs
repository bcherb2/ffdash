use super::{workers, *};

pub(super) fn handle_dashboard_key(key: KeyEvent, state: &mut AppState) {
    match key.code {
        // Switch to config
        KeyCode::Char('c') | KeyCode::Char('C') => {
            state.current_screen = Screen::Config;
        }
        // Switch to stats
        KeyCode::Char('t') | KeyCode::Char('T') => {
            // Update only jobs_pending count from current jobs (keep accumulated stats)
            use crate::engine::JobStatus;
            state.stats.session.jobs_pending = state
                .dashboard
                .jobs
                .iter()
                .filter(|j| j.status == JobStatus::Pending)
                .count();
            state.current_screen = Screen::Stats;
        }
        // Navigate table
        KeyCode::Up => {
            let selected = state.dashboard.table_state.selected();
            if let Some(i) = selected {
                if i > 0 {
                    state.dashboard.table_state.select(Some(i - 1));
                }
            }
        }
        KeyCode::Down => {
            let selected = state.dashboard.table_state.selected();
            let job_count = state.dashboard.jobs.len();
            if let Some(i) = selected {
                if job_count > 0 && i < job_count - 1 {
                    state.dashboard.table_state.select(Some(i + 1));
                }
            }
        }
        // Cycle foreground job
        KeyCode::Tab => {
            use crate::engine::JobStatus;
            let running_count = state
                .dashboard
                .jobs
                .iter()
                .filter(|j| j.status == JobStatus::Running)
                .count();
            if running_count > 0 {
                state.dashboard.foreground_job_index =
                    (state.dashboard.foreground_job_index + 1) % running_count;
            }
        }
        // Start encoding (for testing - prompts for directory)
        KeyCode::Char('s') | KeyCode::Char('S') => {
            // For now, use current directory as default
            // In production, this would show a directory picker dialog
            let is_encoding = state
                .dashboard
                .jobs
                .iter()
                .any(|j| matches!(j.status, crate::engine::JobStatus::Running));
            if !is_encoding {
                let current_dir =
                    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                match workers::start_encoding(state, current_dir) {
                    Ok(_) => {
                        // Encoding started successfully
                    }
                    Err(e) => {
                        // TODO: Show error message in UI
                        eprintln!("Failed to start encoding: {}", e);
                    }
                }
            }
        }
        // Rescan current directory
        KeyCode::Char('r') | KeyCode::Char('R') => {
            // Only allow rescan if not currently encoding
            let is_encoding = state
                .dashboard
                .jobs
                .iter()
                .any(|j| matches!(j.status, crate::engine::JobStatus::Running));
            if !is_encoding {
                if let Some(root) = &state.root_path {
                    match workers::rescan_directory(state, root.clone()) {
                        Ok(_) => {
                            // Rescan successful - clear worker pool if all jobs are done
                            let all_done = state.dashboard.jobs.iter().all(|j| {
                                matches!(
                                    j.status,
                                    crate::engine::JobStatus::Done
                                        | crate::engine::JobStatus::Failed
                                )
                            });
                            if all_done {
                                state.worker_pool = None;
                            }
                        }
                        Err(e) => {
                            // TODO: Show error message in UI
                            eprintln!("Failed to rescan: {}", e);
                        }
                    }
                }
            }
        }
        // Toggle skip status (Space key)
        KeyCode::Char(' ') => {
            if let Some(selected) = state.dashboard.table_state.selected() {
                if selected < state.dashboard.jobs.len() {
                    let job = &mut state.dashboard.jobs[selected];
                    let previous_status = job.status.clone();

                    // Only toggle Pending, Failed, or Skipped jobs
                    match job.status {
                        crate::engine::JobStatus::Pending => {
                            job.status = crate::engine::JobStatus::Skipped;
                        }
                        crate::engine::JobStatus::Failed => {
                            job.status = crate::engine::JobStatus::Skipped;
                            job.last_error = None; // Clear error on skip
                        }
                        crate::engine::JobStatus::Skipped => {
                            job.status = crate::engine::JobStatus::Pending;
                            job.last_error = None; // Fresh start
                        }
                        crate::engine::JobStatus::Running | crate::engine::JobStatus::Done => {
                            // Ignore - these statuses cannot be toggled
                        }
                    }

                    // Update enc_state if it exists (keep in sync)
                    if let Some(ref mut enc_state) = state.enc_state {
                        if selected < enc_state.jobs.len() {
                            enc_state.jobs[selected].status = job.status.clone();
                            enc_state.jobs[selected].last_error = job.last_error.clone();
                        }
                    }

                    // Persist state to disk
                    if let (Some(enc_state), Some(root)) = (&state.enc_state, &state.root_path) {
                        let _ = enc_state.save(root); // Ignore write errors (matches existing pattern)
                    }

                    // If we just re-queued a job and workers are available, try to spawn it
                    if previous_status == crate::engine::JobStatus::Skipped
                        && job.status == crate::engine::JobStatus::Pending
                        && state.worker_pool.is_some()
                    {
                        workers::spawn_next_job(state);
                    }
                }
            }
        }
        // Delete .enc_state and exit
        KeyCode::Char('x') | KeyCode::Char('X') => {
            // Only allow deleting if no jobs are running
            let is_encoding = state
                .dashboard
                .jobs
                .iter()
                .any(|j| matches!(j.status, crate::engine::JobStatus::Running));
            if !is_encoding {
                // Delete .enc_state file if it exists
                if let Some(root) = &state.root_path {
                    let state_path = root.join(".enc_state");
                    if state_path.exists() {
                        if let Err(e) = std::fs::remove_file(&state_path) {
                            eprintln!("Failed to delete .enc_state: {}", e);
                        }
                    }
                }
                // Quit app
                std::process::exit(0);
            }
        }
        // Decrease worker count
        KeyCode::Char('[') => {
            if state.config.max_workers > 1 {
                state.config.max_workers -= 1;
                state.config.is_modified = true;

                if let Some(pool) = &state.worker_pool {
                    pool.set_max_workers(state.config.max_workers as usize);
                }
            }
        }
        // Increase worker count
        KeyCode::Char(']') => {
            if state.config.max_workers < 16 {
                state.config.max_workers += 1;
                state.config.is_modified = true;

                if let Some(pool) = &state.worker_pool {
                    pool.set_max_workers(state.config.max_workers as usize);
                    // Try to spawn additional workers for pending jobs
                    workers::spawn_next_job(state);
                }
            }
        }
        _ => {}
    }
}

pub(super) fn handle_dashboard_mouse(mouse: MouseEvent, state: &mut AppState) {
    let dashboard = &mut state.dashboard;

    // Update hover state on mouse movement
    if matches!(mouse.kind, MouseEventKind::Moved) {
        if let Some(inner_area) = dashboard.table_inner_area {
            dashboard.hovered_row =
                calculate_hovered_row(mouse.row, inner_area, dashboard.table_state.offset());
        }
    }

    // Handle scrolling
    match mouse.kind {
        MouseEventKind::ScrollDown => {
            if is_mouse_in_table(mouse.column, mouse.row, dashboard) {
                let current = dashboard.table_state.selected().unwrap_or(0);
                let job_count = dashboard.jobs.len();
                if job_count > 0 && current < job_count - 1 {
                    dashboard.table_state.select(Some(current + 1));
                }
            }
        }
        MouseEventKind::ScrollUp => {
            if is_mouse_in_table(mouse.column, mouse.row, dashboard) {
                let current = dashboard.table_state.selected().unwrap_or(0);
                if current > 0 {
                    dashboard.table_state.select(Some(current - 1));
                }
            }
        }
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some(inner_area) = dashboard.table_inner_area {
                if let Some(row) =
                    calculate_clicked_row(mouse.row, inner_area, dashboard.table_state.offset())
                {
                    dashboard.table_state.select(Some(row));
                }
            }
        }
        _ => {}
    }
}

fn is_mouse_in_table(x: u16, y: u16, dashboard: &crate::ui::state::DashboardState) -> bool {
    dashboard.table_inner_area.map_or(false, |area| {
        x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height
    })
}

// Calculate which row is hovered based on mouse position
fn calculate_hovered_row(
    mouse_row: u16,
    inner_area: ratatui::layout::Rect,
    offset: usize,
) -> Option<usize> {
    // Adjust for table header row and border
    let first_row_y = inner_area.y + 1; // Skip header
    if mouse_row < first_row_y || mouse_row >= inner_area.y + inner_area.height {
        return None; // Outside table bounds
    }

    Some(offset + (mouse_row - first_row_y) as usize)
}

// Calculate which row was clicked
fn calculate_clicked_row(
    mouse_row: u16,
    inner_area: ratatui::layout::Rect,
    offset: usize,
) -> Option<usize> {
    calculate_hovered_row(mouse_row, inner_area, offset)
}
