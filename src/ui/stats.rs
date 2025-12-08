// Stats screen implementation

use crate::stats::StatsState;
use crate::ui::components::Footer;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use tui_piechart::{PieChart, PieSlice};

pub struct StatsScreen;

impl StatsScreen {
    pub fn render(frame: &mut Frame, state: &mut StatsState) {
        let area = frame.area();

        // Main vertical split
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title bar
                Constraint::Length(8), // Text stats
                Constraint::Min(0),    // Pie charts
                Constraint::Length(1), // Footer
            ])
            .split(area);

        // Render title
        Self::render_title(frame, chunks[0]);

        // Render text stats
        Self::render_text_stats(frame, chunks[1], state);

        // Two-column split for pie charts
        let chart_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // Space savings chart
                Constraint::Percentage(50), // Completion chart
            ])
            .split(chunks[2]);

        // Render charts
        Self::render_space_chart(frame, chart_chunks[0], state);
        Self::render_completion_chart(frame, chart_chunks[1], state);

        // Render footer
        frame.render_widget(Footer::stats(), chunks[3]);
    }

    fn render_title(frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" QUEUE STATISTICS ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan).bold());

        frame.render_widget(block, area);
    }

    fn render_text_stats(frame: &mut Frame, area: Rect, state: &StatsState) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);

        let input_bytes = state.session.input_bytes;
        let output_bytes = state.session.output_bytes;
        let encode_time = state.session.encode_time_secs;
        let jobs_completed = state.session.jobs_done as u64;
        let jobs_failed = state.session.jobs_failed as u64;

        let compression_ratio = if output_bytes > 0 {
            input_bytes as f64 / output_bytes as f64
        } else {
            0.0
        };

        let ratio_text = if compression_ratio > 0.0 {
            format!("{:.2}:1", compression_ratio)
        } else {
            "N/A".to_string()
        };

        let space_saved = input_bytes.saturating_sub(output_bytes);

        let lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::raw("  Total Input: "),
                Span::styled(
                    crate::stats::format_bytes(input_bytes),
                    Style::default().fg(Color::White).bold(),
                ),
                Span::raw("   │   Output: "),
                Span::styled(
                    crate::stats::format_bytes(output_bytes),
                    Style::default().fg(Color::LightBlue).bold(),
                ),
                Span::raw("   │   Saved: "),
                Span::styled(
                    crate::stats::format_bytes(space_saved),
                    Style::default().fg(Color::LightGreen).bold(),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("  Compression: "),
                Span::styled(ratio_text, Style::default().fg(Color::Magenta).bold()),
                Span::raw("   │   Encode Time: "),
                Span::styled(
                    crate::stats::format_duration(encode_time),
                    Style::default().fg(Color::Blue).bold(),
                ),
                Span::raw("   │   Jobs: "),
                Span::styled(
                    format!("{} completed", jobs_completed),
                    Style::default().fg(Color::Green),
                ),
                Span::raw(", "),
                Span::styled(
                    format!("{} failed", jobs_failed),
                    Style::default().fg(Color::Red),
                ),
            ]),
        ];

        let paragraph = Paragraph::new(lines);

        frame.render_widget(block, area);
        frame.render_widget(paragraph, inner);
    }

    fn render_space_chart(frame: &mut Frame, area: Rect, state: &StatsState) {
        let input_bytes = state.session.input_bytes;
        let output_bytes = state.session.output_bytes;

        if input_bytes == 0 {
            let message = Paragraph::new("No data yet")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .title(" Space Savings ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Yellow)),
                );
            frame.render_widget(message, area);
            return;
        }

        let space_saved = input_bytes.saturating_sub(output_bytes);

        // Create labels with sizes - these must live until chart is rendered
        let output_label = format!("Final Size ({})", crate::stats::format_bytes(output_bytes));
        let saved_label = format!("Reclaimed ({})", crate::stats::format_bytes(space_saved));

        // Create slices - only include non-zero values to avoid rendering issues
        let mut slices = Vec::new();
        if output_bytes > 0 {
            slices.push(PieSlice::new(
                &output_label,
                output_bytes as f64,
                Color::LightBlue,
            ));
        }
        if space_saved > 0 {
            slices.push(PieSlice::new(
                &saved_label,
                space_saved as f64,
                Color::LightGreen,
            ));
        }

        // If no slices (shouldn't happen given input_bytes > 0 check above), show all output
        if slices.is_empty() {
            slices.push(PieSlice::new(
                &output_label,
                output_bytes as f64,
                Color::LightBlue,
            ));
        }

        let chart = PieChart::new(slices)
            .show_legend(true)
            .show_percentages(true)
            .block(
                Block::default()
                    .title(" Space Savings ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            );

        frame.render_widget(chart, area);
    }

    fn render_completion_chart(frame: &mut Frame, area: Rect, state: &StatsState) {
        let jobs_done = state.session.jobs_done;
        let jobs_pending = state.session.jobs_pending;
        let jobs_failed = state.session.jobs_failed;

        let total = jobs_done + jobs_pending + jobs_failed;

        if total == 0 {
            let message = Paragraph::new("No jobs yet")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .title(" Job Status ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan)),
                );
            frame.render_widget(message, area);
            return;
        }

        // Simplify to Done vs Not Done (always 2 slices unless 0% or 100%)
        let not_done = jobs_pending + jobs_failed;

        let done_label = format!("Done ({})", jobs_done);
        let not_done_label = format!("Not Done ({})", not_done);

        let mut slices = Vec::new();

        if jobs_done > 0 {
            slices.push(PieSlice::new(&done_label, jobs_done as f64, Color::Cyan));
        }
        if not_done > 0 {
            slices.push(PieSlice::new(
                &not_done_label,
                not_done as f64,
                Color::Yellow,
            ));
        }

        // tui_piechart doesn't render single-slice charts correctly (shows tiny sliver at 0% or 100%)
        // Add a tiny dummy slice when only one category exists to force proper rendering
        if slices.len() == 1 {
            slices.push(PieSlice::new("", 0.001, Color::Reset));
        }

        let chart = PieChart::new(slices)
            .show_legend(true)
            .show_percentages(true)
            .block(
                Block::default()
                    .title(" Job Status ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            );

        frame.render_widget(chart, area);
    }
}
