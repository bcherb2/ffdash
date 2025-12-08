// Enhanced progress bar with different visual states

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressState {
    Running,
    Done,
    Queued,
    Pending,
}

pub struct EnhancedProgress {
    percent: u16,
    state: ProgressState,
}

impl EnhancedProgress {
    pub fn new(percent: u16, state: ProgressState) -> Self {
        Self {
            percent: percent.min(100),
            state,
        }
    }
}

impl Widget for EnhancedProgress {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let ratio = self.percent as f64 / 100.0;
        let filled_width = (area.width as f64 * ratio).round() as u16;

        let (filled_symbol, unfilled_symbol, filled_fg, unfilled_fg) = match self.state {
            ProgressState::Running => ("█", "░", Color::White, Color::DarkGray),
            ProgressState::Done => ("█", " ", Color::White, Color::Black),
            ProgressState::Queued => ("▓", "░", Color::DarkGray, Color::Black),
            ProgressState::Pending => ("░", "░", Color::DarkGray, Color::Black),
        };

        // Draw filled portion
        for x in 0..filled_width {
            if x < area.width {
                buf.set_string(
                    area.x + x,
                    area.y,
                    filled_symbol,
                    Style::default().fg(filled_fg),
                );
            }
        }

        // Draw unfilled portion
        for x in filled_width..area.width {
            buf.set_string(
                area.x + x,
                area.y,
                unfilled_symbol,
                Style::default().fg(unfilled_fg),
            );
        }
    }
}
