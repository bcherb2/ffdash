// Config screen implementation

use crate::ui::{
    components::{Footer, render_button, render_checkbox, render_radio_group},
    constants::*,
    focus::ConfigFocus,
    state::{ConfigState, RateControlMode},
    widgets::Slider,
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Widget},
};

mod sections;

pub struct ConfigScreen;

impl ConfigScreen {
    // Helper function to insert cursor at the correct position
    fn insert_cursor(text: &str, cursor_pos: usize) -> String {
        let char_count = text.chars().count();
        let pos = cursor_pos.min(char_count);
        let chars: Vec<char> = text.chars().collect();
        let before: String = chars.iter().take(pos).collect();
        let after: String = chars.iter().skip(pos).collect();
        format!("{}|{}", before, after)
    }
    pub fn render(frame: &mut Frame, state: &mut ConfigState, viewport: &mut Rect) {
        let area = frame.area();
        *viewport = area;

        // Load available profiles if not already loaded
        if state.available_profiles.is_empty() {
            state.refresh_available_profiles();
        }

        // Auto-clear status message after 3 seconds
        if let Some((_, timestamp)) = state.status_message {
            if timestamp.elapsed().as_secs() >= 3 {
                state.status_message = None;
            }
        }

        // Main vertical structure: Profile Bar | Body | Footer
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Profile bar
                Constraint::Min(20),   // Main body (flexible)
                Constraint::Length(1), // Footer
            ])
            .split(area);

        // Split body into upper (core settings) and lower (advanced) sections
        let body_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(55), // Upper: General/Audio + Core Video
                Constraint::Percentage(45), // Lower: Advanced grid
            ])
            .split(main_chunks[1]);

        // Upper body: 2-column split (General/Audio left, Core Video right)
        let upper_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(35), // Left: General & Audio I/O
                Constraint::Percentage(65), // Right: Core Video Encoding
            ])
            .split(body_chunks[0]);

        // Lower body: 3-column grid (Parallelism, GOP, Tuning)
        let lower_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Ratio(1, 3), // Parallelism
                Constraint::Ratio(1, 3), // GOP & Keyframes
                Constraint::Ratio(1, 3), // Tuning & Filters
            ])
            .split(body_chunks[1]);

        // Render all sections
        Self::render_profile_bar(frame, main_chunks[0], state);
        Self::render_general_audio(frame, upper_cols[0], state);
        Self::render_core_video(frame, upper_cols[1], state);
        Self::render_parallelism(frame, lower_cols[0], state);
        Self::render_gop_keyframes(frame, lower_cols[1], state);
        Self::render_tuning_filters(frame, lower_cols[2], state);
        Footer::config().render(main_chunks[2], frame.buffer_mut());

        // Render popup dropdown if active
        if state.active_dropdown.is_some() {
            Self::render_popup_dropdown(frame, state);
        }

        // Render profile name input dialog if active
        if state.name_input_dialog.is_some() {
            Self::render_name_input_dialog(frame, state);
        }

        // Render status message if present
        if state.status_message.is_some() {
            Self::render_status_message(frame, state);
        }
    }
}
