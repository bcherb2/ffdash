//! Profile management operations for config screen.
//!
//! Handles loading, saving, and deleting encoding profiles.

use crate::ui::state::AppState;

pub(super) fn get_profile_count(config: &crate::ui::state::ConfigState) -> usize {
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

pub(super) fn delete_profile(state: &mut AppState, name: String) {
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

pub(super) fn save_profile_with_name(state: &mut AppState, name: String) {
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

pub(super) fn load_selected_profile(state: &mut AppState) {
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
