use super::*;

pub(super) fn open_help(state: &mut AppState) {
    use crate::engine::{ffmpeg_version, ffprobe_version, hardware};

    // Fetch versions if not already cached
    let ffmpeg = state.ffmpeg_version.clone().or_else(|| {
        let ver = ffmpeg_version().ok();
        state.ffmpeg_version = ver.clone();
        ver
    });

    let ffprobe = state.ffprobe_version.clone().or_else(|| {
        let ver = ffprobe_version().ok();
        state.ffprobe_version = ver.clone();
        ver
    });

    // Fetch hardware preflight result if not already cached
    let hw_result = state.hw_preflight_result.clone().or_else(|| {
        let result = hardware::run_preflight();
        state.hw_preflight_result = Some(result.clone());
        Some(result)
    });

    // Check HuC firmware status if not already cached
    let huc_status = state.huc_available.or_else(|| {
        let available = hardware::check_huc_loaded();
        state.huc_available = Some(available);
        Some(available)
    });

    let vmaf_available = crate::engine::vmaf_filter_available();

    // Check GPU monitoring availability based on detected vendor
    let gpu_metrics_available = if let Some(ref hw) = hw_result {
        hardware::gpu_monitoring_available(hw.gpu_vendor)
    } else {
        false
    };

    state.help_modal = Some(HelpModalState {
        current_section: HelpSection::About,
        scroll_offset: 0,
        max_scroll: 0,
        app_version: state.app_version.clone(),
        ffmpeg_version: ffmpeg,
        ffprobe_version: ffprobe,
        hw_preflight_result: hw_result,
        huc_available: huc_status,
        gpu_metrics_available,
        vmaf_available,
    });
}

pub(super) fn handle_help_key(key: KeyEvent, state: &mut AppState) {
    if let Some(ref mut help_state) = state.help_modal {
        match key.code {
            // Close help
            KeyCode::Esc | KeyCode::Char('h') | KeyCode::Char('H') => {
                state.help_modal = None;
            }
            // Next section
            KeyCode::Tab | KeyCode::Right => {
                help_state.current_section = help_state.current_section.next();
                help_state.scroll_offset = 0; // Reset scroll when changing sections
            }
            // Previous section
            KeyCode::BackTab | KeyCode::Left => {
                help_state.current_section = help_state.current_section.previous();
                help_state.scroll_offset = 0;
            }
            // Scroll up
            KeyCode::Up | KeyCode::Char('k') => {
                help_state.scroll_offset = help_state.scroll_offset.saturating_sub(1);
            }
            // Scroll down
            KeyCode::Down | KeyCode::Char('j') => {
                help_state.scroll_offset = help_state
                    .scroll_offset
                    .saturating_add(1)
                    .min(help_state.max_scroll);
            }
            // Page up
            KeyCode::PageUp => {
                help_state.scroll_offset = help_state.scroll_offset.saturating_sub(10);
            }
            // Page down
            KeyCode::PageDown => {
                help_state.scroll_offset = help_state
                    .scroll_offset
                    .saturating_add(10)
                    .min(help_state.max_scroll);
            }
            // Jump to top
            KeyCode::Home => {
                help_state.scroll_offset = 0;
            }
            // Jump to bottom
            KeyCode::End => {
                help_state.scroll_offset = help_state.max_scroll;
            }
            _ => {}
        }
    }
}
