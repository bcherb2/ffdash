// Custom slider widget for numeric value adjustment

use crossterm::event::KeyCode;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

pub struct Slider {
    value: u32,
    min: u32,
    max: u32,
    label: String,
    focused: bool,
}

impl Slider {
    pub fn new(label: impl Into<String>, min: u32, max: u32) -> Self {
        Self {
            value: min,
            min,
            max,
            label: label.into(),
            focused: false,
        }
    }

    pub fn value(mut self, value: u32) -> Self {
        self.value = value.clamp(self.min, self.max);
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn get_value(&self) -> u32 {
        self.value
    }

    pub fn handle_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Left => {
                if self.value > self.min {
                    self.value -= 1;
                    true
                } else {
                    false
                }
            }
            KeyCode::Right => {
                if self.value < self.max {
                    self.value += 1;
                    true
                } else {
                    false
                }
            }
            KeyCode::Home => {
                if self.value != self.min {
                    self.value = self.min;
                    true
                } else {
                    false
                }
            }
            KeyCode::End => {
                if self.value != self.max {
                    self.value = self.max;
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

impl Widget for Slider {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Calculate dimensions
        let border_style = if self.focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        let value_style = if self.focused {
            Style::default().fg(Color::Cyan).bold()
        } else {
            Style::default().fg(Color::Cyan)
        };

        // Create block with label
        let block = Block::default()
            .borders(Borders::NONE)
            .border_style(border_style);

        let inner = block.inner(area);

        // Render label with value
        if inner.height >= 2 {
            let label_line = Line::from(vec![
                Span::raw(&self.label),
                Span::raw(": "),
                Span::styled(format!("{}", self.value), value_style),
                Span::styled(
                    format!(" ({}-{})", self.min, self.max),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);

            let label_x = inner.x;
            let label_y = inner.y;
            buf.set_line(label_x, label_y, &label_line, inner.width);

            // Render slider bar
            if inner.height > 1 && inner.width > 0 {
                let bar_y = inner.y + 1;
                let range = self.max - self.min;
                let ratio = if range > 0 {
                    (self.value - self.min) as f64 / range as f64
                } else {
                    0.0
                };
                let filled_width = (inner.width as f64 * ratio).round() as u16;

                // Draw filled portion
                for x in 0..filled_width {
                    if x < inner.width {
                        buf.set_string(
                            inner.x + x,
                            bar_y,
                            "█",
                            Style::default().fg(if self.focused {
                                Color::Blue
                            } else {
                                Color::DarkGray
                            }),
                        );
                    }
                }

                // Draw unfilled portion
                for x in filled_width..inner.width {
                    buf.set_string(
                        inner.x + x,
                        bar_y,
                        "─",
                        Style::default().fg(Color::DarkGray),
                    );
                }
            }
        }
    }
}
