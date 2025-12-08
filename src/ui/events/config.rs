use super::*;

fn get_profile_count(config: &crate::ui::state::ConfigState) -> usize {
    use crate::engine::Profile;

    let mut count = 0;

    // Built-in profiles
    count += Profile::builtin_names().len();

    // Saved profiles (excluding built-ins)
    for saved_profile in &config.available_profiles {
        if !Profile::builtin_names().contains(saved_profile) {
            count += 1;
        }
    }

    // "Custom" if modified
    if config.is_modified {
        count += 1;
    }

    // Always "Create New..."
    count += 1;

    count
}

pub(super) fn initialize_default_profile(state: &mut AppState, config: &crate::config::Config) {
    use crate::engine::Profile;

    // Determine which profile to load
    // Priority: last_used_profile > defaults.profile > "1080p Shrinker"
    let profile_name = config
        .defaults
        .last_used_profile
        .as_ref()
        .map(|s| s.as_str())
        .or(Some(config.defaults.profile.as_str()))
        .unwrap_or("1080p Shrinker");

    // Try to load the profile (try user-saved first, then built-in, then fallback to 1080p Shrinker)
    let mut loaded = false;

    // Try loading from disk (user-saved profiles)
    if let Ok(profiles_dir) = Profile::profiles_dir() {
        if let Ok(profile) = Profile::load(&profiles_dir, profile_name) {
            profile.apply_to_config(&mut state.config);
            state.config.current_profile_name = Some(profile_name.to_string());
            state.config.is_modified = false;
            loaded = true;
        }
    }

    // Try built-in profile if not found on disk
    if !loaded {
        if let Some(profile) = Profile::get_builtin(profile_name) {
            profile.apply_to_config(&mut state.config);
            state.config.current_profile_name = Some(profile_name.to_string());
            state.config.is_modified = false;
            loaded = true;
        }
    }

    // Final fallback to 1080p Shrinker if the configured profile doesn't exist
    if !loaded {
        if let Some(profile) = Profile::get_builtin("1080p Shrinker") {
            profile.apply_to_config(&mut state.config);
            state.config.current_profile_name = Some("1080p Shrinker".to_string());
            state.config.is_modified = false;
        }
    }
    // If even 1080p Shrinker builtin not found, keep the hardcoded defaults from ConfigState::default()

    // Apply global settings from config
    state.config.overwrite = config.defaults.overwrite;
    state.config.use_hardware_encoding = config.defaults.use_hardware_encoding;
    state.config.filename_pattern = config.defaults.filename_pattern.clone();

    // Update profile_list_state to select the correct index for the loaded profile
    // Build the profile list (same as UI rendering logic)
    let mut profiles = Vec::new();
    profiles.extend(Profile::builtin_names());

    // Refresh and add saved profiles
    state.config.refresh_available_profiles();
    for saved_profile in &state.config.available_profiles.clone() {
        if !Profile::builtin_names().contains(saved_profile) {
            profiles.push(saved_profile.clone());
        }
    }

    // Find the index of the current profile
    if let Some(ref current_name) = state.config.current_profile_name {
        if let Some(index) = profiles.iter().position(|p| p == current_name) {
            state.config.profile_list_state.select(Some(index));
        }
    }
}

fn delete_profile(state: &mut AppState, name: String) {
    use crate::engine::Profile;

    // Don't allow deleting built-in profiles
    if Profile::builtin_names().contains(&name) {
        return;
    }

    // Get profiles directory and delete
    if let Ok(profiles_dir) = Profile::profiles_dir() {
        if let Ok(()) = Profile::delete(&profiles_dir, &name) {
            // Update state after successful deletion
            state.config.current_profile_name = None;
            state.config.is_modified = true; // Now in custom state
            state.config.refresh_available_profiles();

            // Reset selection to a safe index (first item)
            state.config.profile_list_state.select(Some(0));

            // Show success message
            state.config.status_message = Some((
                format!("Profile '{}' deleted", name),
                std::time::Instant::now(),
            ));
        }
    }
}

fn save_profile_with_name(state: &mut AppState, name: String) {
    use crate::engine::{Profile, write_debug_log};

    // Create Profile from current ConfigState
    let profile = Profile::from_config(name.clone(), &state.config);

    // Get profiles directory and save
    match Profile::profiles_dir() {
        Ok(profiles_dir) => {
            match profile.save(&profiles_dir) {
                Ok(()) => {
                    // Update state after successful save
                    state.config.current_profile_name = Some(name.clone());
                    state.config.is_modified = false;
                    state.config.refresh_available_profiles();

                    // Update last_used_profile, use_hardware_encoding, and filename_pattern in config.toml so they load on next startup
                    if let Ok(mut config) = crate::config::Config::load() {
                        config.defaults.last_used_profile = Some(name.clone());
                        config.defaults.use_hardware_encoding = state.config.use_hardware_encoding;
                        config.defaults.filename_pattern = state.config.filename_pattern.clone();
                        let _ = config.save(); // Ignore errors
                    }

                    // Log success
                    let _ = write_debug_log(&format!("Profile '{}' saved successfully", name));

                    // Show success message
                    state.config.status_message = Some((
                        format!("Profile '{}' saved", name),
                        std::time::Instant::now(),
                    ));
                }
                Err(e) => {
                    // Log error to file (don't show in TUI)
                    let _ = write_debug_log(&format!("Failed to save profile '{}': {}", name, e));
                }
            }
        }
        Err(e) => {
            // Log error to file (don't show in TUI)
            let _ = write_debug_log(&format!(
                "Failed to get profiles directory for '{}': {}",
                name, e
            ));
        }
    }
}

fn load_selected_profile(state: &mut AppState) {
    use crate::engine::Profile;

    // Build profile list (same as UI rendering logic)
    let mut profiles = Vec::new();

    // Add built-in profiles
    profiles.extend(Profile::builtin_names());

    // Add saved profiles (excluding built-ins to avoid duplicates)
    for saved_profile in &state.config.available_profiles.clone() {
        if !Profile::builtin_names().contains(saved_profile) {
            profiles.push(saved_profile.clone());
        }
    }

    // Add "Custom" if modified
    if state.config.is_modified {
        profiles.push("Custom".to_string());
    }

    // Always add "Create New..."
    profiles.push("Create New...".to_string());

    // Get selected profile name
    let selected_index = state.config.profile_list_state.selected().unwrap_or(0);
    let profile_name = profiles.get(selected_index).cloned();

    if let Some(name) = profile_name {
        match name.as_str() {
            "Custom" => {
                // Do nothing - already in custom state
            }
            "Create New..." => {
                // Open profile name input dialog
                state.config.name_input_dialog = Some(String::new());
            }
            _ => {
                // Try to load from disk first (allows users to override built-ins)
                let mut loaded = false;
                if let Ok(profiles_dir) = Profile::profiles_dir() {
                    if let Ok(profile) = Profile::load(&profiles_dir, &name) {
                        profile.apply_to_config(&mut state.config);
                        state.config.current_profile_name = Some(name.clone());
                        state.config.is_modified = false;
                        loaded = true;
                    }
                }

                // Fall back to built-in if not found on disk
                if !loaded {
                    if let Some(builtin_profile) = Profile::get_builtin(&name) {
                        builtin_profile.apply_to_config(&mut state.config);
                        state.config.current_profile_name = Some(name.clone());
                        state.config.is_modified = false;
                    }
                }

                // Don't update last_used_profile here - only update it when user explicitly saves
                // This allows users to browse profiles without changing what loads on next startup
            }
        }
    }
}

pub(super) fn handle_config_key(key: KeyEvent, state: &mut AppState) {
    // If profile name input dialog is active, handle text input
    if let Some(ref mut name) = state.config.name_input_dialog {
        match key.code {
            KeyCode::Esc => {
                // Cancel - close dialog without saving
                state.config.name_input_dialog = None;
                return;
            }
            KeyCode::Enter => {
                // Validate and save profile
                if !name.is_empty() {
                    let profile_name = name.clone();
                    state.config.name_input_dialog = None;
                    save_profile_with_name(state, profile_name);
                }
                return;
            }
            KeyCode::Char(c) => {
                // Add character to name (limit length to 50 chars)
                if name.len() < 50 && (c.is_alphanumeric() || c == '_' || c == '-' || c == ' ') {
                    name.push(c);
                }
                return;
            }
            KeyCode::Backspace => {
                // Remove last character
                name.pop();
                return;
            }
            _ => {
                return;
            }
        }
    }

    // If a dropdown is active, handle popup-specific keys
    if state.config.active_dropdown.is_some() {
        match key.code {
            KeyCode::Esc => {
                // Close popup without selecting
                state.config.active_dropdown = None;
                return;
            }
            KeyCode::Enter => {
                // Load selected profile if ProfileList dropdown
                if state.config.active_dropdown == Some(ConfigFocus::ProfileList) {
                    load_selected_profile(state);
                }
                // Close popup (selection is already highlighted)
                state.config.active_dropdown = None;
                return;
            }
            KeyCode::Up => {
                // Navigate within popup
                handle_focused_widget_key(key, state);
                return;
            }
            KeyCode::Down => {
                // Navigate within popup
                handle_focused_widget_key(key, state);
                return;
            }
            _ => {
                // Close on any other key
                state.config.active_dropdown = None;
                return;
            }
        }
    }

    match key.code {
        // Switch back to dashboard
        KeyCode::Esc => {
            use crate::ui::state::InputMode;
            state.current_screen = Screen::Dashboard;
            state.config.input_mode = InputMode::Normal;
        }
        // Global hotkeys
        KeyCode::Char('s') | KeyCode::Char('S')
            if key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            // Ctrl+S: Save profile
            if let Some(ref name) = state.config.current_profile_name {
                // Have a profile loaded - overwrite it
                save_profile_with_name(state, name.clone());
            } else {
                // No profile loaded (Custom) - prompt for name
                state.config.name_input_dialog = Some(String::new());
            }
        }
        KeyCode::Char('d') | KeyCode::Char('D')
            if key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            // Ctrl+D: Delete current profile
            if let Some(ref name) = state.config.current_profile_name {
                delete_profile(state, name.clone());
            }
        }
        // Focus navigation
        KeyCode::Tab => {
            let start_focus = state.config.focus;
            loop {
                state.config.focus = state.config.focus.next();
                // Skip controls that aren't currently rendered
                if is_focus_visible(&state.config) || state.config.focus == start_focus {
                    break;
                }
            }
            reset_cursor_position_for_focus(state);
            update_input_mode_for_focus(state);
        }
        KeyCode::BackTab => {
            let start_focus = state.config.focus;
            loop {
                state.config.focus = state.config.focus.previous();
                // Skip controls that aren't currently rendered
                if is_focus_visible(&state.config) || state.config.focus == start_focus {
                    break;
                }
            }
            reset_cursor_position_for_focus(state);
            update_input_mode_for_focus(state);
        }
        // Handle focused widget input
        _ => handle_focused_widget_key(key, state),
    }
}

// Check if a focus target is currently visible (has its area set)
fn is_focus_visible(config: &crate::ui::state::ConfigState) -> bool {
    use crate::ui::focus::ConfigFocus;
    match config.focus {
        // Slider controls - check if area is set
        ConfigFocus::CrfSlider => config.crf_slider_area.is_some(),
        ConfigFocus::QsvGlobalQualitySlider => config.qsv_quality_slider_area.is_some(),
        ConfigFocus::VaapiCompressionLevelSlider => {
            config.vaapi_compression_level_slider_area.is_some()
        }
        ConfigFocus::CpuUsedSlider => config.cpu_used_slider_area.is_some(),
        ConfigFocus::CpuUsedPass1Slider => config.cpu_used_pass1_slider_area.is_some(),
        ConfigFocus::CpuUsedPass2Slider => config.cpu_used_pass2_slider_area.is_some(),
        ConfigFocus::TileColumnsSlider => config.tile_columns_slider_area.is_some(),
        ConfigFocus::TileRowsSlider => config.tile_rows_slider_area.is_some(),
        ConfigFocus::LagInFramesSlider => config.lag_in_frames_slider_area.is_some(),
        ConfigFocus::ArnrMaxFramesSlider => config.arnr_max_frames_slider_area.is_some(),
        ConfigFocus::ArnrStrengthSlider => config.arnr_strength_slider_area.is_some(),
        ConfigFocus::SharpnessSlider => config.sharpness_slider_area.is_some(),
        ConfigFocus::NoiseSensitivitySlider => config.noise_sensitivity_slider_area.is_some(),
        ConfigFocus::AudioBitrateSlider => config.audio_bitrate_slider_area.is_some(),
        ConfigFocus::ForceStereoCheckbox => config.force_stereo_checkbox_area.is_some(),

        // Input controls - check if area is set
        ConfigFocus::VideoTargetBitrateInput => config.video_target_bitrate_area.is_some(),
        ConfigFocus::VideoMinBitrateInput => config.video_min_bitrate_area.is_some(),
        ConfigFocus::VideoMaxBitrateInput => config.video_max_bitrate_area.is_some(),
        ConfigFocus::VideoBufsizeInput => config.video_bufsize_area.is_some(),
        ConfigFocus::UndershootPctInput => config.undershoot_pct_area.is_some(),
        ConfigFocus::OvershootPctInput => config.overshoot_pct_area.is_some(),
        ConfigFocus::VaapiBFramesInput => config.vaapi_b_frames_area.is_some(),
        ConfigFocus::VaapiLoopFilterLevelInput => config.vaapi_loop_filter_level_area.is_some(),
        ConfigFocus::VaapiLoopFilterSharpnessInput => {
            config.vaapi_loop_filter_sharpness_area.is_some()
        }
        ConfigFocus::ThreadsInput => config.threads_area.is_some(),
        ConfigFocus::MaxWorkersInput => config.max_workers_area.is_some(),
        ConfigFocus::GopLengthInput => config.gop_length_area.is_some(),
        ConfigFocus::KeyintMinInput => config.keyint_min_area.is_some(),
        ConfigFocus::StaticThreshInput => config.static_thresh_area.is_some(),
        ConfigFocus::MaxIntraRateInput => config.max_intra_rate_area.is_some(),

        // All other controls (checkboxes, dropdowns, buttons) are always visible when in their section
        _ => true,
    }
}

// Reset cursor position to end of text when entering a text input field
fn reset_cursor_position_for_focus(state: &mut AppState) {
    state.config.cursor_pos = match state.config.focus {
        ConfigFocus::OutputDirectory => state.config.output_dir.chars().count(),
        ConfigFocus::FilenamePattern => state
            .config
            .filename_pattern
            .as_ref()
            .map(|s| s.chars().count())
            .unwrap_or(0),
        ConfigFocus::VideoTargetBitrateInput => {
            if state.config.video_target_bitrate == 0 {
                "0 kbps".chars().count()
            } else {
                format!("{} kbps", state.config.video_target_bitrate)
                    .chars()
                    .count()
            }
        }
        ConfigFocus::VideoBufsizeInput => {
            if state.config.video_bufsize == 0 {
                "Auto".chars().count()
            } else {
                format!("{} kbps", state.config.video_bufsize)
                    .chars()
                    .count()
            }
        }
        ConfigFocus::VideoMinBitrateInput => {
            if state.config.video_min_bitrate == 0 {
                "None".chars().count()
            } else {
                format!("{} kbps", state.config.video_min_bitrate)
                    .chars()
                    .count()
            }
        }
        ConfigFocus::VideoMaxBitrateInput => {
            if state.config.video_max_bitrate == 0 {
                "None".chars().count()
            } else {
                format!("{} kbps", state.config.video_max_bitrate)
                    .chars()
                    .count()
            }
        }
        ConfigFocus::ThreadsInput => {
            if state.config.threads == 0 {
                "Auto".chars().count()
            } else {
                state.config.threads.to_string().chars().count()
            }
        }
        ConfigFocus::MaxWorkersInput => format!("{}", state.config.max_workers).chars().count(),
        ConfigFocus::GopLengthInput => format!("{} frames", state.config.gop_length)
            .chars()
            .count(),
        ConfigFocus::KeyintMinInput => {
            if state.config.keyint_min == 0 {
                "Auto".chars().count()
            } else {
                format!("{} frames", state.config.keyint_min)
                    .chars()
                    .count()
            }
        }
        ConfigFocus::StaticThreshInput => {
            if state.config.static_thresh == 0 {
                "Off".chars().count()
            } else {
                state.config.static_thresh.to_string().chars().count()
            }
        }
        ConfigFocus::MaxIntraRateInput => {
            if state.config.max_intra_rate == 0 {
                "Off".chars().count()
            } else {
                format!("{}%", state.config.max_intra_rate).chars().count()
            }
        }
        ConfigFocus::VaapiBFramesInput => state.config.vaapi_b_frames.chars().count(),
        ConfigFocus::VaapiLoopFilterLevelInput => {
            state.config.vaapi_loop_filter_level.chars().count()
        }
        ConfigFocus::VaapiLoopFilterSharpnessInput => {
            state.config.vaapi_loop_filter_sharpness.chars().count()
        }
        _ => 0, // Not a text input field
    };
}

// Update input mode based on current focus
fn update_input_mode_for_focus(state: &mut AppState) {
    use crate::ui::focus::ConfigFocus;
    use crate::ui::state::InputMode;

    // Set to Editing mode only for text fields that accept free-form text input
    state.config.input_mode = match state.config.focus {
        ConfigFocus::OutputDirectory | ConfigFocus::FilenamePattern => InputMode::Editing,
        _ => InputMode::Normal,
    };
}

// Helper function to set focus and update input mode/cursor (for mouse clicks)
fn set_focus_and_update(state: &mut AppState, new_focus: crate::ui::focus::ConfigFocus) {
    state.config.focus = new_focus;
    reset_cursor_position_for_focus(state);
    update_input_mode_for_focus(state);
}

fn handle_focused_widget_key(key: KeyEvent, state: &mut AppState) {
    match state.config.focus {
        ConfigFocus::ProfileList => {
            match key.code {
                KeyCode::Enter => {
                    // If dropdown is not open, load the currently selected profile immediately
                    if state.config.active_dropdown.is_none() {
                        load_selected_profile(state);
                    }
                }
                KeyCode::Char(' ') => {
                    // Space opens dropdown popup
                    state.config.active_dropdown = Some(ConfigFocus::ProfileList);
                }
                KeyCode::Up => {
                    let selected = state.config.profile_list_state.selected().unwrap_or(0);
                    if selected > 0 {
                        state.config.profile_list_state.select(Some(selected - 1));
                    }
                }
                KeyCode::Down => {
                    let selected = state.config.profile_list_state.selected().unwrap_or(0);
                    let profile_count = get_profile_count(&state.config);
                    if selected + 1 < profile_count {
                        state.config.profile_list_state.select(Some(selected + 1));
                    }
                }
                _ => {}
            }
        }
        ConfigFocus::SaveButton => {
            if matches!(
                key.code,
                KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Enter
            ) {
                if let Some(ref name) = state.config.current_profile_name {
                    // Have a profile loaded - overwrite it
                    save_profile_with_name(state, name.clone());
                } else {
                    // No profile loaded (Custom) - prompt for name
                    state.config.name_input_dialog = Some(String::new());
                }
            }
        }
        ConfigFocus::DeleteButton => {
            let is_ctrl_d = matches!(key.code, KeyCode::Char('d') | KeyCode::Char('D'))
                && key.modifiers.contains(KeyModifiers::CONTROL);
            if is_ctrl_d || matches!(key.code, KeyCode::Enter) {
                // Delete currently selected profile (if it's not a built-in)
                if let Some(ref name) = state.config.current_profile_name {
                    delete_profile(state, name.clone());
                }
            }
        }
        ConfigFocus::OutputDirectory | ConfigFocus::FilenamePattern => {
            // Text input handling with cursor support
            let old_output = state.config.output_dir.clone();
            let old_pattern = state.config.filename_pattern.clone();
            match key.code {
                KeyCode::Char(c) => {
                    if state.config.focus == ConfigFocus::OutputDirectory {
                        let chars: Vec<char> = state.config.output_dir.chars().collect();
                        let pos = state.config.cursor_pos.min(chars.len());
                        let mut new_string: String = chars.iter().take(pos).collect();
                        new_string.push(c);
                        new_string.extend(chars.iter().skip(pos));
                        state.config.output_dir = new_string;
                        state.config.cursor_pos += 1;
                    } else {
                        let pattern = state
                            .config
                            .filename_pattern
                            .get_or_insert_with(String::new);
                        let chars: Vec<char> = pattern.chars().collect();
                        let pos = state.config.cursor_pos.min(chars.len());
                        let mut new_string: String = chars.iter().take(pos).collect();
                        new_string.push(c);
                        new_string.extend(chars.iter().skip(pos));
                        *pattern = new_string;
                        state.config.cursor_pos += 1;
                    }
                }
                KeyCode::Backspace => {
                    // Check for Ctrl+Backspace (delete word before cursor)
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        if state.config.focus == ConfigFocus::OutputDirectory {
                            if state.config.cursor_pos > 0 {
                                let chars: Vec<char> = state.config.output_dir.chars().collect();
                                // Find start of word (skip backwards to whitespace or start)
                                let mut new_pos = state.config.cursor_pos;
                                // Skip trailing whitespace first
                                while new_pos > 0
                                    && chars.get(new_pos - 1).map_or(false, |c| c.is_whitespace())
                                {
                                    new_pos -= 1;
                                }
                                // Skip word characters
                                while new_pos > 0
                                    && chars.get(new_pos - 1).map_or(false, |c| !c.is_whitespace())
                                {
                                    new_pos -= 1;
                                }
                                let mut new_string: String = chars.iter().take(new_pos).collect();
                                new_string.extend(chars.iter().skip(state.config.cursor_pos));
                                state.config.output_dir = new_string;
                                state.config.cursor_pos = new_pos;
                            }
                        } else {
                            if let Some(s) = &mut state.config.filename_pattern {
                                if state.config.cursor_pos > 0 {
                                    let chars: Vec<char> = s.chars().collect();
                                    let mut new_pos = state.config.cursor_pos;
                                    while new_pos > 0
                                        && chars
                                            .get(new_pos - 1)
                                            .map_or(false, |c| c.is_whitespace())
                                    {
                                        new_pos -= 1;
                                    }
                                    while new_pos > 0
                                        && chars
                                            .get(new_pos - 1)
                                            .map_or(false, |c| !c.is_whitespace())
                                    {
                                        new_pos -= 1;
                                    }
                                    let mut new_string: String =
                                        chars.iter().take(new_pos).collect();
                                    new_string.extend(chars.iter().skip(state.config.cursor_pos));
                                    *s = new_string;
                                    state.config.cursor_pos = new_pos;
                                    if s.is_empty() {
                                        state.config.filename_pattern = None;
                                    }
                                }
                            }
                        }
                    } else {
                        // Normal backspace (delete char before cursor)
                        if state.config.focus == ConfigFocus::OutputDirectory {
                            if state.config.cursor_pos > 0 {
                                let chars: Vec<char> = state.config.output_dir.chars().collect();
                                let mut new_string: String =
                                    chars.iter().take(state.config.cursor_pos - 1).collect();
                                new_string.extend(chars.iter().skip(state.config.cursor_pos));
                                state.config.output_dir = new_string;
                                state.config.cursor_pos -= 1;
                            }
                        } else {
                            if let Some(s) = &mut state.config.filename_pattern {
                                if state.config.cursor_pos > 0 {
                                    let chars: Vec<char> = s.chars().collect();
                                    let mut new_string: String =
                                        chars.iter().take(state.config.cursor_pos - 1).collect();
                                    new_string.extend(chars.iter().skip(state.config.cursor_pos));
                                    *s = new_string;
                                    state.config.cursor_pos -= 1;
                                    if s.is_empty() {
                                        state.config.filename_pattern = None;
                                    }
                                }
                            }
                        }
                    }
                }
                KeyCode::Delete => {
                    // Delete character after cursor
                    if state.config.focus == ConfigFocus::OutputDirectory {
                        let chars: Vec<char> = state.config.output_dir.chars().collect();
                        if state.config.cursor_pos < chars.len() {
                            let mut new_string: String =
                                chars.iter().take(state.config.cursor_pos).collect();
                            new_string.extend(chars.iter().skip(state.config.cursor_pos + 1));
                            state.config.output_dir = new_string;
                        }
                    } else {
                        if let Some(s) = &mut state.config.filename_pattern {
                            let chars: Vec<char> = s.chars().collect();
                            if state.config.cursor_pos < chars.len() {
                                let mut new_string: String =
                                    chars.iter().take(state.config.cursor_pos).collect();
                                new_string.extend(chars.iter().skip(state.config.cursor_pos + 1));
                                *s = new_string;
                                if s.is_empty() {
                                    state.config.filename_pattern = None;
                                }
                            }
                        }
                    }
                }
                KeyCode::Left => {
                    if state.config.cursor_pos > 0 {
                        state.config.cursor_pos -= 1;
                    }
                }
                KeyCode::Right => {
                    let max_len = if state.config.focus == ConfigFocus::OutputDirectory {
                        state.config.output_dir.chars().count()
                    } else {
                        state
                            .config
                            .filename_pattern
                            .as_ref()
                            .map(|s| s.chars().count())
                            .unwrap_or(0)
                    };
                    if state.config.cursor_pos < max_len {
                        state.config.cursor_pos += 1;
                    }
                }
                KeyCode::Home => {
                    state.config.cursor_pos = 0;
                }
                KeyCode::End => {
                    state.config.cursor_pos = if state.config.focus == ConfigFocus::OutputDirectory
                    {
                        state.config.output_dir.chars().count()
                    } else {
                        state
                            .config
                            .filename_pattern
                            .as_ref()
                            .map(|s| s.chars().count())
                            .unwrap_or(0)
                    };
                }
                _ => {}
            }
            if state.config.output_dir != old_output || state.config.filename_pattern != old_pattern
            {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::OverwriteCheckbox => {
            if matches!(key.code, KeyCode::Char(' ') | KeyCode::Enter) {
                state.config.overwrite = !state.config.overwrite;
                state.config.is_modified = true;

                // Save overwrite setting to config
                if let Ok(mut config) = crate::config::Config::load() {
                    config.defaults.overwrite = state.config.overwrite;
                    let _ = config.save(); // Ignore errors
                }
            }
        }
        ConfigFocus::ContainerDropdown => {
            // Container extension dropdown (visual dropdown like others)
            let old_selection = state.config.container_dropdown_state.selected();
            match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    state.config.active_dropdown = Some(ConfigFocus::ContainerDropdown);
                }
                KeyCode::Left | KeyCode::Up => {
                    let current = state
                        .config
                        .container_dropdown_state
                        .selected()
                        .unwrap_or(0);
                    if current > 0 {
                        state
                            .config
                            .container_dropdown_state
                            .select(Some(current - 1));
                    } else {
                        state.config.container_dropdown_state.select(Some(3)); // wraparound to last
                    }
                }
                KeyCode::Right | KeyCode::Down => {
                    let current = state
                        .config
                        .container_dropdown_state
                        .selected()
                        .unwrap_or(0);
                    state
                        .config
                        .container_dropdown_state
                        .select(Some((current + 1) % 4));
                }
                _ => {}
            }
            if state.config.container_dropdown_state.selected() != old_selection {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::FpsDropdown => {
            match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    state.config.active_dropdown = Some(ConfigFocus::FpsDropdown);
                }
                KeyCode::Left | KeyCode::Up => {
                    // Cycle to previous FPS option (Left for quick nav, Up for dropdown list)
                    let current = state.config.fps_dropdown_state.selected().unwrap_or(0);
                    let new_idx = if current == 0 { 10 } else { current - 1 }; // 11 options (0-10)
                    state.config.fps_dropdown_state.select(Some(new_idx));
                    state.config.is_modified = true;
                }
                KeyCode::Right | KeyCode::Down => {
                    // Cycle to next FPS option (Right for quick nav, Down for dropdown list)
                    let current = state.config.fps_dropdown_state.selected().unwrap_or(0);
                    let new_idx = if current >= 10 { 0 } else { current + 1 }; // 11 options (0-10)
                    state.config.fps_dropdown_state.select(Some(new_idx));
                    state.config.is_modified = true;
                }
                _ => {}
            }
        }
        ConfigFocus::ResolutionDropdown => {
            match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    state.config.active_dropdown = Some(ConfigFocus::ResolutionDropdown);
                }
                KeyCode::Left | KeyCode::Up => {
                    // Cycle to previous resolution option (Left for quick nav, Up for dropdown list)
                    let current = state
                        .config
                        .resolution_dropdown_state
                        .selected()
                        .unwrap_or(0);
                    let new_idx = if current == 0 { 6 } else { current - 1 }; // 7 options (0-6)
                    state.config.resolution_dropdown_state.select(Some(new_idx));
                    state.config.is_modified = true;
                }
                KeyCode::Right | KeyCode::Down => {
                    // Cycle to next resolution option (Right for quick nav, Down for dropdown list)
                    let current = state
                        .config
                        .resolution_dropdown_state
                        .selected()
                        .unwrap_or(0);
                    let new_idx = if current >= 6 { 0 } else { current + 1 }; // 7 options (0-6)
                    state.config.resolution_dropdown_state.select(Some(new_idx));
                    state.config.is_modified = true;
                }
                _ => {}
            }
        }
        ConfigFocus::CrfSlider => {
            let old_value = state.config.crf;
            match key.code {
                KeyCode::Left => {
                    if state.config.crf > 0 {
                        state.config.crf -= 1;
                    }
                }
                KeyCode::Right => {
                    if state.config.crf < 63 {
                        state.config.crf += 1;
                    }
                }
                KeyCode::Home => state.config.crf = 0,
                KeyCode::End => state.config.crf = 63,
                _ => {}
            }
            if state.config.crf != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::CpuUsedSlider => {
            let old_value = state.config.cpu_used;
            match key.code {
                KeyCode::Left => {
                    if state.config.cpu_used > 0 {
                        state.config.cpu_used -= 1;
                    }
                }
                KeyCode::Right => {
                    if state.config.cpu_used < 8 {
                        state.config.cpu_used += 1;
                    }
                }
                KeyCode::Home => state.config.cpu_used = 0,
                KeyCode::End => state.config.cpu_used = 8,
                _ => {}
            }
            if state.config.cpu_used != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::AudioBitrateSlider => {
            let old_value = state.config.audio_bitrate;
            match key.code {
                KeyCode::Left => {
                    if state.config.audio_bitrate > 32 {
                        state.config.audio_bitrate = state.config.audio_bitrate.saturating_sub(8);
                    }
                }
                KeyCode::Right => {
                    if state.config.audio_bitrate < 512 {
                        state.config.audio_bitrate = (state.config.audio_bitrate + 8).min(512);
                    }
                }
                KeyCode::Home => state.config.audio_bitrate = 32,
                KeyCode::End => state.config.audio_bitrate = 512,
                _ => {}
            }
            if state.config.audio_bitrate != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::ForceStereoCheckbox => {
            if matches!(key.code, KeyCode::Char(' ') | KeyCode::Enter) {
                state.config.force_stereo = !state.config.force_stereo;
                state.config.is_modified = true;
            }
        }
        ConfigFocus::TwoPassCheckbox => {
            if matches!(key.code, KeyCode::Char(' ') | KeyCode::Enter) {
                state.config.two_pass = !state.config.two_pass;
                state.config.is_modified = true;
            }
        }
        ConfigFocus::RowMtCheckbox => {
            if matches!(key.code, KeyCode::Char(' ') | KeyCode::Enter) {
                state.config.row_mt = !state.config.row_mt;
                state.config.is_modified = true;
            }
        }
        ConfigFocus::ProfileDropdown => {
            let old_selection = state.config.profile_dropdown_state.selected();
            match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    // Open dropdown popup
                    state.config.active_dropdown = Some(ConfigFocus::ProfileDropdown);
                }
                KeyCode::Up => {
                    let selected = state.config.profile_dropdown_state.selected().unwrap_or(0);
                    if selected > 0 {
                        state
                            .config
                            .profile_dropdown_state
                            .select(Some(selected - 1));
                    }
                }
                KeyCode::Down => {
                    let selected = state.config.profile_dropdown_state.selected().unwrap_or(0);
                    if selected < 3 {
                        // 4 profiles
                        state
                            .config
                            .profile_dropdown_state
                            .select(Some(selected + 1));
                    }
                }
                _ => {}
            }
            if state.config.profile_dropdown_state.selected() != old_selection {
                state.config.is_modified = true;
            }
        }
        // Per-pass CPU-used sliders
        ConfigFocus::CpuUsedPass1Slider => {
            let old_value = state.config.cpu_used_pass1;
            match key.code {
                KeyCode::Left => {
                    if state.config.cpu_used_pass1 > 0 {
                        state.config.cpu_used_pass1 -= 1;
                    }
                }
                KeyCode::Right => {
                    if state.config.cpu_used_pass1 < 8 {
                        state.config.cpu_used_pass1 += 1;
                    }
                }
                KeyCode::Home => state.config.cpu_used_pass1 = 0,
                KeyCode::End => state.config.cpu_used_pass1 = 8,
                _ => {}
            }
            if state.config.cpu_used_pass1 != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::CpuUsedPass2Slider => {
            let old_value = state.config.cpu_used_pass2;
            match key.code {
                KeyCode::Left => {
                    if state.config.cpu_used_pass2 > 0 {
                        state.config.cpu_used_pass2 -= 1;
                    }
                }
                KeyCode::Right => {
                    if state.config.cpu_used_pass2 < 8 {
                        state.config.cpu_used_pass2 += 1;
                    }
                }
                KeyCode::Home => state.config.cpu_used_pass2 = 0,
                KeyCode::End => state.config.cpu_used_pass2 = 8,
                _ => {}
            }
            if state.config.cpu_used_pass2 != old_value {
                state.config.is_modified = true;
            }
        }
        // Parallelism sliders
        ConfigFocus::TileColumnsSlider => {
            let old_value = state.config.tile_columns;
            match key.code {
                KeyCode::Left => {
                    if state.config.tile_columns > 0 {
                        state.config.tile_columns -= 1;
                    }
                }
                KeyCode::Right => {
                    if state.config.tile_columns < 6 {
                        state.config.tile_columns += 1;
                    }
                }
                KeyCode::Home => state.config.tile_columns = 0,
                KeyCode::End => state.config.tile_columns = 6,
                _ => {}
            }
            if state.config.tile_columns != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::TileRowsSlider => {
            let old_value = state.config.tile_rows;
            match key.code {
                KeyCode::Left => {
                    if state.config.tile_rows > 0 {
                        state.config.tile_rows -= 1;
                    }
                }
                KeyCode::Right => {
                    if state.config.tile_rows < 6 {
                        state.config.tile_rows += 1;
                    }
                }
                KeyCode::Home => state.config.tile_rows = 0,
                KeyCode::End => state.config.tile_rows = 6,
                _ => {}
            }
            if state.config.tile_rows != old_value {
                state.config.is_modified = true;
            }
        }
        // GOP & keyframes sliders
        ConfigFocus::LagInFramesSlider => {
            let old_value = state.config.lag_in_frames;
            match key.code {
                KeyCode::Left => {
                    if state.config.lag_in_frames > 0 {
                        state.config.lag_in_frames -= 1;
                    }
                }
                KeyCode::Right => {
                    if state.config.lag_in_frames < 25 {
                        state.config.lag_in_frames += 1;
                    }
                }
                KeyCode::Home => state.config.lag_in_frames = 0,
                KeyCode::End => state.config.lag_in_frames = 25,
                _ => {}
            }
            if state.config.lag_in_frames != old_value {
                state.config.is_modified = true;
            }
        }
        // ARNR sliders
        ConfigFocus::ArnrMaxFramesSlider => {
            let old_value = state.config.arnr_max_frames;
            match key.code {
                KeyCode::Left => {
                    if state.config.arnr_max_frames > 0 {
                        state.config.arnr_max_frames -= 1;
                    }
                }
                KeyCode::Right => {
                    if state.config.arnr_max_frames < 15 {
                        state.config.arnr_max_frames += 1;
                    }
                }
                KeyCode::Home => state.config.arnr_max_frames = 0,
                KeyCode::End => state.config.arnr_max_frames = 15,
                _ => {}
            }
            if state.config.arnr_max_frames != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::ArnrStrengthSlider => {
            let old_value = state.config.arnr_strength;
            match key.code {
                KeyCode::Left => {
                    if state.config.arnr_strength > 0 {
                        state.config.arnr_strength -= 1;
                    }
                }
                KeyCode::Right => {
                    if state.config.arnr_strength < 6 {
                        state.config.arnr_strength += 1;
                    }
                }
                KeyCode::Home => state.config.arnr_strength = 0,
                KeyCode::End => state.config.arnr_strength = 6,
                _ => {}
            }
            if state.config.arnr_strength != old_value {
                state.config.is_modified = true;
            }
        }
        // Advanced tuning sliders
        ConfigFocus::SharpnessSlider => {
            let old_value = state.config.sharpness;
            match key.code {
                KeyCode::Left => {
                    if state.config.sharpness > -1 {
                        state.config.sharpness -= 1;
                    }
                }
                KeyCode::Right => {
                    if state.config.sharpness < 7 {
                        state.config.sharpness += 1;
                    }
                }
                KeyCode::Home => state.config.sharpness = -1,
                KeyCode::End => state.config.sharpness = 7,
                _ => {}
            }
            if state.config.sharpness != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::NoiseSensitivitySlider => {
            let old_value = state.config.noise_sensitivity;
            match key.code {
                KeyCode::Left => {
                    if state.config.noise_sensitivity > 0 {
                        state.config.noise_sensitivity -= 1;
                    }
                }
                KeyCode::Right => {
                    if state.config.noise_sensitivity < 6 {
                        state.config.noise_sensitivity += 1;
                    }
                }
                KeyCode::Home => state.config.noise_sensitivity = 0,
                KeyCode::End => state.config.noise_sensitivity = 6,
                _ => {}
            }
            if state.config.noise_sensitivity != old_value {
                state.config.is_modified = true;
            }
        }
        // Checkboxes
        ConfigFocus::FrameParallelCheckbox => {
            if matches!(key.code, KeyCode::Char(' ') | KeyCode::Enter) {
                state.config.frame_parallel = !state.config.frame_parallel;
                state.config.is_modified = true;
            }
        }
        ConfigFocus::FixedGopCheckbox => {
            if matches!(key.code, KeyCode::Char(' ') | KeyCode::Enter) {
                state.config.fixed_gop = !state.config.fixed_gop;
                state.config.is_modified = true;
            }
        }
        ConfigFocus::AutoAltRefCheckbox => {
            if matches!(key.code, KeyCode::Char(' ') | KeyCode::Enter) {
                state.config.auto_alt_ref = !state.config.auto_alt_ref;
                state.config.is_modified = true;
            }
        }
        ConfigFocus::EnableTplCheckbox => {
            if matches!(key.code, KeyCode::Char(' ') | KeyCode::Enter) {
                state.config.enable_tpl = !state.config.enable_tpl;
                state.config.is_modified = true;
            }
        }
        // Dropdowns - open popup on Enter/Space, navigate with Left/Right
        ConfigFocus::RateControlMode => {
            if state.config.use_hardware_encoding {
                // Hardware mode: CQP only (no cycling needed)
                // ICQ/VBR/CBR removed due to Arc driver issues
                state.config.vaapi_rc_mode = "1".to_string(); // Always CQP
            } else {
                // Software mode: original behavior
                use crate::ui::state::RateControlMode;
                let old_mode = state.config.rate_control_mode;
                match key.code {
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        state.config.active_dropdown = Some(ConfigFocus::RateControlMode);
                    }
                    KeyCode::Left => {
                        // Cycle left through rate control modes (4 modes: CQ, CQCap, TwoPassVBR, CBR)
                        state.config.rate_control_mode = match state.config.rate_control_mode {
                            RateControlMode::CQ => RateControlMode::CBR,
                            RateControlMode::CQCap => RateControlMode::CQ,
                            RateControlMode::TwoPassVBR => RateControlMode::CQCap,
                            RateControlMode::CBR => RateControlMode::TwoPassVBR,
                        };
                    }
                    KeyCode::Right => {
                        // Cycle right through rate control modes
                        state.config.rate_control_mode = match state.config.rate_control_mode {
                            RateControlMode::CQ => RateControlMode::CQCap,
                            RateControlMode::CQCap => RateControlMode::TwoPassVBR,
                            RateControlMode::TwoPassVBR => RateControlMode::CBR,
                            RateControlMode::CBR => RateControlMode::CQ,
                        };
                    }
                    _ => {}
                }
                if state.config.rate_control_mode != old_mode {
                    state.config.is_modified = true;
                }
            }
        }
        ConfigFocus::QualityMode => {
            let old_selection = state.config.quality_mode_state.selected();
            match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    state.config.active_dropdown = Some(ConfigFocus::QualityMode);
                }
                KeyCode::Up => {
                    let selected = state.config.quality_mode_state.selected().unwrap_or(0);
                    if selected > 0 {
                        state.config.quality_mode_state.select(Some(selected - 1));
                    }
                }
                KeyCode::Down => {
                    let selected = state.config.quality_mode_state.selected().unwrap_or(0);
                    if selected < 2 {
                        // 3 quality modes
                        state.config.quality_mode_state.select(Some(selected + 1));
                    }
                }
                _ => {}
            }
            if state.config.quality_mode_state.selected() != old_selection {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::PixFmtDropdown => {
            let old_selection = state.config.pix_fmt_state.selected();
            match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    state.config.active_dropdown = Some(ConfigFocus::PixFmtDropdown);
                }
                KeyCode::Up => {
                    let selected = state.config.pix_fmt_state.selected().unwrap_or(0);
                    if selected > 0 {
                        state.config.pix_fmt_state.select(Some(selected - 1));
                    }
                }
                KeyCode::Down => {
                    let selected = state.config.pix_fmt_state.selected().unwrap_or(0);
                    if selected < 1 {
                        // 2 pixel formats
                        state.config.pix_fmt_state.select(Some(selected + 1));
                    }
                }
                _ => {}
            }
            if state.config.pix_fmt_state.selected() != old_selection {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::AqModeDropdown => {
            let old_selection = state.config.aq_mode_state.selected();
            match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    state.config.active_dropdown = Some(ConfigFocus::AqModeDropdown);
                }
                KeyCode::Up => {
                    let selected = state.config.aq_mode_state.selected().unwrap_or(0);
                    if selected > 0 {
                        state.config.aq_mode_state.select(Some(selected - 1));
                    }
                }
                KeyCode::Down => {
                    let selected = state.config.aq_mode_state.selected().unwrap_or(0);
                    if selected < 5 {
                        // 6 AQ modes
                        state.config.aq_mode_state.select(Some(selected + 1));
                    }
                }
                _ => {}
            }
            if state.config.aq_mode_state.selected() != old_selection {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::ArnrTypeDropdown => {
            let old_selection = state.config.arnr_type_state.selected();
            match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    state.config.active_dropdown = Some(ConfigFocus::ArnrTypeDropdown);
                }
                KeyCode::Up => {
                    let selected = state.config.arnr_type_state.selected().unwrap_or(0);
                    if selected > 0 {
                        state.config.arnr_type_state.select(Some(selected - 1));
                    }
                }
                KeyCode::Down => {
                    let selected = state.config.arnr_type_state.selected().unwrap_or(0);
                    if selected < 3 {
                        // 4 ARNR types
                        state.config.arnr_type_state.select(Some(selected + 1));
                    }
                }
                _ => {}
            }
            if state.config.arnr_type_state.selected() != old_selection {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::TuneContentDropdown => {
            let old_selection = state.config.tune_content_state.selected();
            match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    state.config.active_dropdown = Some(ConfigFocus::TuneContentDropdown);
                }
                KeyCode::Up => {
                    let selected = state.config.tune_content_state.selected().unwrap_or(0);
                    if selected > 0 {
                        state.config.tune_content_state.select(Some(selected - 1));
                    }
                }
                KeyCode::Down => {
                    let selected = state.config.tune_content_state.selected().unwrap_or(0);
                    if selected < 2 {
                        // 3 tune content modes
                        state.config.tune_content_state.select(Some(selected + 1));
                    }
                }
                _ => {}
            }
            if state.config.tune_content_state.selected() != old_selection {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::ColorspaceDropdown => {
            let old_selection = state.config.colorspace_state.selected();
            match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    state.config.active_dropdown = Some(ConfigFocus::ColorspaceDropdown);
                }
                KeyCode::Up => {
                    let selected = state.config.colorspace_state.selected().unwrap_or(0);
                    if selected > 0 {
                        state.config.colorspace_state.select(Some(selected - 1));
                    }
                }
                KeyCode::Down => {
                    let selected = state.config.colorspace_state.selected().unwrap_or(0);
                    if selected < 4 {
                        // 5 colorspaces
                        state.config.colorspace_state.select(Some(selected + 1));
                    }
                }
                _ => {}
            }
            if state.config.colorspace_state.selected() != old_selection {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::ColorPrimariesDropdown => {
            let old_selection = state.config.color_primaries_state.selected();
            match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    state.config.active_dropdown = Some(ConfigFocus::ColorPrimariesDropdown);
                }
                KeyCode::Up => {
                    let selected = state.config.color_primaries_state.selected().unwrap_or(0);
                    if selected > 0 {
                        state
                            .config
                            .color_primaries_state
                            .select(Some(selected - 1));
                    }
                }
                KeyCode::Down => {
                    let selected = state.config.color_primaries_state.selected().unwrap_or(0);
                    if selected < 4 {
                        // 5 primaries
                        state
                            .config
                            .color_primaries_state
                            .select(Some(selected + 1));
                    }
                }
                _ => {}
            }
            if state.config.color_primaries_state.selected() != old_selection {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::ColorTrcDropdown => {
            let old_selection = state.config.color_trc_state.selected();
            match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    state.config.active_dropdown = Some(ConfigFocus::ColorTrcDropdown);
                }
                KeyCode::Up => {
                    let selected = state.config.color_trc_state.selected().unwrap_or(0);
                    if selected > 0 {
                        state.config.color_trc_state.select(Some(selected - 1));
                    }
                }
                KeyCode::Down => {
                    let selected = state.config.color_trc_state.selected().unwrap_or(0);
                    if selected < 4 {
                        // 5 transfer characteristics
                        state.config.color_trc_state.select(Some(selected + 1));
                    }
                }
                _ => {}
            }
            if state.config.color_trc_state.selected() != old_selection {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::ColorRangeDropdown => {
            let old_selection = state.config.color_range_state.selected();
            match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    state.config.active_dropdown = Some(ConfigFocus::ColorRangeDropdown);
                }
                KeyCode::Up => {
                    let selected = state.config.color_range_state.selected().unwrap_or(0);
                    if selected > 0 {
                        state.config.color_range_state.select(Some(selected - 1));
                    }
                }
                KeyCode::Down => {
                    let selected = state.config.color_range_state.selected().unwrap_or(0);
                    if selected < 2 {
                        // 3 color ranges
                        state.config.color_range_state.select(Some(selected + 1));
                    }
                }
                _ => {}
            }
            if state.config.color_range_state.selected() != old_selection {
                state.config.is_modified = true;
            }
        }
        // Numeric inputs (allow digit entry and backspace)
        ConfigFocus::VideoTargetBitrateInput => {
            let old_value = state.config.video_target_bitrate;
            match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let digit = c.to_digit(10).unwrap();
                    state.config.video_target_bitrate = state
                        .config
                        .video_target_bitrate
                        .saturating_mul(10)
                        .saturating_add(digit);
                }
                KeyCode::Backspace => {
                    state.config.video_target_bitrate /= 10;
                }
                KeyCode::Char('0') if state.config.video_target_bitrate == 0 => {
                    // Allow setting to 0
                    state.config.video_target_bitrate = 0;
                }
                _ => {}
            }
            if state.config.video_target_bitrate != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::VideoMinBitrateInput => {
            let old_value = state.config.video_min_bitrate;
            match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let digit = c.to_digit(10).unwrap();
                    state.config.video_min_bitrate = state
                        .config
                        .video_min_bitrate
                        .saturating_mul(10)
                        .saturating_add(digit);
                }
                KeyCode::Backspace => {
                    state.config.video_min_bitrate /= 10;
                }
                _ => {}
            }
            if state.config.video_min_bitrate != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::VideoMaxBitrateInput => {
            let old_value = state.config.video_max_bitrate;
            match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let digit = c.to_digit(10).unwrap();
                    state.config.video_max_bitrate = state
                        .config
                        .video_max_bitrate
                        .saturating_mul(10)
                        .saturating_add(digit);
                }
                KeyCode::Backspace => {
                    state.config.video_max_bitrate /= 10;
                }
                _ => {}
            }
            if state.config.video_max_bitrate != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::VideoBufsizeInput => {
            let old_value = state.config.video_bufsize;
            match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let digit = c.to_digit(10).unwrap();
                    state.config.video_bufsize = state
                        .config
                        .video_bufsize
                        .saturating_mul(10)
                        .saturating_add(digit);
                }
                KeyCode::Backspace => {
                    state.config.video_bufsize /= 10;
                }
                _ => {}
            }
            if state.config.video_bufsize != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::UndershootPctInput => {
            let old_value = state.config.undershoot_pct;
            match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let digit = c.to_digit(10).unwrap() as i32;
                    let new_val = state
                        .config
                        .undershoot_pct
                        .saturating_mul(10)
                        .saturating_add(digit);
                    if new_val <= 100 {
                        state.config.undershoot_pct = new_val;
                    }
                }
                KeyCode::Char('-') if state.config.undershoot_pct >= 0 => {
                    state.config.undershoot_pct = -1; // Set to auto
                }
                KeyCode::Backspace => {
                    if state.config.undershoot_pct == -1 {
                        state.config.undershoot_pct = 0;
                    } else {
                        state.config.undershoot_pct /= 10;
                    }
                }
                _ => {}
            }
            if state.config.undershoot_pct != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::OvershootPctInput => {
            let old_value = state.config.overshoot_pct;
            match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let digit = c.to_digit(10).unwrap() as i32;
                    let new_val = state
                        .config
                        .overshoot_pct
                        .saturating_mul(10)
                        .saturating_add(digit);
                    if new_val <= 1000 {
                        state.config.overshoot_pct = new_val;
                    }
                }
                KeyCode::Char('-') if state.config.overshoot_pct >= 0 => {
                    state.config.overshoot_pct = -1; // Set to auto
                }
                KeyCode::Backspace => {
                    if state.config.overshoot_pct == -1 {
                        state.config.overshoot_pct = 0;
                    } else {
                        state.config.overshoot_pct /= 10;
                    }
                }
                _ => {}
            }
            if state.config.overshoot_pct != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::ThreadsInput => {
            let old_value = state.config.threads;
            match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let digit = c.to_digit(10).unwrap();
                    state.config.threads = state
                        .config
                        .threads
                        .saturating_mul(10)
                        .saturating_add(digit);
                }
                KeyCode::Backspace => {
                    state.config.threads /= 10;
                }
                _ => {}
            }
            if state.config.threads != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::MaxWorkersInput => {
            let old_value = state.config.max_workers;
            match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let digit = c.to_digit(10).unwrap();
                    let new_val = state
                        .config
                        .max_workers
                        .saturating_mul(10)
                        .saturating_add(digit);
                    // Reasonable limit: 1-16 workers
                    if new_val >= 1 && new_val <= 16 {
                        state.config.max_workers = new_val;
                    }
                }
                KeyCode::Backspace => {
                    state.config.max_workers /= 10;
                    // Ensure minimum of 1
                    if state.config.max_workers < 1 {
                        state.config.max_workers = 1;
                    }
                }
                _ => {}
            }
            if state.config.max_workers != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::GopLengthInput => {
            let old_value = state.config.gop_length;
            match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let digit = c.to_digit(10).unwrap();
                    state.config.gop_length = state
                        .config
                        .gop_length
                        .saturating_mul(10)
                        .saturating_add(digit);
                }
                KeyCode::Backspace => {
                    state.config.gop_length /= 10;
                }
                _ => {}
            }
            if state.config.gop_length != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::KeyintMinInput => {
            let old_value = state.config.keyint_min;
            match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let digit = c.to_digit(10).unwrap();
                    state.config.keyint_min = state
                        .config
                        .keyint_min
                        .saturating_mul(10)
                        .saturating_add(digit);
                }
                KeyCode::Backspace => {
                    state.config.keyint_min /= 10;
                }
                _ => {}
            }
            if state.config.keyint_min != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::StaticThreshInput => {
            let old_value = state.config.static_thresh;
            match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let digit = c.to_digit(10).unwrap();
                    state.config.static_thresh = state
                        .config
                        .static_thresh
                        .saturating_mul(10)
                        .saturating_add(digit);
                }
                KeyCode::Backspace => {
                    state.config.static_thresh /= 10;
                }
                _ => {}
            }
            if state.config.static_thresh != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::MaxIntraRateInput => {
            let old_value = state.config.max_intra_rate;
            match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let digit = c.to_digit(10).unwrap();
                    state.config.max_intra_rate = state
                        .config
                        .max_intra_rate
                        .saturating_mul(10)
                        .saturating_add(digit);
                }
                KeyCode::Backspace => {
                    state.config.max_intra_rate /= 10;
                }
                _ => {}
            }
            if state.config.max_intra_rate != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::VaapiBFramesInput => {
            let old_value = state.config.vaapi_b_frames.clone();
            match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let new_val = state.config.vaapi_b_frames.clone() + &c.to_string();
                    if let Ok(num) = new_val.parse::<u32>() {
                        if num <= 4 {
                            state.config.vaapi_b_frames = new_val;
                        }
                    }
                }
                KeyCode::Backspace => {
                    if !state.config.vaapi_b_frames.is_empty() {
                        state.config.vaapi_b_frames.pop();
                        if state.config.vaapi_b_frames.is_empty() {
                            state.config.vaapi_b_frames = "0".to_string();
                        }
                    }
                }
                _ => {}
            }
            if state.config.vaapi_b_frames != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::VaapiLoopFilterLevelInput => {
            let old_value = state.config.vaapi_loop_filter_level.clone();
            match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let new_val = state.config.vaapi_loop_filter_level.clone() + &c.to_string();
                    if let Ok(num) = new_val.parse::<u32>() {
                        if num <= 63 {
                            state.config.vaapi_loop_filter_level = new_val;
                        }
                    }
                }
                KeyCode::Backspace => {
                    if !state.config.vaapi_loop_filter_level.is_empty() {
                        state.config.vaapi_loop_filter_level.pop();
                        if state.config.vaapi_loop_filter_level.is_empty() {
                            state.config.vaapi_loop_filter_level = "16".to_string();
                        }
                    }
                }
                _ => {}
            }
            if state.config.vaapi_loop_filter_level != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::VaapiLoopFilterSharpnessInput => {
            let old_value = state.config.vaapi_loop_filter_sharpness.clone();
            match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let new_val = state.config.vaapi_loop_filter_sharpness.clone() + &c.to_string();
                    if let Ok(num) = new_val.parse::<u32>() {
                        if num <= 15 {
                            state.config.vaapi_loop_filter_sharpness = new_val;
                        }
                    }
                }
                KeyCode::Backspace => {
                    if !state.config.vaapi_loop_filter_sharpness.is_empty() {
                        state.config.vaapi_loop_filter_sharpness.pop();
                        if state.config.vaapi_loop_filter_sharpness.is_empty() {
                            state.config.vaapi_loop_filter_sharpness = "4".to_string();
                        }
                    }
                }
                _ => {}
            }
            if state.config.vaapi_loop_filter_sharpness != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::AudioCodec => {
            let old_selection = state.config.codec_list_state.selected();
            match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    // Open dropdown popup
                    state.config.active_dropdown = Some(ConfigFocus::AudioCodec);
                }
                KeyCode::Up => {
                    let selected = state.config.codec_list_state.selected().unwrap_or(0);
                    if selected > 0 {
                        state.config.codec_list_state.select(Some(selected - 1));
                    }
                }
                KeyCode::Down => {
                    let selected = state.config.codec_list_state.selected().unwrap_or(0);
                    if selected < 3 {
                        // 4 codecs
                        state.config.codec_list_state.select(Some(selected + 1));
                    }
                }
                _ => {}
            }
            if state.config.codec_list_state.selected() != old_selection {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::HardwareEncodingCheckbox => {
            if matches!(key.code, KeyCode::Char(' ') | KeyCode::Enter) {
                // Toggle hardware encoding with pre-flight check
                handle_hw_encoding_toggle(state);
            }
        }
        ConfigFocus::QsvGlobalQualitySlider => {
            let old_value = state.config.qsv_global_quality;
            match key.code {
                KeyCode::Left => {
                    if state.config.qsv_global_quality > 1 {
                        state.config.qsv_global_quality =
                            state.config.qsv_global_quality.saturating_sub(1);
                    }
                }
                KeyCode::Right => {
                    if state.config.qsv_global_quality < 255 {
                        state.config.qsv_global_quality =
                            (state.config.qsv_global_quality + 1).min(255);
                    }
                }
                KeyCode::Home => state.config.qsv_global_quality = 1,
                KeyCode::End => state.config.qsv_global_quality = 255,
                _ => {}
            }
            if state.config.qsv_global_quality != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::VaapiCompressionLevelSlider => {
            let old_value = state.config.vaapi_compression_level.clone();
            let current_val = state
                .config
                .vaapi_compression_level
                .parse::<u32>()
                .unwrap_or(4);
            match key.code {
                KeyCode::Left => {
                    if current_val > 0 {
                        state.config.vaapi_compression_level = (current_val - 1).to_string();
                    }
                }
                KeyCode::Right => {
                    if current_val < 7 {
                        state.config.vaapi_compression_level = (current_val + 1).to_string();
                    }
                }
                KeyCode::Home => state.config.vaapi_compression_level = "0".to_string(),
                KeyCode::End => state.config.vaapi_compression_level = "7".to_string(),
                _ => {}
            }
            if state.config.vaapi_compression_level != old_value {
                state.config.is_modified = true;
            }
        }
    }
}

fn handle_hw_encoding_toggle(state: &mut AppState) {
    use std::time::Instant;

    if !cfg!(target_os = "linux") {
        state.config.status_message = Some(("Linux only".into(), Instant::now()));
        return;
    }

    if state.config.use_hardware_encoding {
        // Turning off
        state.config.use_hardware_encoding = false;
        state.config.status_message = Some(("HW encoding disabled".into(), Instant::now()));
    } else {
        // Turning on - run pre-flight
        let result = crate::engine::hardware::run_preflight();
        state.config.hw_encoding_available = Some(result.available);
        state.config.hw_availability_message = result.error_message.clone();

        if result.available {
            state.config.use_hardware_encoding = true;
            state.dashboard.gpu_model = result.gpu_model;
            state.dashboard.gpu_available = crate::engine::hardware::xpu_smi_available();
            // Initialize hardware encoding parameters
            state.config.vaapi_rc_mode = "1".to_string(); // CQP mode - only supported mode
            state.config.status_message = Some(("QSV enabled".into(), Instant::now()));
        } else {
            state.config.status_message = Some((
                result.error_message.unwrap_or("Unavailable".into()),
                Instant::now(),
            ));
        }
    }
    state.config.is_modified = true;
}

pub(super) fn handle_config_mouse(mouse: MouseEvent, state: &mut AppState) {
    use crate::ui::focus::ConfigFocus;
    use ratatui::layout::Rect;

    let config = &mut state.config;

    // Only handle left clicks for now
    if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
        // Helper function to check if point is in rect
        let is_in_rect = |x: u16, y: u16, rect: Rect| -> bool {
            x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
        };

        // If a dropdown is active, handle popup interactions
        if let Some(active) = config.active_dropdown {
            use crate::ui::ConfigScreen;

            // Get the popup area and item count using the same calculation as rendering
            let (popup_area, item_count, list_state) = match active {
                ConfigFocus::ProfileList => {
                    let item_count = get_profile_count(config);
                    let trigger = config.profile_list_area.unwrap_or(Rect::default());
                    let popup =
                        ConfigScreen::calculate_popup_area(trigger, item_count, state.viewport);
                    (popup, item_count, &mut config.profile_list_state)
                }
                ConfigFocus::QualityMode => {
                    let trigger = config.quality_mode_area.unwrap_or(Rect::default());
                    let popup = ConfigScreen::calculate_popup_area(trigger, 3, state.viewport);
                    (popup, 3, &mut config.quality_mode_state)
                }
                ConfigFocus::ProfileDropdown => {
                    let trigger = config.vp9_profile_list_area.unwrap_or(Rect::default());
                    let popup = ConfigScreen::calculate_popup_area(trigger, 4, state.viewport);
                    (popup, 4, &mut config.profile_dropdown_state)
                }
                ConfigFocus::PixFmtDropdown => {
                    let trigger = config.pix_fmt_area.unwrap_or(Rect::default());
                    let popup = ConfigScreen::calculate_popup_area(trigger, 2, state.viewport);
                    (popup, 2, &mut config.pix_fmt_state)
                }
                ConfigFocus::AqModeDropdown => {
                    let trigger = config.aq_mode_area.unwrap_or(Rect::default());
                    let popup = ConfigScreen::calculate_popup_area(trigger, 6, state.viewport);
                    (popup, 6, &mut config.aq_mode_state)
                }
                ConfigFocus::TuneContentDropdown => {
                    let trigger = config.tune_content_area.unwrap_or(Rect::default());
                    let popup = ConfigScreen::calculate_popup_area(trigger, 3, state.viewport);
                    (popup, 3, &mut config.tune_content_state)
                }
                ConfigFocus::AudioCodec => {
                    let trigger = config.codec_list_area.unwrap_or(Rect::default());
                    let popup = ConfigScreen::calculate_popup_area(trigger, 4, state.viewport);
                    (popup, 4, &mut config.codec_list_state)
                }
                ConfigFocus::ArnrTypeDropdown => {
                    let trigger = config.arnr_type_area.unwrap_or(Rect::default());
                    let popup = ConfigScreen::calculate_popup_area(trigger, 4, state.viewport);
                    (popup, 4, &mut config.arnr_type_state)
                }
                ConfigFocus::ColorspaceDropdown => {
                    let trigger = config.colorspace_area.unwrap_or(Rect::default());
                    let popup = ConfigScreen::calculate_popup_area(trigger, 5, state.viewport);
                    (popup, 5, &mut config.colorspace_state)
                }
                ConfigFocus::ColorPrimariesDropdown => {
                    let trigger = config.color_primaries_area.unwrap_or(Rect::default());
                    let popup = ConfigScreen::calculate_popup_area(trigger, 5, state.viewport);
                    (popup, 5, &mut config.color_primaries_state)
                }
                ConfigFocus::ColorTrcDropdown => {
                    let trigger = config.color_trc_area.unwrap_or(Rect::default());
                    let popup = ConfigScreen::calculate_popup_area(trigger, 5, state.viewport);
                    (popup, 5, &mut config.color_trc_state)
                }
                ConfigFocus::ColorRangeDropdown => {
                    let trigger = config.color_range_area.unwrap_or(Rect::default());
                    let popup = ConfigScreen::calculate_popup_area(trigger, 3, state.viewport);
                    (popup, 3, &mut config.color_range_state)
                }
                ConfigFocus::FpsDropdown => {
                    let trigger = config.fps_area.unwrap_or(Rect::default());
                    let popup = ConfigScreen::calculate_popup_area(trigger, 11, state.viewport);
                    (popup, 11, &mut config.fps_dropdown_state)
                }
                ConfigFocus::ResolutionDropdown => {
                    let trigger = config.scale_width_area.unwrap_or(Rect::default());
                    let popup = ConfigScreen::calculate_popup_area(trigger, 7, state.viewport);
                    (popup, 7, &mut config.resolution_dropdown_state)
                }
                _ => {
                    config.active_dropdown = None;
                    return;
                }
            };

            // Check if click is inside popup
            if is_in_rect(mouse.column, mouse.row, popup_area) {
                // Calculate which item was clicked (accounting for border)
                if mouse.row > popup_area.y
                    && mouse.row < popup_area.y + popup_area.height.saturating_sub(1)
                {
                    let item_index =
                        (mouse.row.saturating_sub(popup_area.y).saturating_sub(1)) as usize;
                    // Bounds check before selecting
                    if item_index < item_count {
                        list_state.select(Some(item_index));
                    }
                }

                // Close popup after selection
                let was_profile_list = active == ConfigFocus::ProfileList;
                config.active_dropdown = None;

                // If ProfileList dropdown, load the selected profile after closing
                if was_profile_list {
                    load_selected_profile(state);
                }

                return;
            } else {
                // Click outside popup - close it without selecting
                config.active_dropdown = None;
                return;
            }
        }

        // Check checkboxes
        if let Some(area) = config.overwrite_checkbox_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::OverwriteCheckbox;
                config.overwrite = !config.overwrite;

                // Save overwrite setting to config
                if let Ok(mut global_config) = crate::config::Config::load() {
                    global_config.defaults.overwrite = config.overwrite;
                    let _ = global_config.save(); // Ignore errors
                }

                return;
            }
        }

        if let Some(area) = config.two_pass_checkbox_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::TwoPassCheckbox;
                config.two_pass = !config.two_pass;
                return;
            }
        }

        if let Some(area) = config.row_mt_checkbox_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::RowMtCheckbox;
                config.row_mt = !config.row_mt;
                return;
            }
        }

        // Check buttons
        if let Some(area) = config.save_button_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::SaveButton;
                // Trigger save action
                if let Some(ref name) = state.config.current_profile_name {
                    // Have a profile loaded - overwrite it
                    save_profile_with_name(state, name.clone());
                } else {
                    // No profile loaded (Custom) - prompt for name
                    state.config.name_input_dialog = Some(String::new());
                }
                return;
            }
        }

        if let Some(area) = config.delete_button_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::DeleteButton;
                // Trigger delete action
                if let Some(ref name) = state.config.current_profile_name {
                    delete_profile(state, name.clone());
                }
                return;
            }
        }

        // Check text inputs
        if let Some(area) = config.output_dir_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                set_focus_and_update(state, ConfigFocus::OutputDirectory);
                return;
            }
        }

        if let Some(area) = config.filename_pattern_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                set_focus_and_update(state, ConfigFocus::FilenamePattern);
                return;
            }
        }

        if let Some(area) = config.max_workers_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::MaxWorkersInput;
                return;
            }
        }

        if let Some(area) = config.container_dropdown_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                // Toggle dropdown: if already open, close it; otherwise open it
                if config.active_dropdown == Some(ConfigFocus::ContainerDropdown) {
                    config.active_dropdown = None;
                } else {
                    config.focus = ConfigFocus::ContainerDropdown;
                    config.active_dropdown = Some(ConfigFocus::ContainerDropdown);
                }
                return;
            }
        }

        // Video output dropdowns - toggle on click
        if let Some(area) = config.fps_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                // Toggle dropdown: if already open, close it; otherwise open it
                if config.active_dropdown == Some(ConfigFocus::FpsDropdown) {
                    config.active_dropdown = None;
                } else {
                    config.focus = ConfigFocus::FpsDropdown;
                    config.active_dropdown = Some(ConfigFocus::FpsDropdown);
                }
                return;
            }
        }

        if let Some(area) = config.scale_width_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                // Toggle dropdown: if already open, close it; otherwise open it
                if config.active_dropdown == Some(ConfigFocus::ResolutionDropdown) {
                    config.active_dropdown = None;
                } else {
                    config.focus = ConfigFocus::ResolutionDropdown;
                    config.active_dropdown = Some(ConfigFocus::ResolutionDropdown);
                }
                return;
            }
        }

        // Check sliders - click to focus and set value
        if let Some(area) = config.crf_slider_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::CrfSlider;
                // Calculate value based on click position (only on bar line, not label line)
                if mouse.row == area.y + 1 && mouse.column >= area.x && area.width > 0 {
                    let relative_x = (mouse.column - area.x) as f64;
                    let ratio = (relative_x / area.width as f64).clamp(0.0, 1.0);
                    let min = 0;
                    let max = 63;
                    config.crf = (min as f64 + ratio * (max - min) as f64).round() as u32;
                }
                return;
            }
        }

        if let Some(area) = config.cpu_used_slider_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::CpuUsedSlider;
                // Calculate value based on click position (only on bar line, not label line)
                if mouse.row == area.y + 1 && mouse.column >= area.x && area.width > 0 {
                    let relative_x = (mouse.column - area.x) as f64;
                    let ratio = (relative_x / area.width as f64).clamp(0.0, 1.0);
                    let min = 0;
                    let max = 8;
                    config.cpu_used = (min as f64 + ratio * (max - min) as f64).round() as u32;
                }
                return;
            }
        }

        if let Some(area) = config.audio_bitrate_slider_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::AudioBitrateSlider;
                // Calculate value based on click position (only on bar line, not label line)
                if mouse.row == area.y + 1 && mouse.column >= area.x && area.width > 0 {
                    let relative_x = (mouse.column - area.x) as f64;
                    let ratio = (relative_x / area.width as f64).clamp(0.0, 1.0);
                    let min = 32;
                    let max = 512;
                    config.audio_bitrate = (min as f64 + ratio * (max - min) as f64).round() as u32;
                }
                return;
            }
        }

        // Check dropdowns - open popup on click
        if let Some(area) = config.profile_list_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::ProfileList;
                config.active_dropdown = Some(ConfigFocus::ProfileList);
                return;
            }
        }

        if let Some(area) = config.vp9_profile_list_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::ProfileDropdown;
                config.active_dropdown = Some(ConfigFocus::ProfileDropdown);
                return;
            }
        }

        if let Some(area) = config.codec_list_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::AudioCodec;
                config.active_dropdown = Some(ConfigFocus::AudioCodec);
                return;
            }
        }

        if let Some(area) = config.force_stereo_checkbox_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::ForceStereoCheckbox;
                config.force_stereo = !config.force_stereo;
                config.is_modified = true;
                return;
            }
        }

        // New checkboxes
        if let Some(area) = config.frame_parallel_checkbox_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::FrameParallelCheckbox;
                config.frame_parallel = !config.frame_parallel;
                return;
            }
        }

        if let Some(area) = config.fixed_gop_checkbox_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::FixedGopCheckbox;
                config.fixed_gop = !config.fixed_gop;
                return;
            }
        }

        if let Some(area) = config.auto_alt_ref_checkbox_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::AutoAltRefCheckbox;
                config.auto_alt_ref = !config.auto_alt_ref;
                return;
            }
        }

        if let Some(area) = config.enable_tpl_checkbox_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::EnableTplCheckbox;
                config.enable_tpl = !config.enable_tpl;
                return;
            }
        }

        // New sliders (per-pass cpu-used, tile rows, lag, ARNR, sharpness, noise sensitivity)
        if let Some(area) = config.cpu_used_pass1_slider_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::CpuUsedPass1Slider;
                if mouse.row == area.y + 1 && mouse.column >= area.x && area.width > 0 {
                    let relative_x = (mouse.column - area.x) as f64;
                    let ratio = (relative_x / area.width as f64).clamp(0.0, 1.0);
                    config.cpu_used_pass1 = (ratio * 8.0).round() as u32;
                }
                return;
            }
        }

        if let Some(area) = config.cpu_used_pass2_slider_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::CpuUsedPass2Slider;
                if mouse.row == area.y + 1 && mouse.column >= area.x && area.width > 0 {
                    let relative_x = (mouse.column - area.x) as f64;
                    let ratio = (relative_x / area.width as f64).clamp(0.0, 1.0);
                    config.cpu_used_pass2 = (ratio * 8.0).round() as u32;
                }
                return;
            }
        }

        if let Some(area) = config.tile_columns_slider_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::TileColumnsSlider;
                if mouse.row == area.y + 1 && mouse.column >= area.x && area.width > 0 {
                    let relative_x = (mouse.column - area.x) as f64;
                    let ratio = (relative_x / area.width as f64).clamp(0.0, 1.0);
                    config.tile_columns = (ratio * 6.0).round() as i32;
                }
                return;
            }
        }

        if let Some(area) = config.tile_rows_slider_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::TileRowsSlider;
                return;
            }
        }

        if let Some(area) = config.lag_in_frames_slider_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::LagInFramesSlider;
                if mouse.row == area.y + 1 && mouse.column >= area.x && area.width > 0 {
                    let relative_x = (mouse.column - area.x) as f64;
                    let ratio = (relative_x / area.width as f64).clamp(0.0, 1.0);
                    config.lag_in_frames = (ratio * 25.0).round() as u32;
                }
                return;
            }
        }

        if let Some(area) = config.arnr_max_frames_slider_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::ArnrMaxFramesSlider;
                return;
            }
        }

        if let Some(area) = config.arnr_strength_slider_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::ArnrStrengthSlider;
                return;
            }
        }

        if let Some(area) = config.sharpness_slider_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::SharpnessSlider;
                return;
            }
        }

        if let Some(area) = config.noise_sensitivity_slider_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::NoiseSensitivitySlider;
                return;
            }
        }

        // New numeric inputs
        if let Some(area) = config.video_target_bitrate_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::VideoTargetBitrateInput;
                return;
            }
        }

        if let Some(area) = config.video_min_bitrate_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::VideoMinBitrateInput;
                return;
            }
        }

        if let Some(area) = config.video_max_bitrate_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::VideoMaxBitrateInput;
                return;
            }
        }

        if let Some(area) = config.video_bufsize_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::VideoBufsizeInput;
                return;
            }
        }

        if let Some(area) = config.undershoot_pct_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::UndershootPctInput;
                return;
            }
        }

        if let Some(area) = config.overshoot_pct_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::OvershootPctInput;
                return;
            }
        }

        if let Some(area) = config.threads_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::ThreadsInput;
                return;
            }
        }

        if let Some(area) = config.gop_length_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::GopLengthInput;
                return;
            }
        }

        if let Some(area) = config.keyint_min_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::KeyintMinInput;
                return;
            }
        }

        if let Some(area) = config.static_thresh_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::StaticThreshInput;
                return;
            }
        }

        if let Some(area) = config.max_intra_rate_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::MaxIntraRateInput;
                return;
            }
        }

        // Rate control mode radio buttons - calculate which button was clicked
        if let Some(area) = config.rate_control_mode_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::RateControlMode;
                let relative_x = mouse.column.saturating_sub(area.x) as usize;

                if config.use_hardware_encoding {
                    // Hardware mode: CQP only (no mouse interaction needed)
                    config.vaapi_rc_mode = "1".to_string(); // Always CQP
                } else {
                    // Software mode: "(•) CQ  ( ) CQ+Cap  ( ) VBR  ( ) CBR"
                    use crate::ui::state::RateControlMode;
                    let options = ["CQ", "CQ+Cap", "VBR", "CBR"];
                    let mut x_pos = 0;

                    for (i, option) in options.iter().enumerate() {
                        let button_width = 4 + option.len() + 2; // "(•) " + label + "  "
                        if relative_x >= x_pos && relative_x < x_pos + button_width {
                            // Clicked on this option
                            config.rate_control_mode = match i {
                                0 => RateControlMode::CQ,
                                1 => RateControlMode::CQCap,
                                2 => RateControlMode::TwoPassVBR,
                                3 => RateControlMode::CBR,
                                _ => config.rate_control_mode,
                            };
                            config.is_modified = true;
                            break;
                        }
                        x_pos += button_width;
                    }
                }

                return;
            }
        }

        if let Some(area) = config.quality_mode_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::QualityMode;
                config.active_dropdown = Some(ConfigFocus::QualityMode);
                return;
            }
        }

        if let Some(area) = config.pix_fmt_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::PixFmtDropdown;
                config.active_dropdown = Some(ConfigFocus::PixFmtDropdown);
                return;
            }
        }

        if let Some(area) = config.aq_mode_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::AqModeDropdown;
                config.active_dropdown = Some(ConfigFocus::AqModeDropdown);
                return;
            }
        }

        if let Some(area) = config.arnr_type_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::ArnrTypeDropdown;
                config.active_dropdown = Some(ConfigFocus::ArnrTypeDropdown);
                return;
            }
        }

        if let Some(area) = config.tune_content_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::TuneContentDropdown;
                config.active_dropdown = Some(ConfigFocus::TuneContentDropdown);
                return;
            }
        }

        if let Some(area) = config.colorspace_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::ColorspaceDropdown;
                config.active_dropdown = Some(ConfigFocus::ColorspaceDropdown);
                return;
            }
        }

        if let Some(area) = config.color_primaries_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::ColorPrimariesDropdown;
                config.active_dropdown = Some(ConfigFocus::ColorPrimariesDropdown);
                return;
            }
        }

        if let Some(area) = config.color_trc_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::ColorTrcDropdown;
                config.active_dropdown = Some(ConfigFocus::ColorTrcDropdown);
                return;
            }
        }

        if let Some(area) = config.color_range_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::ColorRangeDropdown;
                config.active_dropdown = Some(ConfigFocus::ColorRangeDropdown);
                return;
            }
        }

        // VAAPI Hardware Encoding Controls
        // Hardware encoding checkbox
        if let Some(area) = config.hw_encoding_checkbox_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.use_hardware_encoding = !config.use_hardware_encoding;
                // Re-run preflight check if enabling
                if config.use_hardware_encoding {
                    use crate::engine::hardware;
                    let result = hardware::run_preflight();
                    config.hw_encoding_available = Some(result.available);
                    config.hw_availability_message = if result.available {
                        Some("Hardware encoding available".to_string())
                    } else {
                        result.error_message
                    };
                    // Initialize hardware encoding parameters
                    config.vaapi_rc_mode = "1".to_string(); // CQP mode - only supported mode
                }
                return;
            }
        }

        // VAAPI quality slider (1-255 range)
        if config.use_hardware_encoding && cfg!(target_os = "linux") {
            if let Some(area) = config.qsv_quality_slider_area {
                // Only respond to clicks on the bar line (second line of the 2-line widget)
                if mouse.row == area.y + 1
                    && mouse.column >= area.x
                    && mouse.column < area.x + area.width
                {
                    // The Slider widget renders the bar line with NO prefix - just bar characters at full width
                    let click_x = mouse.column.saturating_sub(area.x);
                    let ratio = (click_x as f64) / (area.width as f64).max(1.0);
                    config.qsv_global_quality = (ratio * 254.0 + 1.0).clamp(1.0, 255.0) as u32;
                    config.focus = ConfigFocus::QsvGlobalQualitySlider;
                    config.is_modified = true;
                    return;
                }
            }

            // VAAPI Compression Level slider (0-7 range)
            if let Some(area) = config.vaapi_compression_level_slider_area {
                // Only respond to clicks on the bar line (second line of the 2-line widget)
                if mouse.row == area.y + 1
                    && mouse.column >= area.x
                    && mouse.column < area.x + area.width
                {
                    // The Slider widget renders the bar line with NO prefix - just bar characters at full width
                    let click_x = mouse.column.saturating_sub(area.x);
                    let ratio = (click_x as f64) / (area.width as f64).max(1.0);
                    let new_val = (ratio * 7.0).clamp(0.0, 7.0) as u32;
                    config.vaapi_compression_level = new_val.to_string();
                    config.focus = ConfigFocus::VaapiCompressionLevelSlider;
                    config.is_modified = true;
                    return;
                }
            }

            // VAAPI B-frames textbox
            if let Some(area) = config.vaapi_b_frames_area {
                if is_in_rect(mouse.column, mouse.row, area) {
                    config.focus = ConfigFocus::VaapiBFramesInput;
                    return;
                }
            }

            // VAAPI Loop filter level textbox
            if let Some(area) = config.vaapi_loop_filter_level_area {
                if is_in_rect(mouse.column, mouse.row, area) {
                    config.focus = ConfigFocus::VaapiLoopFilterLevelInput;
                    return;
                }
            }

            // VAAPI Loop filter sharpness textbox
            if let Some(area) = config.vaapi_loop_filter_sharpness_area {
                if is_in_rect(mouse.column, mouse.row, area) {
                    config.focus = ConfigFocus::VaapiLoopFilterSharpnessInput;
                    return;
                }
            }
        }
    }
}

// Helper function to check if mouse is within table area
