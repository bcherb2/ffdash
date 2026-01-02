use super::*;

use crate::ui::options;

// Re-export profile operations from dedicated module
use super::config_profile::{
    delete_profile, get_profile_count, load_selected_profile, save_profile_with_name,
};

fn set_colorspace_preset_selection(config: &mut crate::ui::state::ConfigState, idx: usize) {
    config.colorspace_preset_state.select(Some(idx));
    config.colorspace_preset = options::colorspace_preset_from_idx(idx);

    // Sync numeric values
    let (cs, cp, ct, cr) = options::colorspace_preset_to_values(config.colorspace_preset);
    config.colorspace = cs;
    config.color_primaries = cp;
    config.color_trc = ct;
    config.color_range = cr;

    // Mark as modified
    config.is_modified = true;
}

fn set_arnr_type_selection(config: &mut crate::ui::state::ConfigState, idx: usize) {
    config.arnr_type_state.select(Some(idx));
    config.arnr_type = options::arnr_type_from_idx(idx);
}

fn set_fps_selection(config: &mut crate::ui::state::ConfigState, idx: usize) {
    config.fps_dropdown_state.select(Some(idx));
    config.fps = options::fps_from_idx(idx);
}

fn set_resolution_selection(config: &mut crate::ui::state::ConfigState, idx: usize) {
    config.resolution_dropdown_state.select(Some(idx));
    let (w, h) = options::resolution_from_idx(idx);
    config.scale_width = w;
    config.scale_height = h;
}

fn set_video_codec_selection(config: &mut crate::ui::state::ConfigState, idx: usize) {
    config.video_codec_state.select(Some(idx));
    config.codec_selection = options::codec_selection_from_idx(idx);
}

pub(super) fn initialize_default_profile(state: &mut AppState, config: &crate::config::Config) {
    use crate::engine::Profile;

    // Determine which profile to load
    // Priority: last_used_profile > defaults.profile > "1080p Shrinker"
    let profile_name = config
        .defaults
        .last_used_profile
        .as_deref()
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
                // Handle special dropdowns that need additional logic
                if state.config.active_dropdown == Some(ConfigFocus::ProfileList) {
                    load_selected_profile(state);
                } else if state.config.active_dropdown == Some(ConfigFocus::VideoCodecDropdown) {
                    // Update codec_selection enum based on video_codec_state
                    let selected = state.config.video_codec_state.selected().unwrap_or(0);
                    set_video_codec_selection(&mut state.config, selected);
                    state.config.is_modified = true;
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
            validate_numeric_field_on_blur(state);
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
            validate_numeric_field_on_blur(state);
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
        ConfigFocus::Vp9QsvPresetSlider => config.vp9_qsv_preset_area.is_some(),
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
        ConfigFocus::AudioPrimaryCodec => config.audio_primary_codec_area.is_some(),
        ConfigFocus::AudioPrimaryBitrate => config.audio_primary_bitrate_area.is_some(),
        ConfigFocus::AudioPrimaryDownmix => config.audio_primary_downmix_area.is_some(),
        ConfigFocus::AudioAc3Checkbox => config.audio_ac3_checkbox_area.is_some(),
        ConfigFocus::AudioAc3Bitrate => config.audio_ac3_bitrate_area.is_some(),
        ConfigFocus::AudioStereoCheckbox => config.audio_stereo_checkbox_area.is_some(),
        ConfigFocus::AudioStereoCodec => config.audio_stereo_codec_area.is_some(),
        ConfigFocus::AudioStereoBitrate => config.audio_stereo_bitrate_area.is_some(),

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
        ConfigFocus::HwDenoiseInput => config.hw_denoise_area.is_some(),
        ConfigFocus::HwDetailInput => config.hw_detail_area.is_some(),
        ConfigFocus::Vp9QsvLookaheadCheckbox => config.vp9_qsv_lookahead_checkbox_area.is_some(),
        ConfigFocus::Vp9QsvLookaheadDepthInput => config.vp9_qsv_lookahead_depth_area.is_some(),
        ConfigFocus::ThreadsInput => config.threads_area.is_some(),
        ConfigFocus::MaxWorkersInput => config.max_workers_area.is_some(),
        ConfigFocus::GopLengthInput => config.gop_length_area.is_some(),
        ConfigFocus::KeyintMinInput => config.keyint_min_area.is_some(),
        ConfigFocus::StaticThreshInput => config.static_thresh_area.is_some(),
        ConfigFocus::MaxIntraRateInput => config.max_intra_rate_area.is_some(),
        ConfigFocus::AutoVmafTargetInput => config.auto_vmaf_target_area.is_some(),
        ConfigFocus::AutoVmafStepInput => config.auto_vmaf_step_area.is_some(),
        ConfigFocus::AutoVmafMaxAttemptsInput => config.auto_vmaf_max_attempts_area.is_some(),
        ConfigFocus::Av1HwLookaheadInput => config.av1_hw_lookahead_area.is_some(),
        ConfigFocus::Av1HwTileColsInput => config.av1_hw_tile_cols_area.is_some(),
        ConfigFocus::Av1HwTileRowsInput => config.av1_hw_tile_rows_area.is_some(),
        ConfigFocus::AdditionalArgsInput => config.additional_args_area.is_some(),

        // All other controls (checkboxes, dropdowns, buttons) are always visible when in their section
        _ => true,
    }
}

// Reset cursor position to end of text when entering a text input field
fn reset_cursor_position_for_focus(state: &mut AppState) {
    state.config.cursor_pos = match state.config.focus {
        ConfigFocus::OutputDirectory => state.config.output_dir.chars().count(),
        ConfigFocus::FilenamePattern => state.config.filename_pattern.chars().count(),
        ConfigFocus::AdditionalArgsInput => state.config.additional_args.chars().count(),
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
        ConfigFocus::GopLengthInput => state.config.gop_length.chars().count(),
        ConfigFocus::KeyintMinInput => state.config.keyint_min.chars().count(),
        ConfigFocus::StaticThreshInput => state.config.static_thresh.chars().count(),
        ConfigFocus::MaxIntraRateInput => state.config.max_intra_rate.chars().count(),
        ConfigFocus::AutoVmafTargetInput => state.config.auto_vmaf_target.chars().count(),
        ConfigFocus::AutoVmafStepInput => state.config.auto_vmaf_step.chars().count(),
        ConfigFocus::AutoVmafMaxAttemptsInput => {
            state.config.auto_vmaf_max_attempts.chars().count()
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
        ConfigFocus::OutputDirectory
        | ConfigFocus::FilenamePattern
        | ConfigFocus::AdditionalArgsInput => InputMode::Editing,
        _ => InputMode::Normal,
    };
}

// Validate numeric text fields when focus leaves - reset to default if empty/invalid
fn validate_numeric_field_on_blur(state: &mut AppState) {
    use crate::ui::focus::ConfigFocus;
    match state.config.focus {
        ConfigFocus::GopLengthInput => {
            if state.config.gop_length.is_empty() || state.config.gop_length.parse::<u32>().is_err()
            {
                state.config.gop_length = "240".to_string();
            }
        }
        ConfigFocus::KeyintMinInput => {
            if state.config.keyint_min.is_empty() || state.config.keyint_min.parse::<u32>().is_err()
            {
                state.config.keyint_min = "0".to_string();
            }
        }
        ConfigFocus::StaticThreshInput => {
            if state.config.static_thresh.is_empty()
                || state.config.static_thresh.parse::<u32>().is_err()
            {
                state.config.static_thresh = "0".to_string();
            }
        }
        ConfigFocus::MaxIntraRateInput => {
            let valid = state
                .config
                .max_intra_rate
                .parse::<u32>()
                .map(|n| n <= 100)
                .unwrap_or(false);
            if state.config.max_intra_rate.is_empty() || !valid {
                state.config.max_intra_rate = "0".to_string();
            }
        }
        _ => {}
    }
}

// Helper function to set focus and update input mode/cursor (for mouse clicks)
fn set_focus_and_update(state: &mut AppState, new_focus: crate::ui::focus::ConfigFocus) {
    validate_numeric_field_on_blur(state);
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
        ConfigFocus::OutputDirectory
        | ConfigFocus::FilenamePattern
        | ConfigFocus::AdditionalArgsInput => {
            // Text input handling with cursor support
            let old_output = state.config.output_dir.clone();
            let old_pattern = state.config.filename_pattern.clone();
            let old_args = state.config.additional_args.clone();
            match key.code {
                KeyCode::Char(c) => {
                    let field = match state.config.focus {
                        ConfigFocus::OutputDirectory => &mut state.config.output_dir,
                        ConfigFocus::FilenamePattern => &mut state.config.filename_pattern,
                        ConfigFocus::AdditionalArgsInput => &mut state.config.additional_args,
                        _ => unreachable!(),
                    };
                    let chars: Vec<char> = field.chars().collect();
                    let pos = state.config.cursor_pos.min(chars.len());
                    let mut new_string: String = chars.iter().take(pos).collect();
                    new_string.push(c);
                    new_string.extend(chars.iter().skip(pos));
                    *field = new_string;
                    state.config.cursor_pos += 1;
                }
                KeyCode::Backspace => {
                    let field = match state.config.focus {
                        ConfigFocus::OutputDirectory => &mut state.config.output_dir,
                        ConfigFocus::FilenamePattern => &mut state.config.filename_pattern,
                        ConfigFocus::AdditionalArgsInput => &mut state.config.additional_args,
                        _ => unreachable!(),
                    };
                    // Check for Ctrl+Backspace (delete word before cursor)
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        if state.config.cursor_pos > 0 {
                            let chars: Vec<char> = field.chars().collect();
                            // Find start of word (skip backwards to whitespace or start)
                            let mut new_pos = state.config.cursor_pos;
                            // Skip trailing whitespace first
                            while new_pos > 0
                                && chars.get(new_pos - 1).is_some_and(|c| c.is_whitespace())
                            {
                                new_pos -= 1;
                            }
                            // Skip word characters
                            while new_pos > 0
                                && chars.get(new_pos - 1).is_some_and(|c| !c.is_whitespace())
                            {
                                new_pos -= 1;
                            }
                            let mut new_string: String = chars.iter().take(new_pos).collect();
                            new_string.extend(chars.iter().skip(state.config.cursor_pos));
                            *field = new_string;
                            state.config.cursor_pos = new_pos;
                        }
                    } else {
                        // Normal backspace (delete char before cursor)
                        if state.config.cursor_pos > 0 {
                            let chars: Vec<char> = field.chars().collect();
                            let mut new_string: String =
                                chars.iter().take(state.config.cursor_pos - 1).collect();
                            new_string.extend(chars.iter().skip(state.config.cursor_pos));
                            *field = new_string;
                            state.config.cursor_pos -= 1;
                        }
                    }
                }
                KeyCode::Delete => {
                    // Delete character after cursor
                    let field = match state.config.focus {
                        ConfigFocus::OutputDirectory => &mut state.config.output_dir,
                        ConfigFocus::FilenamePattern => &mut state.config.filename_pattern,
                        ConfigFocus::AdditionalArgsInput => &mut state.config.additional_args,
                        _ => unreachable!(),
                    };
                    let chars: Vec<char> = field.chars().collect();
                    if state.config.cursor_pos < chars.len() {
                        let mut new_string: String =
                            chars.iter().take(state.config.cursor_pos).collect();
                        new_string.extend(chars.iter().skip(state.config.cursor_pos + 1));
                        *field = new_string;
                    }
                }
                KeyCode::Left => {
                    if state.config.cursor_pos > 0 {
                        state.config.cursor_pos -= 1;
                    }
                }
                KeyCode::Right => {
                    let max_len = match state.config.focus {
                        ConfigFocus::OutputDirectory => state.config.output_dir.chars().count(),
                        ConfigFocus::FilenamePattern => {
                            state.config.filename_pattern.chars().count()
                        }
                        ConfigFocus::AdditionalArgsInput => {
                            state.config.additional_args.chars().count()
                        }
                        _ => 0,
                    };
                    if state.config.cursor_pos < max_len {
                        state.config.cursor_pos += 1;
                    }
                }
                KeyCode::Home => {
                    state.config.cursor_pos = 0;
                }
                KeyCode::End => {
                    state.config.cursor_pos = match state.config.focus {
                        ConfigFocus::OutputDirectory => state.config.output_dir.chars().count(),
                        ConfigFocus::FilenamePattern => {
                            state.config.filename_pattern.chars().count()
                        }
                        ConfigFocus::AdditionalArgsInput => {
                            state.config.additional_args.chars().count()
                        }
                        _ => 0,
                    };
                }
                _ => {}
            }
            if state.config.output_dir != old_output
                || state.config.filename_pattern != old_pattern
                || state.config.additional_args != old_args
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
        ConfigFocus::AutoVmafCheckbox => {
            if matches!(key.code, KeyCode::Char(' ') | KeyCode::Enter) {
                state.config.auto_vmaf_enabled = !state.config.auto_vmaf_enabled;
                state.config.is_modified = true;
            }
        }
        ConfigFocus::AutoVmafTargetInput => {
            let old_value = state.config.auto_vmaf_target.clone();
            match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => {
                    // Only allow one decimal point
                    if c == '.' && state.config.auto_vmaf_target.contains('.') {
                        // Ignore additional decimal points
                    } else {
                        let chars: Vec<char> = state.config.auto_vmaf_target.chars().collect();
                        let pos = state.config.cursor_pos.min(chars.len());
                        let mut new_string: String = chars.iter().take(pos).collect();
                        new_string.push(c);
                        new_string.extend(chars.iter().skip(pos));
                        state.config.auto_vmaf_target = new_string;
                        state.config.cursor_pos += 1;
                    }
                }
                KeyCode::Backspace => {
                    if state.config.cursor_pos > 0 {
                        let chars: Vec<char> = state.config.auto_vmaf_target.chars().collect();
                        let mut new_string: String =
                            chars.iter().take(state.config.cursor_pos - 1).collect();
                        new_string.extend(chars.iter().skip(state.config.cursor_pos));
                        state.config.auto_vmaf_target = new_string;
                        state.config.cursor_pos -= 1;
                    }
                }
                KeyCode::Delete => {
                    let chars: Vec<char> = state.config.auto_vmaf_target.chars().collect();
                    if state.config.cursor_pos < chars.len() {
                        let mut new_string: String =
                            chars.iter().take(state.config.cursor_pos).collect();
                        new_string.extend(chars.iter().skip(state.config.cursor_pos + 1));
                        state.config.auto_vmaf_target = new_string;
                    }
                }
                KeyCode::Left => {
                    if state.config.cursor_pos > 0 {
                        state.config.cursor_pos -= 1;
                    }
                }
                KeyCode::Right => {
                    let len = state.config.auto_vmaf_target.chars().count();
                    if state.config.cursor_pos < len {
                        state.config.cursor_pos += 1;
                    }
                }
                KeyCode::Home => {
                    state.config.cursor_pos = 0;
                }
                KeyCode::End => {
                    state.config.cursor_pos = state.config.auto_vmaf_target.chars().count();
                }
                _ => {}
            }
            if state.config.auto_vmaf_target != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::AutoVmafStepInput => {
            let old_value = state.config.auto_vmaf_step.clone();
            match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let chars: Vec<char> = state.config.auto_vmaf_step.chars().collect();
                    let pos = state.config.cursor_pos.min(chars.len());
                    let mut new_string: String = chars.iter().take(pos).collect();
                    new_string.push(c);
                    new_string.extend(chars.iter().skip(pos));
                    state.config.auto_vmaf_step = new_string;
                    state.config.cursor_pos += 1;
                }
                KeyCode::Backspace => {
                    if state.config.cursor_pos > 0 {
                        let chars: Vec<char> = state.config.auto_vmaf_step.chars().collect();
                        let mut new_string: String =
                            chars.iter().take(state.config.cursor_pos - 1).collect();
                        new_string.extend(chars.iter().skip(state.config.cursor_pos));
                        state.config.auto_vmaf_step = new_string;
                        state.config.cursor_pos -= 1;
                    }
                }
                KeyCode::Delete => {
                    let chars: Vec<char> = state.config.auto_vmaf_step.chars().collect();
                    if state.config.cursor_pos < chars.len() {
                        let mut new_string: String =
                            chars.iter().take(state.config.cursor_pos).collect();
                        new_string.extend(chars.iter().skip(state.config.cursor_pos + 1));
                        state.config.auto_vmaf_step = new_string;
                    }
                }
                KeyCode::Left => {
                    if state.config.cursor_pos > 0 {
                        state.config.cursor_pos -= 1;
                    }
                }
                KeyCode::Right => {
                    let len = state.config.auto_vmaf_step.chars().count();
                    if state.config.cursor_pos < len {
                        state.config.cursor_pos += 1;
                    }
                }
                KeyCode::Home => {
                    state.config.cursor_pos = 0;
                }
                KeyCode::End => {
                    state.config.cursor_pos = state.config.auto_vmaf_step.chars().count();
                }
                _ => {}
            }
            if state.config.auto_vmaf_step != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::AutoVmafMaxAttemptsInput => {
            let old_value = state.config.auto_vmaf_max_attempts.clone();
            match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let chars: Vec<char> = state.config.auto_vmaf_max_attempts.chars().collect();
                    let pos = state.config.cursor_pos.min(chars.len());
                    let mut new_string: String = chars.iter().take(pos).collect();
                    new_string.push(c);
                    new_string.extend(chars.iter().skip(pos));
                    state.config.auto_vmaf_max_attempts = new_string;
                    state.config.cursor_pos += 1;
                }
                KeyCode::Backspace => {
                    if state.config.cursor_pos > 0 {
                        let chars: Vec<char> =
                            state.config.auto_vmaf_max_attempts.chars().collect();
                        let mut new_string: String =
                            chars.iter().take(state.config.cursor_pos - 1).collect();
                        new_string.extend(chars.iter().skip(state.config.cursor_pos));
                        state.config.auto_vmaf_max_attempts = new_string;
                        state.config.cursor_pos -= 1;
                    }
                }
                KeyCode::Delete => {
                    let chars: Vec<char> = state.config.auto_vmaf_max_attempts.chars().collect();
                    if state.config.cursor_pos < chars.len() {
                        let mut new_string: String =
                            chars.iter().take(state.config.cursor_pos).collect();
                        new_string.extend(chars.iter().skip(state.config.cursor_pos + 1));
                        state.config.auto_vmaf_max_attempts = new_string;
                    }
                }
                KeyCode::Left => {
                    if state.config.cursor_pos > 0 {
                        state.config.cursor_pos -= 1;
                    }
                }
                KeyCode::Right => {
                    let len = state.config.auto_vmaf_max_attempts.chars().count();
                    if state.config.cursor_pos < len {
                        state.config.cursor_pos += 1;
                    }
                }
                KeyCode::Home => {
                    state.config.cursor_pos = 0;
                }
                KeyCode::End => {
                    state.config.cursor_pos = state.config.auto_vmaf_max_attempts.chars().count();
                }
                _ => {}
            }
            if state.config.auto_vmaf_max_attempts != old_value {
                state.config.is_modified = true;
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
                    set_fps_selection(&mut state.config, new_idx);
                    state.config.is_modified = true;
                }
                KeyCode::Right | KeyCode::Down => {
                    // Cycle to next FPS option (Right for quick nav, Down for dropdown list)
                    let current = state.config.fps_dropdown_state.selected().unwrap_or(0);
                    let new_idx = if current >= 10 { 0 } else { current + 1 }; // 11 options (0-10)
                    set_fps_selection(&mut state.config, new_idx);
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
                    set_resolution_selection(&mut state.config, new_idx);
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
                    set_resolution_selection(&mut state.config, new_idx);
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
        ConfigFocus::AudioPrimaryCodec => {
            let old_selection = state.config.audio_primary_codec_state.selected();
            match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    state.config.active_dropdown = Some(ConfigFocus::AudioPrimaryCodec);
                }
                KeyCode::Up => {
                    let selected = state
                        .config
                        .audio_primary_codec_state
                        .selected()
                        .unwrap_or(0);
                    if selected > 0 {
                        state
                            .config
                            .audio_primary_codec_state
                            .select(Some(selected - 1));
                        state.config.audio_primary_codec =
                            crate::ui::state::AudioPrimaryCodec::from_index(selected - 1);
                    }
                }
                KeyCode::Down => {
                    let selected = state
                        .config
                        .audio_primary_codec_state
                        .selected()
                        .unwrap_or(0);
                    if selected < 4 {
                        state
                            .config
                            .audio_primary_codec_state
                            .select(Some(selected + 1));
                        state.config.audio_primary_codec =
                            crate::ui::state::AudioPrimaryCodec::from_index(selected + 1);
                    }
                }
                _ => {}
            }
            if state.config.audio_primary_codec_state.selected() != old_selection {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::AudioPrimaryBitrate => {
            // Only allow adjustments when not passthrough
            if !state.config.audio_primary_codec.is_passthrough() {
                let old_value = state.config.audio_primary_bitrate;
                match key.code {
                    KeyCode::Left => {
                        state.config.audio_primary_bitrate =
                            state.config.audio_primary_bitrate.saturating_sub(8).max(32);
                    }
                    KeyCode::Right => {
                        state.config.audio_primary_bitrate =
                            (state.config.audio_primary_bitrate + 8).min(512);
                    }
                    KeyCode::Home => state.config.audio_primary_bitrate = 32,
                    KeyCode::End => state.config.audio_primary_bitrate = 512,
                    _ => {}
                }
                if state.config.audio_primary_bitrate != old_value {
                    state.config.is_modified = true;
                }
            }
        }
        ConfigFocus::AudioPrimaryDownmix => {
            // Only allow toggle when not passthrough
            if !state.config.audio_primary_codec.is_passthrough()
                && matches!(key.code, KeyCode::Char(' ') | KeyCode::Enter)
            {
                state.config.audio_primary_downmix = !state.config.audio_primary_downmix;
                state.config.is_modified = true;
            }
        }
        ConfigFocus::AudioAc3Checkbox => {
            if matches!(key.code, KeyCode::Char(' ') | KeyCode::Enter) {
                state.config.audio_add_ac3 = !state.config.audio_add_ac3;
                state.config.is_modified = true;
            }
        }
        ConfigFocus::AudioAc3Bitrate => {
            if state.config.audio_add_ac3 {
                let old_value = state.config.audio_ac3_bitrate;
                match key.code {
                    KeyCode::Left => {
                        state.config.audio_ac3_bitrate =
                            state.config.audio_ac3_bitrate.saturating_sub(64).max(384);
                    }
                    KeyCode::Right => {
                        state.config.audio_ac3_bitrate =
                            (state.config.audio_ac3_bitrate + 64).min(640);
                    }
                    KeyCode::Home => state.config.audio_ac3_bitrate = 384,
                    KeyCode::End => state.config.audio_ac3_bitrate = 640,
                    _ => {}
                }
                if state.config.audio_ac3_bitrate != old_value {
                    state.config.is_modified = true;
                }
            }
        }
        ConfigFocus::AudioStereoCheckbox => {
            if matches!(key.code, KeyCode::Char(' ') | KeyCode::Enter) {
                state.config.audio_add_stereo = !state.config.audio_add_stereo;
                state.config.is_modified = true;
            }
        }
        ConfigFocus::AudioStereoCodec => {
            if state.config.audio_add_stereo {
                let old_selection = state.config.audio_stereo_codec_state.selected();
                match key.code {
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        state.config.active_dropdown = Some(ConfigFocus::AudioStereoCodec);
                    }
                    KeyCode::Up | KeyCode::Down => {
                        let selected = state
                            .config
                            .audio_stereo_codec_state
                            .selected()
                            .unwrap_or(0);
                        let new_selected = if selected == 0 { 1 } else { 0 };
                        state
                            .config
                            .audio_stereo_codec_state
                            .select(Some(new_selected));
                        state.config.audio_stereo_codec =
                            crate::ui::state::AudioStereoCodec::from_index(new_selected);
                    }
                    _ => {}
                }
                if state.config.audio_stereo_codec_state.selected() != old_selection {
                    state.config.is_modified = true;
                }
            }
        }
        ConfigFocus::AudioStereoBitrate => {
            if state.config.audio_add_stereo {
                let old_value = state.config.audio_stereo_bitrate;
                match key.code {
                    KeyCode::Left => {
                        state.config.audio_stereo_bitrate =
                            state.config.audio_stereo_bitrate.saturating_sub(8).max(64);
                    }
                    KeyCode::Right => {
                        state.config.audio_stereo_bitrate =
                            (state.config.audio_stereo_bitrate + 8).min(256);
                    }
                    KeyCode::Home => state.config.audio_stereo_bitrate = 64,
                    KeyCode::End => state.config.audio_stereo_bitrate = 256,
                    _ => {}
                }
                if state.config.audio_stereo_bitrate != old_value {
                    state.config.is_modified = true;
                }
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
                // Cycle through 0 (disabled), 1 (enabled), 2 (enabled with statistics)
                state.config.auto_alt_ref = (state.config.auto_alt_ref + 1) % 3;
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
                    if selected < 2 {
                        // 3 pixel formats
                        state.config.pix_fmt_state.select(Some(selected + 1));
                    }
                }
                _ => {}
            }
            if state.config.pix_fmt_state.selected() != old_selection {
                state.config.is_modified = true;
            }
        }
        // Video codec selector (VP9/AV1)
        ConfigFocus::VideoCodecDropdown => {
            let old_selection = state.config.video_codec_state.selected();
            match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    state.config.active_dropdown = Some(ConfigFocus::VideoCodecDropdown);
                }
                KeyCode::Up | KeyCode::Left => {
                    let selected = state.config.video_codec_state.selected().unwrap_or(0);
                    if selected > 0 {
                        set_video_codec_selection(&mut state.config, selected - 1);
                    }
                }
                KeyCode::Down | KeyCode::Right => {
                    let selected = state.config.video_codec_state.selected().unwrap_or(0);
                    if selected < 1 {
                        set_video_codec_selection(&mut state.config, selected + 1);
                    }
                }
                _ => {}
            }
            if state.config.video_codec_state.selected() != old_selection {
                state.config.is_modified = true;
            }
        }
        // AV1 software settings
        ConfigFocus::Av1PresetSlider => {
            let old_value = state.config.av1_preset;
            match key.code {
                KeyCode::Left => {
                    if state.config.av1_preset > 0 {
                        state.config.av1_preset -= 1;
                    }
                }
                KeyCode::Right => {
                    if state.config.av1_preset < 13 {
                        state.config.av1_preset += 1;
                    }
                }
                KeyCode::Home => state.config.av1_preset = 0,
                KeyCode::End => state.config.av1_preset = 13,
                _ => {}
            }
            if state.config.av1_preset != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::Av1TuneDropdown => {
            let old_selection = state.config.av1_tune_state.selected();
            match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    state.config.active_dropdown = Some(ConfigFocus::Av1TuneDropdown);
                }
                KeyCode::Up | KeyCode::Left => {
                    let selected = state.config.av1_tune_state.selected().unwrap_or(0);
                    if selected > 0 {
                        state.config.av1_tune_state.select(Some(selected - 1));
                    }
                }
                KeyCode::Down | KeyCode::Right => {
                    let selected = state.config.av1_tune_state.selected().unwrap_or(0);
                    if selected < 2 {
                        state.config.av1_tune_state.select(Some(selected + 1));
                    }
                }
                _ => {}
            }
            if state.config.av1_tune_state.selected() != old_selection {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::Av1FilmGrainSlider => {
            let old_value = state.config.av1_film_grain;
            match key.code {
                KeyCode::Left => {
                    if state.config.av1_film_grain > 0 {
                        state.config.av1_film_grain -= 1;
                    }
                }
                KeyCode::Right => {
                    if state.config.av1_film_grain < 50 {
                        state.config.av1_film_grain += 1;
                    }
                }
                KeyCode::Home => state.config.av1_film_grain = 0,
                KeyCode::End => state.config.av1_film_grain = 50,
                _ => {}
            }
            if state.config.av1_film_grain != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::Av1FilmGrainDenoiseCheckbox => {
            if matches!(key.code, KeyCode::Char(' ') | KeyCode::Enter) {
                state.config.av1_film_grain_denoise = !state.config.av1_film_grain_denoise;
                state.config.is_modified = true;
            }
        }
        ConfigFocus::Av1EnableOverlaysCheckbox => {
            if matches!(key.code, KeyCode::Char(' ') | KeyCode::Enter) {
                state.config.av1_enable_overlays = !state.config.av1_enable_overlays;
                state.config.is_modified = true;
            }
        }
        ConfigFocus::Av1ScdCheckbox => {
            if matches!(key.code, KeyCode::Char(' ') | KeyCode::Enter) {
                state.config.av1_scd = !state.config.av1_scd;
                state.config.is_modified = true;
            }
        }
        ConfigFocus::Av1ScmDropdown => {
            let old_selection = state.config.av1_scm_state.selected();
            match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    state.config.active_dropdown = Some(ConfigFocus::Av1ScmDropdown);
                }
                KeyCode::Up | KeyCode::Left => {
                    let selected = state.config.av1_scm_state.selected().unwrap_or(0);
                    if selected > 0 {
                        state.config.av1_scm_state.select(Some(selected - 1));
                    }
                }
                KeyCode::Down | KeyCode::Right => {
                    let selected = state.config.av1_scm_state.selected().unwrap_or(0);
                    if selected < 2 {
                        state.config.av1_scm_state.select(Some(selected + 1));
                    }
                }
                _ => {}
            }
            if state.config.av1_scm_state.selected() != old_selection {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::Av1EnableTfCheckbox => {
            if matches!(key.code, KeyCode::Char(' ') | KeyCode::Enter) {
                state.config.av1_enable_tf = !state.config.av1_enable_tf;
                state.config.is_modified = true;
            }
        }
        // AV1 hardware settings
        ConfigFocus::Av1HwPresetSlider => {
            let old_value = state.config.av1_hw_preset;
            match key.code {
                KeyCode::Left => {
                    if state.config.av1_hw_preset > 1 {
                        state.config.av1_hw_preset -= 1;
                    }
                }
                KeyCode::Right => {
                    if state.config.av1_hw_preset < 7 {
                        state.config.av1_hw_preset += 1;
                    }
                }
                KeyCode::Home => state.config.av1_hw_preset = 1,
                KeyCode::End => state.config.av1_hw_preset = 7,
                _ => {}
            }
            if state.config.av1_hw_preset != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::Av1HwCqSlider => {
            // Per-encoder quality: NVENC uses 0-63, QSV/VAAPI use 1-255
            // Read/write the encoder-specific field based on GPU vendor
            let (min_cq, max_cq, current_val) = match state.config.gpu_vendor {
                crate::engine::hardware::GpuVendor::Nvidia => (0, 63, state.config.av1_nvenc_cq),
                crate::engine::hardware::GpuVendor::Intel => (1, 255, state.config.av1_qsv_cq),
                _ => (1, 255, state.config.av1_vaapi_cq), // AMD and others use VAAPI
            };

            let old_value = current_val;
            let new_value = match key.code {
                KeyCode::Left => {
                    if current_val > min_cq {
                        current_val - 1
                    } else {
                        current_val
                    }
                }
                KeyCode::Right => {
                    if current_val < max_cq {
                        current_val + 1
                    } else {
                        current_val
                    }
                }
                KeyCode::Home => min_cq,
                KeyCode::End => max_cq,
                _ => current_val,
            };

            // Write back to the correct per-encoder field
            match state.config.gpu_vendor {
                crate::engine::hardware::GpuVendor::Nvidia => state.config.av1_nvenc_cq = new_value,
                crate::engine::hardware::GpuVendor::Intel => state.config.av1_qsv_cq = new_value,
                _ => state.config.av1_vaapi_cq = new_value,
            };

            if new_value != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::Av1HwLookaheadInput => {
            let old_value = state.config.av1_hw_lookahead;
            match key.code {
                KeyCode::Left => {
                    if state.config.av1_hw_lookahead > 0 {
                        state.config.av1_hw_lookahead -= 1;
                    }
                }
                KeyCode::Right => {
                    if state.config.av1_hw_lookahead < 100 {
                        state.config.av1_hw_lookahead += 1;
                    }
                }
                KeyCode::Home => state.config.av1_hw_lookahead = 0,
                KeyCode::End => state.config.av1_hw_lookahead = 100,
                _ => {}
            }
            if state.config.av1_hw_lookahead != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::Av1HwTileColsInput => {
            let old_value = state.config.av1_hw_tile_cols;
            match key.code {
                KeyCode::Left => {
                    if state.config.av1_hw_tile_cols > 0 {
                        state.config.av1_hw_tile_cols -= 1;
                    }
                }
                KeyCode::Right => {
                    if state.config.av1_hw_tile_cols < 4 {
                        state.config.av1_hw_tile_cols += 1;
                    }
                }
                _ => {}
            }
            if state.config.av1_hw_tile_cols != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::Av1HwTileRowsInput => {
            let old_value = state.config.av1_hw_tile_rows;
            match key.code {
                KeyCode::Left => {
                    if state.config.av1_hw_tile_rows > 0 {
                        state.config.av1_hw_tile_rows -= 1;
                    }
                }
                KeyCode::Right => {
                    if state.config.av1_hw_tile_rows < 4 {
                        state.config.av1_hw_tile_rows += 1;
                    }
                }
                _ => {}
            }
            if state.config.av1_hw_tile_rows != old_value {
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
                        set_arnr_type_selection(&mut state.config, selected - 1);
                    }
                }
                KeyCode::Down => {
                    let selected = state.config.arnr_type_state.selected().unwrap_or(0);
                    if selected < 3 {
                        // 4 ARNR types
                        set_arnr_type_selection(&mut state.config, selected + 1);
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
        ConfigFocus::ColorSpacePresetDropdown => {
            let old_selection = state.config.colorspace_preset_state.selected();
            match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    state.config.active_dropdown = Some(ConfigFocus::ColorSpacePresetDropdown);
                }
                KeyCode::Up => {
                    let selected = state.config.colorspace_preset_state.selected().unwrap_or(0);
                    if selected > 0 {
                        set_colorspace_preset_selection(&mut state.config, selected - 1);
                    }
                }
                KeyCode::Down => {
                    let selected = state.config.colorspace_preset_state.selected().unwrap_or(0);
                    if selected < 2 {
                        // 3 presets (Auto, SDR, HDR10)
                        set_colorspace_preset_selection(&mut state.config, selected + 1);
                    }
                }
                _ => {}
            }
            if state.config.colorspace_preset_state.selected() != old_selection {
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
                    if (1..=16).contains(&new_val) {
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
            let old_value = state.config.gop_length.clone();
            match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let chars: Vec<char> = state.config.gop_length.chars().collect();
                    let pos = state.config.cursor_pos.min(chars.len());
                    let mut new_string: String = chars.iter().take(pos).collect();
                    new_string.push(c);
                    new_string.extend(chars.iter().skip(pos));
                    if let Ok(_num) = new_string.parse::<u32>() {
                        state.config.gop_length = new_string;
                        state.config.cursor_pos += 1;
                    }
                }
                KeyCode::Left => {
                    if state.config.cursor_pos > 0 {
                        state.config.cursor_pos -= 1;
                    }
                }
                KeyCode::Right => {
                    let char_count = state.config.gop_length.chars().count();
                    if state.config.cursor_pos < char_count {
                        state.config.cursor_pos += 1;
                    }
                }
                KeyCode::Home => {
                    state.config.cursor_pos = 0;
                }
                KeyCode::End => {
                    state.config.cursor_pos = state.config.gop_length.chars().count();
                }
                KeyCode::Backspace => {
                    if state.config.cursor_pos > 0 {
                        let chars: Vec<char> = state.config.gop_length.chars().collect();
                        let mut new_string: String =
                            chars.iter().take(state.config.cursor_pos - 1).collect();
                        new_string.extend(chars.iter().skip(state.config.cursor_pos));
                        state.config.gop_length = new_string;
                        state.config.cursor_pos -= 1;
                    }
                }
                KeyCode::Delete => {
                    let chars: Vec<char> = state.config.gop_length.chars().collect();
                    if state.config.cursor_pos < chars.len() {
                        let mut new_string: String =
                            chars.iter().take(state.config.cursor_pos).collect();
                        new_string.extend(chars.iter().skip(state.config.cursor_pos + 1));
                        state.config.gop_length = new_string;
                    }
                }
                _ => {}
            }
            if state.config.gop_length != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::KeyintMinInput => {
            let old_value = state.config.keyint_min.clone();
            match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let chars: Vec<char> = state.config.keyint_min.chars().collect();
                    let pos = state.config.cursor_pos.min(chars.len());
                    let mut new_string: String = chars.iter().take(pos).collect();
                    new_string.push(c);
                    new_string.extend(chars.iter().skip(pos));
                    if let Ok(_num) = new_string.parse::<u32>() {
                        state.config.keyint_min = new_string;
                        state.config.cursor_pos += 1;
                    }
                }
                KeyCode::Left => {
                    if state.config.cursor_pos > 0 {
                        state.config.cursor_pos -= 1;
                    }
                }
                KeyCode::Right => {
                    let char_count = state.config.keyint_min.chars().count();
                    if state.config.cursor_pos < char_count {
                        state.config.cursor_pos += 1;
                    }
                }
                KeyCode::Home => {
                    state.config.cursor_pos = 0;
                }
                KeyCode::End => {
                    state.config.cursor_pos = state.config.keyint_min.chars().count();
                }
                KeyCode::Backspace => {
                    if state.config.cursor_pos > 0 {
                        let chars: Vec<char> = state.config.keyint_min.chars().collect();
                        let mut new_string: String =
                            chars.iter().take(state.config.cursor_pos - 1).collect();
                        new_string.extend(chars.iter().skip(state.config.cursor_pos));
                        state.config.keyint_min = new_string;
                        state.config.cursor_pos -= 1;
                    }
                }
                KeyCode::Delete => {
                    let chars: Vec<char> = state.config.keyint_min.chars().collect();
                    if state.config.cursor_pos < chars.len() {
                        let mut new_string: String =
                            chars.iter().take(state.config.cursor_pos).collect();
                        new_string.extend(chars.iter().skip(state.config.cursor_pos + 1));
                        state.config.keyint_min = new_string;
                    }
                }
                _ => {}
            }
            if state.config.keyint_min != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::StaticThreshInput => {
            let old_value = state.config.static_thresh.clone();
            match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let chars: Vec<char> = state.config.static_thresh.chars().collect();
                    let pos = state.config.cursor_pos.min(chars.len());
                    let mut new_string: String = chars.iter().take(pos).collect();
                    new_string.push(c);
                    new_string.extend(chars.iter().skip(pos));
                    if let Ok(_num) = new_string.parse::<u32>() {
                        state.config.static_thresh = new_string;
                        state.config.cursor_pos += 1;
                    }
                }
                KeyCode::Left => {
                    if state.config.cursor_pos > 0 {
                        state.config.cursor_pos -= 1;
                    }
                }
                KeyCode::Right => {
                    let char_count = state.config.static_thresh.chars().count();
                    if state.config.cursor_pos < char_count {
                        state.config.cursor_pos += 1;
                    }
                }
                KeyCode::Home => {
                    state.config.cursor_pos = 0;
                }
                KeyCode::End => {
                    state.config.cursor_pos = state.config.static_thresh.chars().count();
                }
                KeyCode::Backspace => {
                    if state.config.cursor_pos > 0 {
                        let chars: Vec<char> = state.config.static_thresh.chars().collect();
                        let mut new_string: String =
                            chars.iter().take(state.config.cursor_pos - 1).collect();
                        new_string.extend(chars.iter().skip(state.config.cursor_pos));
                        state.config.static_thresh = new_string;
                        state.config.cursor_pos -= 1;
                    }
                }
                KeyCode::Delete => {
                    let chars: Vec<char> = state.config.static_thresh.chars().collect();
                    if state.config.cursor_pos < chars.len() {
                        let mut new_string: String =
                            chars.iter().take(state.config.cursor_pos).collect();
                        new_string.extend(chars.iter().skip(state.config.cursor_pos + 1));
                        state.config.static_thresh = new_string;
                    }
                }
                _ => {}
            }
            if state.config.static_thresh != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::MaxIntraRateInput => {
            let old_value = state.config.max_intra_rate.clone();
            match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let chars: Vec<char> = state.config.max_intra_rate.chars().collect();
                    let pos = state.config.cursor_pos.min(chars.len());
                    let mut new_string: String = chars.iter().take(pos).collect();
                    new_string.push(c);
                    new_string.extend(chars.iter().skip(pos));
                    if let Ok(num) = new_string.parse::<u32>() {
                        if num <= 100 {
                            state.config.max_intra_rate = new_string;
                            state.config.cursor_pos += 1;
                        }
                    }
                }
                KeyCode::Left => {
                    if state.config.cursor_pos > 0 {
                        state.config.cursor_pos -= 1;
                    }
                }
                KeyCode::Right => {
                    let char_count = state.config.max_intra_rate.chars().count();
                    if state.config.cursor_pos < char_count {
                        state.config.cursor_pos += 1;
                    }
                }
                KeyCode::Home => {
                    state.config.cursor_pos = 0;
                }
                KeyCode::End => {
                    state.config.cursor_pos = state.config.max_intra_rate.chars().count();
                }
                KeyCode::Backspace => {
                    if state.config.cursor_pos > 0 {
                        let chars: Vec<char> = state.config.max_intra_rate.chars().collect();
                        let mut new_string: String =
                            chars.iter().take(state.config.cursor_pos - 1).collect();
                        new_string.extend(chars.iter().skip(state.config.cursor_pos));
                        state.config.max_intra_rate = new_string;
                        state.config.cursor_pos -= 1;
                    }
                }
                KeyCode::Delete => {
                    let chars: Vec<char> = state.config.max_intra_rate.chars().collect();
                    if state.config.cursor_pos < chars.len() {
                        let mut new_string: String =
                            chars.iter().take(state.config.cursor_pos).collect();
                        new_string.extend(chars.iter().skip(state.config.cursor_pos + 1));
                        state.config.max_intra_rate = new_string;
                    }
                }
                _ => {}
            }
            if state.config.max_intra_rate != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::VaapiBFramesInput => {
            let old_value = state.config.vaapi_b_frames.clone();
            let mut value = state.config.vaapi_b_frames.parse::<u32>().unwrap_or(0);
            let mut changed = false;
            match key.code {
                KeyCode::Left | KeyCode::Up => {
                    if value > 0 {
                        value -= 1;
                        changed = true;
                    }
                }
                KeyCode::Right | KeyCode::Down => {
                    if value < 4 {
                        value += 1;
                        changed = true;
                    }
                }
                KeyCode::Home => {
                    if value != 0 {
                        value = 0;
                        changed = true;
                    }
                }
                KeyCode::End => {
                    if value != 4 {
                        value = 4;
                        changed = true;
                    }
                }
                _ => {}
            }
            if changed {
                state.config.vaapi_b_frames = value.to_string();
            }
            if state.config.vaapi_b_frames != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::VaapiLoopFilterLevelInput => {
            let old_value = state.config.vaapi_loop_filter_level.clone();
            let mut value = state
                .config
                .vaapi_loop_filter_level
                .parse::<u32>()
                .unwrap_or(16);
            let mut changed = false;
            match key.code {
                KeyCode::Left | KeyCode::Up => {
                    if value > 0 {
                        value -= 1;
                        changed = true;
                    }
                }
                KeyCode::Right | KeyCode::Down => {
                    if value < 63 {
                        value += 1;
                        changed = true;
                    }
                }
                KeyCode::Home => {
                    if value != 0 {
                        value = 0;
                        changed = true;
                    }
                }
                KeyCode::End => {
                    if value != 63 {
                        value = 63;
                        changed = true;
                    }
                }
                _ => {}
            }
            if changed {
                state.config.vaapi_loop_filter_level = value.to_string();
            }
            if state.config.vaapi_loop_filter_level != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::VaapiLoopFilterSharpnessInput => {
            let old_value = state.config.vaapi_loop_filter_sharpness.clone();
            let mut value = state
                .config
                .vaapi_loop_filter_sharpness
                .parse::<u32>()
                .unwrap_or(4);
            let mut changed = false;
            match key.code {
                KeyCode::Left | KeyCode::Up => {
                    if value > 0 {
                        value -= 1;
                        changed = true;
                    }
                }
                KeyCode::Right | KeyCode::Down => {
                    if value < 15 {
                        value += 1;
                        changed = true;
                    }
                }
                KeyCode::Home => {
                    if value != 0 {
                        value = 0;
                        changed = true;
                    }
                }
                KeyCode::End => {
                    if value != 15 {
                        value = 15;
                        changed = true;
                    }
                }
                _ => {}
            }
            if changed {
                state.config.vaapi_loop_filter_sharpness = value.to_string();
            }
            if state.config.vaapi_loop_filter_sharpness != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::HwDenoiseInput => {
            let old_value = state.config.hw_denoise.clone();
            let mut value = state.config.hw_denoise.parse::<u32>().unwrap_or(0);
            let mut changed = false;
            match key.code {
                KeyCode::Left | KeyCode::Up => {
                    if value > 0 {
                        value -= 1;
                        changed = true;
                    }
                }
                KeyCode::Right | KeyCode::Down => {
                    if value < 100 {
                        value += 1;
                        changed = true;
                    }
                }
                KeyCode::Home => {
                    if value != 0 {
                        value = 0;
                        changed = true;
                    }
                }
                KeyCode::End => {
                    if value != 100 {
                        value = 100;
                        changed = true;
                    }
                }
                _ => {}
            }
            if changed {
                state.config.hw_denoise = value.to_string();
            }
            if state.config.hw_denoise != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::HwDetailInput => {
            let old_value = state.config.hw_detail.clone();
            let mut value = state.config.hw_detail.parse::<u32>().unwrap_or(0);
            let mut changed = false;
            match key.code {
                KeyCode::Left | KeyCode::Up => {
                    if value > 0 {
                        value -= 1;
                        changed = true;
                    }
                }
                KeyCode::Right | KeyCode::Down => {
                    if value < 100 {
                        value += 1;
                        changed = true;
                    }
                }
                KeyCode::Home => {
                    if value != 0 {
                        value = 0;
                        changed = true;
                    }
                }
                KeyCode::End => {
                    if value != 100 {
                        value = 100;
                        changed = true;
                    }
                }
                _ => {}
            }
            if changed {
                state.config.hw_detail = value.to_string();
            }
            if state.config.hw_detail != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::Vp9QsvPresetSlider => {
            let old_value = state.config.vp9_qsv_preset;
            match key.code {
                KeyCode::Left => {
                    if state.config.vp9_qsv_preset > 1 {
                        state.config.vp9_qsv_preset -= 1;
                    }
                }
                KeyCode::Right => {
                    if state.config.vp9_qsv_preset < 7 {
                        state.config.vp9_qsv_preset += 1;
                    }
                }
                KeyCode::Home => state.config.vp9_qsv_preset = 1,
                KeyCode::End => state.config.vp9_qsv_preset = 7,
                _ => {}
            }
            if state.config.vp9_qsv_preset != old_value {
                state.config.is_modified = true;
            }
        }
        ConfigFocus::Vp9QsvLookaheadCheckbox => {
            if matches!(key.code, KeyCode::Char(' ') | KeyCode::Enter) {
                state.config.vp9_qsv_lookahead = !state.config.vp9_qsv_lookahead;
                state.config.is_modified = true;
            }
        }
        ConfigFocus::Vp9QsvLookaheadDepthInput => {
            let old_value = state.config.vp9_qsv_lookahead_depth;
            match key.code {
                KeyCode::Left => {
                    if state.config.vp9_qsv_lookahead_depth > 0 {
                        state.config.vp9_qsv_lookahead_depth -= 1;
                    }
                }
                KeyCode::Right => {
                    if state.config.vp9_qsv_lookahead_depth < 120 {
                        state.config.vp9_qsv_lookahead_depth += 1;
                    }
                }
                KeyCode::Home => state.config.vp9_qsv_lookahead_depth = 0,
                KeyCode::End => state.config.vp9_qsv_lookahead_depth = 120,
                _ => {}
            }
            if state.config.vp9_qsv_lookahead_depth != old_value {
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
            state.config.gpu_vendor = result.gpu_vendor;
            state.dashboard.gpu_model = result.gpu_model;
            state.dashboard.gpu_vendor = result.gpu_vendor;
            state.dashboard.gpu_available =
                crate::engine::hardware::gpu_monitoring_available(result.gpu_vendor);
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
            let (popup_area, item_count) = match active {
                ConfigFocus::ProfileList => {
                    let item_count = get_profile_count(config);
                    let trigger = config.profile_list_area.unwrap_or_default();
                    let popup =
                        ConfigScreen::calculate_popup_area(trigger, item_count, state.viewport);
                    (popup, item_count)
                }
                ConfigFocus::QualityMode => {
                    let trigger = config.quality_mode_area.unwrap_or_default();
                    let popup = ConfigScreen::calculate_popup_area(trigger, 3, state.viewport);
                    (popup, 3)
                }
                ConfigFocus::ProfileDropdown => {
                    let trigger = config.vp9_profile_list_area.unwrap_or_default();
                    let popup = ConfigScreen::calculate_popup_area(trigger, 4, state.viewport);
                    (popup, 4)
                }
                ConfigFocus::PixFmtDropdown => {
                    let trigger = config.pix_fmt_area.unwrap_or_default();
                    let popup = ConfigScreen::calculate_popup_area(trigger, 3, state.viewport);
                    (popup, 3)
                }
                ConfigFocus::AqModeDropdown => {
                    let trigger = config.aq_mode_area.unwrap_or_default();
                    let popup = ConfigScreen::calculate_popup_area(trigger, 6, state.viewport);
                    (popup, 6)
                }
                ConfigFocus::TuneContentDropdown => {
                    let trigger = config.tune_content_area.unwrap_or_default();
                    let popup = ConfigScreen::calculate_popup_area(trigger, 3, state.viewport);
                    (popup, 3)
                }
                ConfigFocus::AudioPrimaryCodec => {
                    let trigger = config.audio_primary_codec_area.unwrap_or_default();
                    let popup = ConfigScreen::calculate_popup_area(trigger, 5, state.viewport);
                    (popup, 5) // Passthrough, Opus, AAC, MP3, Vorbis
                }
                ConfigFocus::AudioStereoCodec => {
                    let trigger = config.audio_stereo_codec_area.unwrap_or_default();
                    let popup = ConfigScreen::calculate_popup_area(trigger, 2, state.viewport);
                    (popup, 2) // AAC, Opus
                }
                ConfigFocus::ArnrTypeDropdown => {
                    let trigger = config.arnr_type_area.unwrap_or_default();
                    let popup = ConfigScreen::calculate_popup_area(trigger, 4, state.viewport);
                    (popup, 4)
                }
                ConfigFocus::ColorSpacePresetDropdown => {
                    let trigger = config.colorspace_preset_area.unwrap_or_default();
                    let popup = ConfigScreen::calculate_popup_area(trigger, 3, state.viewport);
                    (popup, 3)
                }
                ConfigFocus::FpsDropdown => {
                    let trigger = config.fps_area.unwrap_or_default();
                    let popup = ConfigScreen::calculate_popup_area(trigger, 11, state.viewport);
                    (popup, 11)
                }
                ConfigFocus::ResolutionDropdown => {
                    let trigger = config.scale_width_area.unwrap_or_default();
                    let popup = ConfigScreen::calculate_popup_area(trigger, 7, state.viewport);
                    (popup, 7)
                }
                ConfigFocus::ContainerDropdown => {
                    let trigger = config.container_dropdown_area.unwrap_or_default();
                    let popup = ConfigScreen::calculate_popup_area(trigger, 4, state.viewport);
                    (popup, 4)
                }
                ConfigFocus::VideoCodecDropdown => {
                    let trigger = config.video_codec_area.unwrap_or_default();
                    // Use narrow width for codec dropdown (VP9/AV1 are short)
                    let narrow_trigger = Rect {
                        width: 20,
                        ..trigger
                    };
                    let popup =
                        ConfigScreen::calculate_popup_area(narrow_trigger, 2, state.viewport);
                    (popup, 2)
                }
                ConfigFocus::Av1TuneDropdown => {
                    let trigger = config.av1_tune_area.unwrap_or_default();
                    let popup = ConfigScreen::calculate_popup_area(trigger, 3, state.viewport);
                    (popup, 3)
                }
                ConfigFocus::Av1ScmDropdown => {
                    let trigger = config.av1_scm_area.unwrap_or_default();
                    let popup = ConfigScreen::calculate_popup_area(trigger, 3, state.viewport);
                    (popup, 3)
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
                        match active {
                            ConfigFocus::ColorSpacePresetDropdown => {
                                set_colorspace_preset_selection(config, item_index);
                            }
                            ConfigFocus::ArnrTypeDropdown => {
                                set_arnr_type_selection(config, item_index);
                            }
                            ConfigFocus::FpsDropdown => {
                                set_fps_selection(config, item_index);
                            }
                            ConfigFocus::ResolutionDropdown => {
                                set_resolution_selection(config, item_index);
                            }
                            ConfigFocus::VideoCodecDropdown => {
                                set_video_codec_selection(config, item_index);
                            }
                            _ => {
                                // Other dropdowns only need the selection updated
                                match active {
                                    ConfigFocus::ProfileList => {
                                        config.profile_list_state.select(Some(item_index));
                                    }
                                    ConfigFocus::QualityMode => {
                                        config.quality_mode_state.select(Some(item_index));
                                    }
                                    ConfigFocus::ProfileDropdown => {
                                        config.profile_dropdown_state.select(Some(item_index));
                                    }
                                    ConfigFocus::PixFmtDropdown => {
                                        config.pix_fmt_state.select(Some(item_index));
                                    }
                                    ConfigFocus::AqModeDropdown => {
                                        config.aq_mode_state.select(Some(item_index));
                                    }
                                    ConfigFocus::TuneContentDropdown => {
                                        config.tune_content_state.select(Some(item_index));
                                    }
                                    ConfigFocus::AudioPrimaryCodec => {
                                        config.audio_primary_codec_state.select(Some(item_index));
                                        config.audio_primary_codec =
                                            crate::ui::state::AudioPrimaryCodec::from_index(
                                                item_index,
                                            );
                                    }
                                    ConfigFocus::AudioStereoCodec => {
                                        config.audio_stereo_codec_state.select(Some(item_index));
                                        config.audio_stereo_codec =
                                            crate::ui::state::AudioStereoCodec::from_index(
                                                item_index,
                                            );
                                    }
                                    ConfigFocus::ContainerDropdown => {
                                        config.container_dropdown_state.select(Some(item_index));
                                    }
                                    ConfigFocus::Av1TuneDropdown => {
                                        config.av1_tune_state.select(Some(item_index));
                                    }
                                    ConfigFocus::Av1ScmDropdown => {
                                        config.av1_scm_state.select(Some(item_index));
                                    }
                                    _ => {}
                                }
                            }
                        }
                        if active != ConfigFocus::ProfileList {
                            config.is_modified = true;
                        }
                    }
                }

                // Close popup after selection
                let was_profile_list = active == ConfigFocus::ProfileList;
                let was_codec_dropdown = active == ConfigFocus::VideoCodecDropdown;
                let was_container_dropdown = active == ConfigFocus::ContainerDropdown;
                config.active_dropdown = None;

                // If ProfileList dropdown, load the selected profile after closing
                if was_profile_list {
                    load_selected_profile(state);
                } else if was_codec_dropdown {
                    // Update codec_selection enum based on video_codec_state
                    use crate::ui::state::CodecSelection;
                    let selected = config.video_codec_state.selected().unwrap_or(0);
                    config.codec_selection = match selected {
                        0 => CodecSelection::Vp9,
                        1 => CodecSelection::Av1,
                        _ => CodecSelection::Vp9,
                    };
                    config.is_modified = true;
                } else if was_container_dropdown {
                    // Mark as modified when container changes
                    config.is_modified = true;
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

        if let Some(area) = config.additional_args_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                set_focus_and_update(state, ConfigFocus::AdditionalArgsInput);
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

        // Audio primary codec dropdown
        if let Some(area) = config.audio_primary_codec_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::AudioPrimaryCodec;
                config.active_dropdown = Some(ConfigFocus::AudioPrimaryCodec);
                return;
            }
        }

        // Audio primary bitrate
        if let Some(area) = config.audio_primary_bitrate_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::AudioPrimaryBitrate;
                return;
            }
        }

        // Downmix 2ch checkbox
        if let Some(area) = config.audio_primary_downmix_area {
            if is_in_rect(mouse.column, mouse.row, area)
                && !config.audio_primary_codec.is_passthrough()
            {
                config.focus = ConfigFocus::AudioPrimaryDownmix;
                config.audio_primary_downmix = !config.audio_primary_downmix;
                config.is_modified = true;
                return;
            }
        }

        // AC3 5.1 checkbox
        if let Some(area) = config.audio_ac3_checkbox_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::AudioAc3Checkbox;
                config.audio_add_ac3 = !config.audio_add_ac3;
                config.is_modified = true;
                return;
            }
        }

        // AC3 bitrate
        if let Some(area) = config.audio_ac3_bitrate_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::AudioAc3Bitrate;
                return;
            }
        }

        // Stereo checkbox
        if let Some(area) = config.audio_stereo_checkbox_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::AudioStereoCheckbox;
                config.audio_add_stereo = !config.audio_add_stereo;
                config.is_modified = true;
                return;
            }
        }

        // Stereo codec dropdown
        if let Some(area) = config.audio_stereo_codec_area {
            if is_in_rect(mouse.column, mouse.row, area) && config.audio_add_stereo {
                config.focus = ConfigFocus::AudioStereoCodec;
                config.active_dropdown = Some(ConfigFocus::AudioStereoCodec);
                return;
            }
        }

        // Stereo bitrate
        if let Some(area) = config.audio_stereo_bitrate_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::AudioStereoBitrate;
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
                // Cycle through 0 (disabled), 1 (enabled), 2 (enabled with statistics)
                config.auto_alt_ref = (config.auto_alt_ref + 1) % 3;
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

        if let Some(area) = config.colorspace_preset_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::ColorSpacePresetDropdown;
                config.active_dropdown = Some(ConfigFocus::ColorSpacePresetDropdown);
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

        // Auto-VMAF checkbox
        if let Some(area) = config.auto_vmaf_checkbox_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::AutoVmafCheckbox;
                config.auto_vmaf_enabled = !config.auto_vmaf_enabled;
                config.is_modified = true;
                return;
            }
        }

        // Auto-VMAF target input
        if let Some(area) = config.auto_vmaf_target_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                set_focus_and_update(state, ConfigFocus::AutoVmafTargetInput);
                return;
            }
        }

        // Auto-VMAF step input
        if let Some(area) = config.auto_vmaf_step_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                set_focus_and_update(state, ConfigFocus::AutoVmafStepInput);
                return;
            }
        }

        // Auto-VMAF max attempts input
        if let Some(area) = config.auto_vmaf_max_attempts_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                set_focus_and_update(state, ConfigFocus::AutoVmafMaxAttemptsInput);
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

            // Hardware denoise input
            if let Some(area) = config.hw_denoise_area {
                if is_in_rect(mouse.column, mouse.row, area) {
                    config.focus = ConfigFocus::HwDenoiseInput;
                    return;
                }
            }

            // Hardware detail input
            if let Some(area) = config.hw_detail_area {
                if is_in_rect(mouse.column, mouse.row, area) {
                    config.focus = ConfigFocus::HwDetailInput;
                    return;
                }
            }

            // VP9 QSV preset slider
            if let Some(area) = config.vp9_qsv_preset_area {
                if is_in_rect(mouse.column, mouse.row, area) {
                    config.focus = ConfigFocus::Vp9QsvPresetSlider;
                    // Handle slider click on bar line
                    if mouse.row == area.y + 1
                        && mouse.column >= area.x
                        && mouse.column < area.x + area.width
                    {
                        let click_x = mouse.column.saturating_sub(area.x);
                        let ratio = (click_x as f64) / (area.width as f64).max(1.0);
                        config.vp9_qsv_preset = (ratio * 6.0 + 1.0).clamp(1.0, 7.0) as u32;
                        config.is_modified = true;
                    }
                    return;
                }
            }

            if let Some(area) = config.vp9_qsv_lookahead_checkbox_area {
                if is_in_rect(mouse.column, mouse.row, area) {
                    config.vp9_qsv_lookahead = !config.vp9_qsv_lookahead;
                    config.focus = ConfigFocus::Vp9QsvLookaheadCheckbox;
                    config.is_modified = true;
                    return;
                }
            }

            if let Some(area) = config.vp9_qsv_lookahead_depth_area {
                if is_in_rect(mouse.column, mouse.row, area) {
                    config.focus = ConfigFocus::Vp9QsvLookaheadDepthInput;
                    return;
                }
            }
        }

        // Video codec selector (VP9/AV1)
        if let Some(area) = config.video_codec_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::VideoCodecDropdown;
                config.active_dropdown = Some(ConfigFocus::VideoCodecDropdown);
                return;
            }
        }

        // AV1 software settings
        if let Some(area) = config.av1_preset_slider_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::Av1PresetSlider;
                // Handle slider click on bar line
                if mouse.row == area.y + 1
                    && mouse.column >= area.x
                    && mouse.column < area.x + area.width
                {
                    let click_x = mouse.column.saturating_sub(area.x);
                    let ratio = (click_x as f64) / (area.width as f64).max(1.0);
                    config.av1_preset = (ratio * 13.0).clamp(0.0, 13.0) as u32;
                    config.is_modified = true;
                }
                return;
            }
        }

        if let Some(area) = config.av1_tune_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::Av1TuneDropdown;
                config.active_dropdown = Some(ConfigFocus::Av1TuneDropdown);
                return;
            }
        }

        if let Some(area) = config.av1_film_grain_slider_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::Av1FilmGrainSlider;
                return;
            }
        }

        if let Some(area) = config.av1_film_grain_denoise_checkbox_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.av1_film_grain_denoise = !config.av1_film_grain_denoise;
                config.focus = ConfigFocus::Av1FilmGrainDenoiseCheckbox;
                config.is_modified = true;
                return;
            }
        }

        if let Some(area) = config.av1_enable_overlays_checkbox_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.av1_enable_overlays = !config.av1_enable_overlays;
                config.focus = ConfigFocus::Av1EnableOverlaysCheckbox;
                config.is_modified = true;
                return;
            }
        }

        if let Some(area) = config.av1_scd_checkbox_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.av1_scd = !config.av1_scd;
                config.focus = ConfigFocus::Av1ScdCheckbox;
                config.is_modified = true;
                return;
            }
        }

        if let Some(area) = config.av1_scm_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::Av1ScmDropdown;
                config.active_dropdown = Some(ConfigFocus::Av1ScmDropdown);
                return;
            }
        }

        if let Some(area) = config.av1_enable_tf_checkbox_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.av1_enable_tf = !config.av1_enable_tf;
                config.focus = ConfigFocus::Av1EnableTfCheckbox;
                config.is_modified = true;
                return;
            }
        }

        // AV1 hardware settings
        if let Some(area) = config.av1_hw_preset_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::Av1HwPresetSlider;
                // Handle slider click on bar line
                if mouse.row == area.y + 1
                    && mouse.column >= area.x
                    && mouse.column < area.x + area.width
                {
                    let click_x = mouse.column.saturating_sub(area.x);
                    let ratio = (click_x as f64) / (area.width as f64).max(1.0);
                    config.av1_hw_preset = (ratio * 6.0 + 1.0).clamp(1.0, 7.0) as u32;
                    config.is_modified = true;
                }
                return;
            }
        }

        if let Some(area) = config.av1_hw_cq_slider_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::Av1HwCqSlider;
                // Handle slider click on bar line
                if mouse.row == area.y + 1
                    && mouse.column >= area.x
                    && mouse.column < area.x + area.width
                {
                    // Per-encoder quality: NVENC uses 0-63, QSV/VAAPI use 1-255
                    let (min_cq, max_cq) = match config.gpu_vendor {
                        crate::engine::hardware::GpuVendor::Nvidia => (0, 63),
                        crate::engine::hardware::GpuVendor::Intel => (1, 255),
                        _ => (1, 255), // AMD and others use VAAPI
                    };
                    let click_x = mouse.column.saturating_sub(area.x);
                    let ratio = (click_x as f64) / (area.width as f64).max(1.0);
                    let range = (max_cq - min_cq) as f64;
                    let new_value =
                        (ratio * range + min_cq as f64).clamp(min_cq as f64, max_cq as f64) as u32;
                    // Write to the correct per-encoder field
                    match config.gpu_vendor {
                        crate::engine::hardware::GpuVendor::Nvidia => {
                            config.av1_nvenc_cq = new_value
                        }
                        crate::engine::hardware::GpuVendor::Intel => config.av1_qsv_cq = new_value,
                        _ => config.av1_vaapi_cq = new_value,
                    };
                    config.is_modified = true;
                }
                return;
            }
        }

        if let Some(area) = config.av1_hw_lookahead_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::Av1HwLookaheadInput;
                return;
            }
        }

        if let Some(area) = config.av1_hw_tile_cols_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::Av1HwTileColsInput;
                return;
            }
        }

        if let Some(area) = config.av1_hw_tile_rows_area {
            if is_in_rect(mouse.column, mouse.row, area) {
                config.focus = ConfigFocus::Av1HwTileRowsInput;
            }
        }
    }
}

// Helper function to check if mouse is within table area

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::layout::Rect;

    #[test]
    fn colorspace_preset_key_updates_all_values() {
        let mut state = AppState::default();
        state.config.focus = ConfigFocus::ColorSpacePresetDropdown;

        // Press Down to select SDR preset (index 1)
        handle_focused_widget_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), &mut state);

        // Verify preset state is updated to index 1
        assert_eq!(state.config.colorspace_preset_state.selected(), Some(1));
        // Verify preset enum is SDR
        assert_eq!(
            state.config.colorspace_preset,
            crate::ui::state::ColorSpacePreset::Sdr
        );
        // Verify ALL numeric values match SDR preset (1, 1, 1, 0)
        assert_eq!(state.config.colorspace, 1);
        assert_eq!(state.config.color_primaries, 1);
        assert_eq!(state.config.color_trc, 1);
        assert_eq!(state.config.color_range, 0);
    }

    #[test]
    fn fps_key_updates_numeric_value() {
        let mut state = AppState::default();
        state.config.focus = ConfigFocus::FpsDropdown;

        handle_focused_widget_key(
            KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
            &mut state,
        );

        assert_eq!(state.config.fps_dropdown_state.selected(), Some(1));
        assert_eq!(state.config.fps, options::fps_from_idx(1));
    }

    #[test]
    fn resolution_key_updates_width_and_height() {
        let mut state = AppState::default();
        state.config.focus = ConfigFocus::ResolutionDropdown;

        handle_focused_widget_key(
            KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
            &mut state,
        );

        assert_eq!(state.config.resolution_dropdown_state.selected(), Some(1));
        assert_eq!(
            (state.config.scale_width, state.config.scale_height),
            options::resolution_from_idx(1)
        );
    }

    #[test]
    fn video_codec_key_updates_enum() {
        let mut state = AppState::default();
        state.config.focus = ConfigFocus::VideoCodecDropdown;

        handle_focused_widget_key(
            KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
            &mut state,
        );

        assert_eq!(state.config.video_codec_state.selected(), Some(1));
        assert_eq!(
            state.config.codec_selection,
            crate::ui::state::CodecSelection::Av1
        );
    }

    #[test]
    fn arnr_type_key_updates_numeric_value() {
        let mut state = AppState::default();
        state.config.focus = ConfigFocus::ArnrTypeDropdown;

        handle_focused_widget_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), &mut state);

        assert_eq!(state.config.arnr_type_state.selected(), Some(1));
        assert_eq!(state.config.arnr_type, options::arnr_type_from_idx(1));
    }

    #[test]
    fn quality_mode_key_updates_selection() {
        let mut state = AppState::default();
        state.config.focus = ConfigFocus::QualityMode;
        state.config.is_modified = false;

        handle_focused_widget_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), &mut state);

        assert_eq!(state.config.quality_mode_state.selected(), Some(1));
        assert!(state.config.is_modified);
    }

    #[test]
    fn tune_content_key_updates_selection() {
        let mut state = AppState::default();
        state.config.focus = ConfigFocus::TuneContentDropdown;
        state.config.is_modified = false;

        handle_focused_widget_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), &mut state);

        assert_eq!(state.config.tune_content_state.selected(), Some(1));
        assert!(state.config.is_modified);
    }

    #[test]
    fn aq_mode_key_updates_selection() {
        let mut state = AppState::default();
        state.config.focus = ConfigFocus::AqModeDropdown;
        state.config.is_modified = false;
        state.config.aq_mode_state.select(Some(0));

        handle_focused_widget_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), &mut state);

        assert_eq!(state.config.aq_mode_state.selected(), Some(1));
        assert!(state.config.is_modified);
    }

    #[test]
    fn pix_fmt_key_updates_selection() {
        let mut state = AppState::default();
        state.config.focus = ConfigFocus::PixFmtDropdown;
        state.config.is_modified = false;

        handle_focused_widget_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), &mut state);

        assert_eq!(state.config.pix_fmt_state.selected(), Some(1));
        assert!(state.config.is_modified);
    }

    #[test]
    fn audio_primary_codec_key_updates_selection() {
        let mut state = AppState::default();
        state.config.focus = ConfigFocus::AudioPrimaryCodec;
        state.config.is_modified = false;
        // Start at Opus (index 1), pressing down should go to AAC (index 2)
        state.config.audio_primary_codec_state.select(Some(1));

        handle_focused_widget_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), &mut state);

        assert_eq!(state.config.audio_primary_codec_state.selected(), Some(2));
        assert!(state.config.is_modified);
    }

    #[test]
    fn container_key_updates_selection() {
        let mut state = AppState::default();
        state.config.focus = ConfigFocus::ContainerDropdown;
        state.config.is_modified = false;

        handle_focused_widget_key(
            KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
            &mut state,
        );

        assert_eq!(state.config.container_dropdown_state.selected(), Some(1));
        assert!(state.config.is_modified);
    }

    #[test]
    fn mouse_selects_colorspace_preset_and_updates_numeric() {
        let mut state = AppState::default();
        state.viewport = Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 24,
        };
        state.config.active_dropdown = Some(ConfigFocus::ColorSpacePresetDropdown);
        state.config.colorspace_preset_area = Some(Rect {
            x: 0,
            y: 0,
            width: 20,
            height: 1,
        });

        let popup = crate::ui::ConfigScreen::calculate_popup_area(
            state.config.colorspace_preset_area.unwrap(),
            3,
            state.viewport,
        );

        // Click on the second item (index 1 = SDR preset)
        let event = crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: popup.x + 1,
            row: popup.y + 2,
            modifiers: KeyModifiers::NONE,
        };

        handle_config_mouse(event, &mut state);

        // Verify preset state is updated
        assert_eq!(state.config.colorspace_preset_state.selected(), Some(1));
        // Verify preset enum is SDR
        assert_eq!(
            state.config.colorspace_preset,
            crate::ui::state::ColorSpacePreset::Sdr
        );
        // Verify numeric values match SDR preset (1, 1, 1, 0)
        assert_eq!(state.config.colorspace, 1);
        assert_eq!(state.config.color_primaries, 1);
        assert_eq!(state.config.color_trc, 1);
        assert_eq!(state.config.color_range, 0);
        assert!(state.config.is_modified);
        assert!(state.config.active_dropdown.is_none());
    }

    #[test]
    fn vaapi_b_frames_arrow_keys_adjust_value() {
        let mut state = AppState::default();
        state.config.focus = ConfigFocus::VaapiBFramesInput;
        state.config.is_modified = false;
        state.config.vaapi_b_frames = "0".to_string();

        handle_focused_widget_key(
            KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
            &mut state,
        );
        assert_eq!(state.config.vaapi_b_frames, "1");
        assert!(state.config.is_modified);
    }

    #[test]
    fn vaapi_loop_filter_level_arrow_keys_adjust_value() {
        let mut state = AppState::default();
        state.config.focus = ConfigFocus::VaapiLoopFilterLevelInput;
        state.config.is_modified = false;
        state.config.vaapi_loop_filter_level = "16".to_string();

        handle_focused_widget_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE), &mut state);
        assert_eq!(state.config.vaapi_loop_filter_level, "15");
        assert!(state.config.is_modified);
    }

    #[test]
    fn vaapi_loop_filter_sharpness_arrow_keys_adjust_value() {
        let mut state = AppState::default();
        state.config.focus = ConfigFocus::VaapiLoopFilterSharpnessInput;
        state.config.is_modified = false;
        state.config.vaapi_loop_filter_sharpness = "4".to_string();

        handle_focused_widget_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE), &mut state);
        assert_eq!(state.config.vaapi_loop_filter_sharpness, "15");
        assert!(state.config.is_modified);
    }
}
