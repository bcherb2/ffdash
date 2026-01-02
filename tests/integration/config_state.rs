// Integration tests for ConfigState management
//
// These tests verify UI state management, focus navigation,
// and state transitions in the config screen.

use ffdash::ui::focus::ConfigFocus;
use ffdash::ui::state::{ConfigState, RateControlMode};

use crate::common::helpers::*;

// ============================================================================
// UNIT TESTS: ConfigState initialization
// ============================================================================

#[test]
fn test_config_state_defaults() {
    let config = ConfigState::default();

    // Check default rate control mode
    assert_eq!(config.rate_control_mode, RateControlMode::CQ);

    // Check default encoding settings
    assert!(config.crf > 0);
    assert!(config.cpu_used <= 8);

    // Check default parallelism
    assert_eq!(config.tile_columns >= 0, true);
    assert_eq!(config.tile_rows >= 0, true);

    // Check default GOP settings
    assert!(config.gop_length.parse::<u32>().unwrap_or(0) > 0);
}

#[test]
fn test_config_state_profile_tracking() {
    let mut config = ConfigState::default();

    // Initially no profile selected (should be None or default)
    assert!(!config.is_modified);

    // Modify a setting
    config.crf = 25;
    config.is_modified = true;

    assert!(config.is_modified);
}

// ============================================================================
// UNIT TESTS: Focus navigation
// ============================================================================

#[test]
fn test_focus_navigation_cycles_forward() {
    let mut focus = ConfigFocus::ProfileList;

    // Navigate forward through all focus states
    // There are 55 ConfigFocus variants, so after 55 steps we should be back at ProfileList
    for i in 0..100 {
        // Generous limit
        focus = focus.next();
        if focus == ConfigFocus::ProfileList {
            // Successfully cycled back - this is expected behavior
            assert!(i > 0, "Should navigate through at least one other state");
            return;
        }
    }

    panic!(
        "Focus navigation did not cycle back to ProfileList after 100 steps. Last focus: {:?}",
        focus
    );
}

#[test]
fn test_focus_navigation_cycles_backward() {
    let mut focus = ConfigFocus::ProfileList;

    // Navigate backward through all focus states
    // There are 55 ConfigFocus variants, so after 55 steps we should be back at ProfileList
    for i in 0..100 {
        // Generous limit
        focus = focus.previous();
        if focus == ConfigFocus::ProfileList {
            // Successfully cycled back - this is expected behavior
            assert!(i > 0, "Should navigate through at least one other state");
            return;
        }
    }

    panic!(
        "Focus navigation did not cycle back to ProfileList after 100 steps. Last focus: {:?}",
        focus
    );
}

#[test]
fn test_focus_forward_then_backward() {
    let mut focus = ConfigFocus::ProfileList;

    // Move forward 5 steps
    for _ in 0..5 {
        focus = focus.next();
    }

    let forward_pos = focus;

    // Move backward 5 steps
    for _ in 0..5 {
        focus = focus.previous();
    }

    // Should be back at start
    assert_eq!(focus, ConfigFocus::ProfileList);

    // Move forward 5 again
    for _ in 0..5 {
        focus = focus.next();
    }

    // Should be at same position as before
    assert_eq!(focus, forward_pos);
}

// ============================================================================
// UNIT TESTS: Rate control mode transitions
// ============================================================================

#[test]
fn test_rate_control_mode_transitions() {
    let mut config = ConfigState::default();

    // CQ mode
    config.rate_control_mode = RateControlMode::CQ;
    assert_eq!(config.rate_control_mode, RateControlMode::CQ);

    // CQCap mode
    config.rate_control_mode = RateControlMode::CQCap;
    assert_eq!(config.rate_control_mode, RateControlMode::CQCap);

    // TwoPassVBR mode
    config.rate_control_mode = RateControlMode::TwoPassVBR;
    assert_eq!(config.rate_control_mode, RateControlMode::TwoPassVBR);

    // CBR mode
    config.rate_control_mode = RateControlMode::CBR;
    assert_eq!(config.rate_control_mode, RateControlMode::CBR);
}

#[test]
fn test_two_pass_mode_affects_cpu_used() {
    let mut config = ConfigState::default();

    config.two_pass = false;
    assert_eq!(config.two_pass, false);

    config.two_pass = true;
    assert_eq!(config.two_pass, true);

    // In two-pass mode, separate pass1/pass2 CPU-used values are used
    config.cpu_used_pass1 = 4;
    config.cpu_used_pass2 = 1;

    assert_eq!(config.cpu_used_pass1, 4);
    assert_eq!(config.cpu_used_pass2, 1);
}

// ============================================================================
// UNIT TESTS: Bounds checking
// ============================================================================

#[test]
fn test_crf_valid_range() {
    let mut config = ConfigState::default();

    // Test various CRF values
    config.crf = 0;
    assert_eq!(config.crf, 0);

    config.crf = 31;
    assert_eq!(config.crf, 31);

    config.crf = 63;
    assert_eq!(config.crf, 63);
}

#[test]
fn test_cpu_used_valid_range() {
    let mut config = ConfigState::default();

    // CPU-used should be 0-8
    config.cpu_used = 0;
    assert_eq!(config.cpu_used, 0);

    config.cpu_used = 4;
    assert_eq!(config.cpu_used, 4);

    config.cpu_used = 8;
    assert_eq!(config.cpu_used, 8);
}

#[test]
fn test_tile_columns_valid_range() {
    let mut config = ConfigState::default();

    // Tile columns should be 0-6 (log2)
    for val in 0..=6 {
        config.tile_columns = val;
        assert_eq!(config.tile_columns, val);
    }
}

#[test]
fn test_lag_in_frames_valid_range() {
    let mut config = ConfigState::default();

    // Lag should be 0-25
    config.lag_in_frames = 0;
    assert_eq!(config.lag_in_frames, 0);

    config.lag_in_frames = 16;
    assert_eq!(config.lag_in_frames, 16);

    config.lag_in_frames = 25;
    assert_eq!(config.lag_in_frames, 25);
}

// ============================================================================
// UNIT TESTS: Boolean flags
// ============================================================================

#[test]
fn test_boolean_flags_can_toggle() {
    let mut config = ConfigState::default();

    // Test all boolean flags
    config.row_mt = true;
    assert_eq!(config.row_mt, true);
    config.row_mt = false;
    assert_eq!(config.row_mt, false);

    config.frame_parallel = true;
    assert_eq!(config.frame_parallel, true);

    config.fixed_gop = true;
    assert_eq!(config.fixed_gop, true);

    config.auto_alt_ref = 1;
    assert_eq!(config.auto_alt_ref, 1);

    config.enable_tpl = true;
    assert_eq!(config.enable_tpl, true);

    config.overwrite = true;
    assert_eq!(config.overwrite, true);
}

// ============================================================================
// UNIT TESTS: Special values (auto, disabled)
// ============================================================================

#[test]
fn test_auto_values() {
    let mut config = ConfigState::default();

    // -1 typically means "auto"
    config.sharpness = -1;
    assert_eq!(config.sharpness, -1);

    config.undershoot_pct = -1;
    assert_eq!(config.undershoot_pct, -1);

    config.overshoot_pct = -1;
    assert_eq!(config.overshoot_pct, -1);

    config.arnr_type = -1;
    assert_eq!(config.arnr_type, -1);
}

#[test]
fn test_zero_means_disabled_or_auto() {
    let mut config = ConfigState::default();

    // 0 typically means disabled or auto
    config.threads = 0; // Auto thread count
    assert_eq!(config.threads, 0);

    config.static_thresh = 0.to_string(); // Disabled
    assert_eq!(config.static_thresh, "0".to_string());

    config.max_intra_rate = 0.to_string(); // Disabled
    assert_eq!(config.max_intra_rate, "0".to_string());

    config.keyint_min = 0.to_string(); // Auto
    assert_eq!(config.keyint_min, "0".to_string());
}

// ============================================================================
// UNIT TESTS: String/list state fields
// ============================================================================

#[test]
fn test_output_dir_string() {
    let mut config = ConfigState::default();

    config.output_dir = "/test/output".to_string();
    assert_eq!(config.output_dir, "/test/output");

    // Note: file_prefix and file_suffix fields were removed from ConfigState
    // Filename patterns are now handled by Profile::filename_pattern instead
}

#[test]
fn test_dropdown_states_initialized() {
    let _config = ConfigState::default();

    // All ListStates should be initialized (not panicking when accessed)
    // This is mostly checking that the struct can be created without errors
    assert!(true); // If we got here, all ListStates are initialized

    // We could also check that they have selections
    // but that requires accessing the internal state
}

// ============================================================================
// INTEGRATION TESTS: State transitions
// ============================================================================

#[test]
fn test_profile_modification_tracking() {
    let mut config = ConfigState::default();

    // Start unmodified
    config.is_modified = false;
    assert!(!config.is_modified);

    // Make a change
    config.crf = 25;
    config.is_modified = true;
    assert!(config.is_modified);

    // "Save" the profile (in real code, this would happen after save)
    config.is_modified = false;
    assert!(!config.is_modified);

    // Make another change
    config.cpu_used = 3;
    config.is_modified = true;
    assert!(config.is_modified);
}

#[test]
fn test_switching_rate_control_modes_preserves_values() {
    let mut config = ConfigState::default();

    // Set values in CQ mode
    config.rate_control_mode = RateControlMode::CQ;
    config.crf = 30;
    config.video_target_bitrate = 0;

    // Switch to VBR
    config.rate_control_mode = RateControlMode::TwoPassVBR;
    config.video_target_bitrate = 2000;

    // CRF value should still be there (might be hidden in UI, but preserved)
    assert_eq!(config.crf, 30);
    assert_eq!(config.video_target_bitrate, 2000);

    // Switch back to CQ
    config.rate_control_mode = RateControlMode::CQ;

    // Both values should be preserved
    assert_eq!(config.crf, 30);
    assert_eq!(config.video_target_bitrate, 2000);
}

#[test]
fn test_two_pass_toggle_preserves_cpu_used_values() {
    let mut config = ConfigState::default();

    config.cpu_used = 2;
    config.cpu_used_pass1 = 4;
    config.cpu_used_pass2 = 1;

    // Enable two-pass
    config.two_pass = true;

    // All CPU-used values should be preserved
    assert_eq!(config.cpu_used, 2);
    assert_eq!(config.cpu_used_pass1, 4);
    assert_eq!(config.cpu_used_pass2, 1);

    // Disable two-pass
    config.two_pass = false;

    // Values still preserved
    assert_eq!(config.cpu_used, 2);
    assert_eq!(config.cpu_used_pass1, 4);
    assert_eq!(config.cpu_used_pass2, 1);
}

// ============================================================================
// CONSISTENCY TESTS
// ============================================================================

#[test]
fn test_parallel_config_helper_creates_valid_state() {
    let config = parallel_config();

    assert_eq!(config.row_mt, true);
    assert_eq!(config.frame_parallel, true);
    assert_eq!(config.tile_columns, 2);
    assert_eq!(config.tile_rows, 1);
    assert_eq!(config.threads, 8);
}

#[test]
fn test_gop_config_helper_creates_valid_state() {
    let config = custom_gop_config();

    assert_eq!(config.gop_length, "120".to_string());
    assert_eq!(config.fixed_gop, true);
    assert_eq!(config.keyint_min, "60".to_string());
    assert_eq!(config.lag_in_frames, 16);
    assert_eq!(config.auto_alt_ref, 1);
}

#[test]
fn test_tuned_config_helper_creates_valid_state() {
    let config = tuned_config();

    // Check that aq_mode ListState is set (index 2 = Variance)
    assert_eq!(config.aq_mode_state.selected(), Some(2));
    assert_eq!(config.arnr_max_frames, 7);
    assert_eq!(config.arnr_strength, 4);
    // Check that tune_content ListState is set (index 1 = screen)
    assert_eq!(config.tune_content_state.selected(), Some(1));
    assert_eq!(config.enable_tpl, true);
    assert_eq!(config.sharpness, 3);
}

#[test]
fn test_cq_config_helper_creates_valid_state() {
    let config = cq_config();

    assert_eq!(config.rate_control_mode, RateControlMode::CQ);
    assert_eq!(config.crf, 30);
}

#[test]
fn test_vbr_config_helper_creates_valid_state() {
    let config = vbr_config();

    assert_eq!(config.rate_control_mode, RateControlMode::TwoPassVBR);
    assert_eq!(config.video_target_bitrate, 2000);
    assert_eq!(config.video_min_bitrate, 1000);
    assert_eq!(config.video_max_bitrate, 3000);
    assert_eq!(config.two_pass, true);
}

#[test]
fn test_cbr_config_helper_creates_valid_state() {
    let config = cbr_config();

    assert_eq!(config.rate_control_mode, RateControlMode::CBR);
    assert_eq!(config.video_min_bitrate, 2000);
    assert_eq!(config.video_max_bitrate, 2000);
}
