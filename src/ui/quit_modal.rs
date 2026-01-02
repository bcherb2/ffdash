// Quit confirmation modal

use crate::ui::state::QuitConfirmationState;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub struct QuitModal;

impl QuitModal {
    pub fn render(frame: &mut Frame, state: &QuitConfirmationState) {
        let area = frame.area();

        // Small centered modal
        let modal_width = 50.min(area.width.saturating_sub(4));
        let modal_height = 7.min(area.height.saturating_sub(2));

        let modal_area = Rect {
            x: (area.width.saturating_sub(modal_width)) / 2,
            y: (area.height.saturating_sub(modal_height)) / 2,
            width: modal_width,
            height: modal_height,
        };

        // Clear background
        frame.render_widget(Clear, modal_area);

        // Render bordered box
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Quit Confirmation ")
            .title_alignment(Alignment::Center)
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(modal_area);
        frame.render_widget(block, modal_area);

        // Build content
        let encode_text = if state.running_count == 1 {
            "1 encode is".to_string()
        } else {
            format!("{} encodes are", state.running_count)
        };

        let lines = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                format!("{} currently running.", encode_text),
                Style::default().fg(Color::White),
            )]),
            Line::from(vec![Span::styled(
                "Quitting will cancel them.",
                Style::default().fg(Color::Gray),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "[Y]",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Quit   "),
                Span::styled(
                    "[N]",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Cancel"),
            ]),
        ];

        let paragraph = Paragraph::new(lines)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White));

        frame.render_widget(paragraph, inner);
    }
}
