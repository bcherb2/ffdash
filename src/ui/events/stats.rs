use super::*;

pub(super) fn handle_stats_key(key: KeyEvent, state: &mut AppState) {
    match key.code {
        // Return to dashboard
        KeyCode::Esc
        | KeyCode::Char('d')
        | KeyCode::Char('D')
        | KeyCode::Char('t')
        | KeyCode::Char('T') => {
            state.current_screen = Screen::Dashboard;
        }
        _ => {}
    }
}
