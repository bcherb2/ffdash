// Dashboard screen implementation

use crate::ui::components::Footer;
use crate::ui::state::DashboardState;
use crate::ui::widgets::ProgressState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Table, Widget},
};
use std::collections::VecDeque;
use waveformchart::{WaveformMode, WaveformWidget};

mod sections;

pub struct Dashboard;

impl Dashboard {
    pub fn render(
        frame: &mut Frame,
        state: &mut DashboardState,
        target_workers: u32,
        active_workers: usize,
        profile_name: Option<&str>,
        use_hw: bool,
        auto_vmaf_enabled: bool,
    ) {
        use crate::engine::JobStatus;

        let area = frame.area();

        // Compact layout for inline mode
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(10), // System metrics (scatter charts)
                Constraint::Length(4),  // Queue overall
                Constraint::Min(0),     // Active jobs table
                Constraint::Length(1),  // Footer
            ])
            .split(area);

        // Render each section
        Self::render_system_metrics(frame, chunks[0], state, use_hw);
        Self::render_queue_overall(frame, chunks[1], state, profile_name, target_workers);
        Self::render_active_jobs(frame, chunks[2], state, auto_vmaf_enabled);

        // Calculate stats for footer (exclude skipped jobs from total)
        let all_jobs = state.jobs.len();
        let skipped = state
            .jobs
            .iter()
            .filter(|j| j.status == JobStatus::Skipped)
            .count();
        let total = all_jobs - skipped;
        let completed = state
            .jobs
            .iter()
            .filter(|j| j.status == JobStatus::Done)
            .count();
        let errors = state
            .jobs
            .iter()
            .filter(|j| j.status == JobStatus::Failed)
            .count();
        let uptime = Self::format_uptime(state.start_time.elapsed().as_secs());

        Footer::dashboard_with_stats(
            total,
            completed,
            errors,
            uptime,
            target_workers,
            active_workers,
        )
        .render(chunks[3], frame.buffer_mut());
    }
}
