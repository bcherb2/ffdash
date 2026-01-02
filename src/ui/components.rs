// Reusable UI components

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

pub struct Footer {
    content: Line<'static>,
}

impl Footer {
    pub fn dashboard_with_stats(
        total: usize,
        completed: usize,
        errors: usize,
        uptime: String,
        target_workers: u32,
        active_workers: usize,
    ) -> Self {
        let stats_text = format!(
            "Total Jobs: {}, Completed: {}, Errors: {}, Workers: {}/{}, Uptime: {}  |  ",
            total, completed, errors, active_workers, target_workers, uptime
        );

        let mut spans = vec![Span::raw(stats_text)];

        let controls = [
            ("[S]", "tart"),
            ("[R]", "escan"),
            ("[D]", "elete"),
            ("[T]", " Stats"),
            ("[C]", "onfig"),
            ("[H]", "elp"),
            ("[Q]", "uit"),
            ("[ ]", " Workers"),
        ];

        for (i, (hotkey, desc)) in controls.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw("  "));
            }
            spans.push(Span::styled(*hotkey, Style::default().fg(Color::Yellow)));
            spans.push(Span::raw(*desc));
        }

        Self {
            content: Line::from(spans),
        }
    }

    pub fn dashboard() -> Self {
        Self::dashboard_with_stats(0, 0, 0, "00:00:00".to_string(), 1, 0)
    }

    pub fn config() -> Self {
        let controls = [
            ("[↑/↓]", "Navigate"),
            ("[←/→]", "Adjust"),
            ("[Space]", "Toggle"),
            ("[Enter]", "Edit"),
            ("[S]", "ave Profile"),
            ("[D]", "elete Profile"),
            ("[H]", "elp"),
            ("[Esc]", "Cancel"),
        ];

        let mut spans = vec![Span::raw("CONTROLS: ")];

        for (i, (hotkey, desc)) in controls.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw("  "));
            }
            spans.push(Span::styled(*hotkey, Style::default().fg(Color::Yellow)));
            spans.push(Span::raw(" "));
            spans.push(Span::raw(*desc));
        }

        Self {
            content: Line::from(spans),
        }
    }

    pub fn stats() -> Self {
        let controls = [("[T]", " Toggle Stats"), ("[H]", "elp"), ("[Esc]", " Back")];

        let mut spans = vec![Span::raw("CONTROLS: ")];

        for (i, (hotkey, desc)) in controls.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw("  "));
            }
            spans.push(Span::styled(*hotkey, Style::default().fg(Color::Yellow)));
            spans.push(Span::raw(*desc));
        }

        Self {
            content: Line::from(spans),
        }
    }
}

impl Widget for Footer {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(self.content)
            .style(Style::default().bg(Color::DarkGray))
            .render(area, buf);
    }
}

pub fn render_button(label: &str, hotkey: &str, focused: bool, area: Rect, buf: &mut Buffer) {
    let style = if focused {
        Style::default().bg(Color::Blue).fg(Color::White).bold()
    } else {
        Style::default().fg(Color::White)
    };

    let text = Line::from(vec![
        Span::raw("["),
        Span::styled(hotkey, Style::default().fg(Color::Yellow).bold()),
        Span::raw("]"),
        Span::raw(label),
    ])
    .style(style);

    let mut centered_area = area;
    let text_width = label.len() as u16 + hotkey.len() as u16 + 2; // [hotkey] + label
    if area.width > text_width {
        let padding = (area.width - text_width) / 2;
        centered_area.x += padding;
        centered_area.width = text_width;
    }

    buf.set_line(centered_area.x, centered_area.y, &text, centered_area.width);
}

pub fn render_checkbox(label: &str, checked: bool, focused: bool, area: Rect, buf: &mut Buffer) {
    let symbol = if checked { "[x]" } else { "[ ]" };
    let symbol_style = if focused {
        Style::default().fg(Color::Yellow).bold()
    } else {
        Style::default().fg(Color::Cyan)
    };

    let text = Line::from(vec![
        Span::styled(symbol, symbol_style),
        Span::raw(" "),
        Span::raw(label),
    ]);

    buf.set_line(area.x, area.y, &text, area.width);
}

pub fn render_radio_group(
    options: &[&str],
    selected_index: usize,
    focused: bool,
    area: Rect,
    buf: &mut Buffer,
) {
    let mut spans = Vec::new();

    for (i, option) in options.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }

        let symbol = if i == selected_index { "(•)" } else { "( )" };
        let symbol_style = if focused && i == selected_index {
            Style::default().fg(Color::Yellow).bold()
        } else if i == selected_index {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        spans.push(Span::styled(symbol, symbol_style));
        spans.push(Span::raw(" "));
        spans.push(Span::raw(*option));
    }

    let text = Line::from(spans);
    buf.set_line(area.x, area.y, &text, area.width);
}
