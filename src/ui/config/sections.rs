use super::*;
use crate::ui::constants;

impl ConfigScreen {
    pub(super) fn render_profile_bar(frame: &mut Frame, area: Rect, state: &mut ConfigState) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Profile")
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Horizontal split: Profile dropdown (60%) | Buttons (40%)
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(inner);

        // Profile dropdown
        state.profile_list_area = Some(chunks[0]);

        // Build profile list: built-in + saved + Custom (if modified) + Create New...
        let mut profiles = Vec::new();
        use crate::engine::Profile;

        // Add built-in profiles
        profiles.extend(Profile::builtin_names());

        // Add saved profiles (excluding built-ins to avoid duplicates)
        for saved_profile in &state.available_profiles {
            if !Profile::builtin_names().contains(saved_profile) {
                profiles.push(saved_profile.clone());
            }
        }

        // Add "Custom" if modified
        if state.is_modified {
            profiles.push("Custom".to_string());
        }

        // Always add "Create New..."
        profiles.push("Create New...".to_string());

        // Determine what to display
        let display_value = if state.is_modified {
            "Custom".to_string()
        } else if let Some(ref name) = state.current_profile_name {
            name.clone()
        } else {
            "Custom".to_string()
        };

        let selected_index = state.profile_list_state.selected().unwrap_or(0);
        let selected_value = profiles.get(selected_index).unwrap_or(&display_value);
        let profile_style = if state.focus == ConfigFocus::ProfileList {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        let profile_line = Line::from(vec![
            Span::raw("Profile: "),
            Span::styled(selected_value.clone(), profile_style),
            Span::raw(" ▼"),
        ]);
        frame.render_widget(Paragraph::new(profile_line), chunks[0]);

        // Buttons
        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

        state.save_button_area = Some(button_chunks[0]);
        state.delete_button_area = Some(button_chunks[1]);

        render_button(
            "Save",
            "Ctrl+S",
            state.focus == ConfigFocus::SaveButton,
            button_chunks[0],
            frame.buffer_mut(),
        );
        render_button(
            "Delete",
            "Ctrl+D",
            state.focus == ConfigFocus::DeleteButton,
            button_chunks[1],
            frame.buffer_mut(),
        );
    }

    pub(super) fn render_general_audio(frame: &mut Frame, area: Rect, state: &mut ConfigState) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("General & Audio I/O")
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut y = inner.y;

        // Output Directory
        let output_dir_area = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        };
        state.output_dir_area = Some(output_dir_area);
        let dir_style = if state.focus == ConfigFocus::OutputDirectory {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        let dir_text = if state.focus == ConfigFocus::OutputDirectory {
            Self::insert_cursor(&state.output_dir, state.cursor_pos)
        } else {
            state.output_dir.clone()
        };
        let dir_line = Line::from(vec![
            Span::raw("Output Dir: "),
            Span::styled(dir_text, dir_style),
        ]);
        frame.render_widget(Paragraph::new(dir_line), output_dir_area);
        y += 1;

        // Output Filename Pattern
        let pattern_area = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        };
        state.filename_pattern_area = Some(pattern_area);
        let pattern_style = if state.focus == ConfigFocus::FilenamePattern {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        let pattern_value = state.filename_pattern.as_deref().unwrap_or("{basename}");
        let pattern_text = if state.focus == ConfigFocus::FilenamePattern {
            Self::insert_cursor(pattern_value, state.cursor_pos)
        } else {
            pattern_value.to_string()
        };
        let pattern_line = Line::from(vec![
            Span::raw("Output Pattern: "),
            Span::styled(pattern_text, pattern_style),
        ]);
        frame.render_widget(Paragraph::new(pattern_line), pattern_area);
        y += 1;

        // Container Extension Dropdown
        let container_area = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        };
        state.container_dropdown_area = Some(container_area);
        let container_idx = state.container_dropdown_state.selected().unwrap_or(0);
        let container_value = constants::CONTAINER_FORMATS
            .get(container_idx)
            .unwrap_or(&"webm");
        let container_style = if state.focus == ConfigFocus::ContainerDropdown {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        let container_line = Line::from(vec![
            Span::raw("Container: "),
            Span::styled(*container_value, container_style),
            Span::raw(" ▼"),
        ]);
        frame.render_widget(Paragraph::new(container_line), container_area);
        y += 2;

        // Video Output constraints separator
        let video_separator = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("─ "),
                Span::styled("Video Output", Style::default().fg(Color::Cyan)),
                Span::raw(" ─"),
            ])),
            video_separator,
        );
        y += 1;

        // FPS and Resolution dropdowns on one line
        let output_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            });

        // FPS dropdown
        state.fps_area = Some(output_chunks[0]);
        let fps_idx = state.fps_dropdown_state.selected().unwrap_or(0);
        let fps_value = FPS_OPTIONS.get(fps_idx).unwrap_or(&"Source");
        let fps_style = if state.focus == ConfigFocus::FpsDropdown {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("FPS: "),
                Span::styled(*fps_value, fps_style),
                Span::raw(" ▼"),
            ])),
            output_chunks[0],
        );

        // Resolution dropdown
        state.scale_width_area = Some(output_chunks[1]);
        state.scale_height_area = Some(output_chunks[1]);
        let res_idx = state.resolution_dropdown_state.selected().unwrap_or(0);
        let res_value = RESOLUTION_OPTIONS.get(res_idx).unwrap_or(&"Source");
        let res_style = if state.focus == ConfigFocus::ResolutionDropdown {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("Resolution: "),
                Span::styled(*res_value, res_style),
                Span::raw(" ▼"),
            ])),
            output_chunks[1],
        );
        y += 2;

        // Overwrite checkbox
        let overwrite_area = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        };
        state.overwrite_checkbox_area = Some(overwrite_area);
        render_checkbox(
            "Overwrite Existing",
            state.overwrite,
            state.focus == ConfigFocus::OverwriteCheckbox,
            overwrite_area,
            frame.buffer_mut(),
        );
        y += 2;

        // Audio Settings separator
        let separator_area = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("─ "),
                Span::styled("Audio Settings", Style::default().fg(Color::Cyan)),
                Span::raw(" ─"),
            ])),
            separator_area,
        );
        y += 1;

        // Audio codec
        let codec_area = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        };
        state.codec_list_area = Some(codec_area);
        let selected_index = state.codec_list_state.selected().unwrap_or(0);
        let selected_value = AUDIO_CODECS.get(selected_index).unwrap_or(&"libopus");
        let codec_style = if state.focus == ConfigFocus::AudioCodec {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        let codec_line = Line::from(vec![
            Span::raw("Codec: "),
            Span::styled(*selected_value, codec_style),
            Span::raw(" ▼"),
        ]);
        frame.render_widget(Paragraph::new(codec_line), codec_area);
        y += 1;

        // Downmix to stereo checkbox
        let stereo_area = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        };
        state.force_stereo_checkbox_area = Some(stereo_area);
        render_checkbox(
            "Downmix to Stereo",
            state.force_stereo,
            state.focus == ConfigFocus::ForceStereoCheckbox,
            stereo_area,
            frame.buffer_mut(),
        );
        y += 2;

        // Audio bitrate slider
        let bitrate_area = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 2,
        };
        state.audio_bitrate_slider_area = Some(bitrate_area);
        let audio_bitrate_slider = Slider::new("Audio Bitrate (kbps)", 32, 512)
            .value(state.audio_bitrate)
            .focused(state.focus == ConfigFocus::AudioBitrateSlider);
        frame.render_widget(audio_bitrate_slider, bitrate_area);
    }

    pub(super) fn render_core_video(frame: &mut Frame, area: Rect, state: &mut ConfigState) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Core Video Encoding")
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut y = inner.y;

        // Hardware Encoding toggle (first item - top of Core Video section)
        let hw_label = if cfg!(target_os = "linux") {
            "Use Hardware Encoding (Intel Quick Sync - 7th gen+)"
        } else {
            "Use Hardware Encoding (Linux only)"
        };

        let hw_enabled = cfg!(target_os = "linux") && state.use_hardware_encoding;
        let hw_available = cfg!(target_os = "linux");

        let hw_area = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        };
        state.hw_encoding_checkbox_area = Some(hw_area);

        // Render checkbox - greyed out on non-Linux
        let checkbox_char = if hw_enabled { '✓' } else { ' ' };
        let checkbox_style = if !hw_available {
            Style::default().fg(Color::DarkGray)
        } else if state.focus == ConfigFocus::HardwareEncodingCheckbox {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else if hw_enabled {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        };

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("[{}] ", checkbox_char), checkbox_style),
                Span::styled(hw_label, checkbox_style),
            ])),
            hw_area,
        );
        y += 1;

        // Status message (if HW encoding checked/attempted)
        if let Some(msg) = &state.hw_availability_message {
            let status_style = if state.hw_encoding_available == Some(true) {
                Style::default().fg(Color::Green)
            } else if msg.contains("using software encoding") {
                // Fallback message - show as warning (yellow) not error (red)
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Red)
            };
            frame.render_widget(
                Paragraph::new(format!("  Status: {}", msg)).style(status_style),
                Rect {
                    x: inner.x,
                    y,
                    width: inner.width,
                    height: 1,
                },
            );
            y += 1;
        }

        // Codec and Pix Fmt on one line
        let codec_pix_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            });

        // VP9 Profile dropdown
        state.vp9_profile_list_area = Some(codec_pix_chunks[0]);
        let selected_index = state.profile_dropdown_state.selected().unwrap_or(0);
        let selected_value = VP9_PROFILES.get(selected_index).unwrap_or(&"VP9 (8-bit)");
        let codec_style = if state.focus == ConfigFocus::ProfileDropdown {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("Codec: "),
                Span::styled(*selected_value, codec_style),
                Span::raw(" ▼"),
            ])),
            codec_pix_chunks[0],
        );

        // Pixel format dropdown
        state.pix_fmt_area = Some(codec_pix_chunks[1]);
        let selected_index = state.pix_fmt_state.selected().unwrap_or(0);
        let selected_value = PIX_FMTS.get(selected_index).unwrap_or(&"yuv420p");
        let pix_style = if state.focus == ConfigFocus::PixFmtDropdown {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("Pix Fmt: "),
                Span::styled(*selected_value, pix_style),
                Span::raw(" ▼"),
            ])),
            codec_pix_chunks[1],
        );
        y += 1;

        // Two-pass checkbox (only for software encoding)
        if !state.use_hardware_encoding {
            let two_pass_area = Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            };
            state.two_pass_checkbox_area = Some(two_pass_area);
            render_checkbox(
                "2-Pass Encoding",
                state.two_pass,
                state.focus == ConfigFocus::TwoPassCheckbox,
                two_pass_area,
                frame.buffer_mut(),
            );
        }

        // Rate Control section (for both software and hardware encoding)
        y += 2;

        let rc_separator = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("─ "),
                Span::styled("Rate Control", Style::default().fg(Color::Cyan)),
                Span::raw(" ─"),
            ])),
            rc_separator,
        );
        y += 1;

        // Rate control mode radio buttons
        let mode_area = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        };
        state.rate_control_mode_area = Some(mode_area);

        // Different labels for hardware vs software encoding
        let (labels, mode_index) = if state.use_hardware_encoding {
            // Hardware VAAPI: CQP only (other modes removed due to Arc driver issues)
            (&["CQP"][..], 0)
        } else {
            // Software encoder modes
            let index = match state.rate_control_mode {
                RateControlMode::CQ => 0,
                RateControlMode::CQCap => 1,
                RateControlMode::TwoPassVBR => 2,
                RateControlMode::CBR => 3,
            };
            (&["CQ", "CQ+Cap", "VBR", "CBR"][..], index)
        };

        render_radio_group(
            labels,
            mode_index,
            state.focus == ConfigFocus::RateControlMode,
            mode_area,
            frame.buffer_mut(),
        );
        y += 1;

        // Clear rate control-related areas to prevent stale areas from previous modes
        state.crf_slider_area = None;
        state.qsv_quality_slider_area = None;
        state.vaapi_compression_level_slider_area = None;
        state.video_target_bitrate_area = None;
        state.video_bufsize_area = None;
        state.video_min_bitrate_area = None;
        state.video_max_bitrate_area = None;
        state.quality_mode_area = None; // Clear software-only quality mode dropdown
        state.cpu_used_slider_area = None;
        state.cpu_used_pass1_slider_area = None;
        state.cpu_used_pass2_slider_area = None;

        // Clear software-only tuning/filter areas (from render_tuning_filters)
        state.aq_mode_area = None;
        state.arnr_max_frames_slider_area = None;
        state.arnr_strength_slider_area = None;
        state.tune_content_area = None;
        state.sharpness_slider_area = None;
        state.enable_tpl_checkbox_area = None;
        state.noise_sensitivity_slider_area = None;
        state.arnr_type_area = None;

        // Clear software-only GOP/keyframe areas (from render_gop_keyframes)
        state.gop_length_area = None;
        state.fixed_gop_checkbox_area = None;
        state.keyint_min_area = None;
        state.lag_in_frames_slider_area = None;
        state.auto_alt_ref_checkbox_area = None;

        // Clear software-only parallelism areas (from render_parallelism)
        state.row_mt_checkbox_area = None;
        state.frame_parallel_checkbox_area = None;
        state.tile_columns_slider_area = None;
        state.tile_rows_slider_area = None;
        state.threads_area = None;

        // Quality slider (CRF for software, global_quality for hardware CQP mode)
        if state.use_hardware_encoding && state.vaapi_rc_mode == "1" {
            let quality_area = Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 2,
            };
            state.qsv_quality_slider_area = Some(quality_area);
            let quality_slider = Slider::new("Quality (1-255, lower=better)", 1, 255)
                .value(state.qsv_global_quality)
                .focused(state.focus == ConfigFocus::QsvGlobalQualitySlider);
            frame.render_widget(quality_slider, quality_area);
            y += 3;
        }

        // Compression level slider (always shown for hardware)
        if state.use_hardware_encoding {
            let compression_area = Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 2,
            };
            state.vaapi_compression_level_slider_area = Some(compression_area);
            let compression_val = state.vaapi_compression_level.parse::<u32>().unwrap_or(4);
            let compression_slider = Slider::new(
                "Compression Level (0-7, 0=slowest/best 7=fastest/worst)",
                0,
                7,
            )
            .value(compression_val)
            .focused(state.focus == ConfigFocus::VaapiCompressionLevelSlider);
            frame.render_widget(compression_slider, compression_area);
            y += 3;

            // Hardware: No bitrate inputs (CQP mode only)
            // VBR/CBR modes removed
            if false {
                // Disabled
                // Target bitrate
                let target_area = Rect {
                    x: inner.x,
                    y,
                    width: inner.width,
                    height: 1,
                };
                state.video_target_bitrate_area = Some(target_area);
                let target_base = if state.video_target_bitrate == 0 {
                    "0 kbps".to_string()
                } else {
                    format!("{} kbps", state.video_target_bitrate)
                };
                let target_text = if state.focus == ConfigFocus::VideoTargetBitrateInput {
                    Self::insert_cursor(&target_base, state.cursor_pos)
                } else {
                    target_base
                };
                let target_style = if state.focus == ConfigFocus::VideoTargetBitrateInput {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Green)
                };
                frame.render_widget(
                    Paragraph::new(Line::from(vec![
                        Span::raw("Target Bitrate: "),
                        Span::raw("["),
                        Span::styled(target_text, target_style),
                        Span::raw("]"),
                    ])),
                    target_area,
                );
                y += 1;

                // Max bitrate
                let max_area = Rect {
                    x: inner.x,
                    y,
                    width: inner.width,
                    height: 1,
                };
                state.video_max_bitrate_area = Some(max_area);
                let max_base = if state.video_max_bitrate == 0 {
                    "None".to_string()
                } else {
                    format!("{} kbps", state.video_max_bitrate)
                };
                let max_text = if state.focus == ConfigFocus::VideoMaxBitrateInput {
                    Self::insert_cursor(&max_base, state.cursor_pos)
                } else {
                    max_base
                };
                let max_style = if state.focus == ConfigFocus::VideoMaxBitrateInput {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Green)
                };
                frame.render_widget(
                    Paragraph::new(Line::from(vec![
                        Span::raw("Max Bitrate: "),
                        Span::raw("["),
                        Span::styled(max_text, max_style),
                        Span::raw("]"),
                    ])),
                    max_area,
                );
                y += 1;
            }

            // CBR mode removed
            if false {
                // Disabled
                // Target bitrate (constant bitrate)
                let target_area = Rect {
                    x: inner.x,
                    y,
                    width: inner.width,
                    height: 1,
                };
                state.video_target_bitrate_area = Some(target_area);
                let target_base = if state.video_target_bitrate == 0 {
                    "0 kbps".to_string()
                } else {
                    format!("{} kbps", state.video_target_bitrate)
                };
                let target_text = if state.focus == ConfigFocus::VideoTargetBitrateInput {
                    Self::insert_cursor(&target_base, state.cursor_pos)
                } else {
                    target_base
                };
                let target_style = if state.focus == ConfigFocus::VideoTargetBitrateInput {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Green)
                };
                frame.render_widget(
                    Paragraph::new(Line::from(vec![
                        Span::raw("Constant Bitrate: "),
                        Span::raw("["),
                        Span::styled(target_text, target_style),
                        Span::raw("]"),
                    ])),
                    target_area,
                );
                y += 1;
            }
        } else {
            // Software: Show CRF slider for CQ and CQCap modes
            if matches!(
                state.rate_control_mode,
                RateControlMode::CQ | RateControlMode::CQCap
            ) {
                let crf_area = Rect {
                    x: inner.x,
                    y,
                    width: inner.width,
                    height: 2,
                };
                state.crf_slider_area = Some(crf_area);
                let crf_slider = Slider::new("CRF", 0, 63)
                    .value(state.crf)
                    .focused(state.focus == ConfigFocus::CrfSlider);
                frame.render_widget(crf_slider, crf_area);
                y += 3;
            }
        }

        // VBR mode: Target bitrate, bufsize, min/max
        if matches!(state.rate_control_mode, RateControlMode::TwoPassVBR) {
            // Target bitrate
            let target_area = Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            };
            state.video_target_bitrate_area = Some(target_area);
            let target_base = if state.video_target_bitrate == 0 {
                "0 kbps".to_string()
            } else {
                format!("{} kbps", state.video_target_bitrate)
            };
            let target_text = if state.focus == ConfigFocus::VideoTargetBitrateInput {
                Self::insert_cursor(&target_base, state.cursor_pos)
            } else {
                target_base
            };
            let target_style = if state.focus == ConfigFocus::VideoTargetBitrateInput {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            };
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw("Target Bitrate: "),
                    Span::raw("["),
                    Span::styled(target_text, target_style),
                    Span::raw("]"),
                ])),
                target_area,
            );
            y += 1;

            // Bufsize
            let bufsize_area = Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            };
            state.video_bufsize_area = Some(bufsize_area);
            let bufsize_base = if state.video_bufsize == 0 {
                "Auto".to_string()
            } else {
                format!("{} kbps", state.video_bufsize)
            };
            let bufsize_text = if state.focus == ConfigFocus::VideoBufsizeInput {
                Self::insert_cursor(&bufsize_base, state.cursor_pos)
            } else {
                bufsize_base
            };
            let bufsize_style = if state.focus == ConfigFocus::VideoBufsizeInput {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            };
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw("Buffer Size: "),
                    Span::raw("["),
                    Span::styled(bufsize_text, bufsize_style),
                    Span::raw("]"),
                ])),
                bufsize_area,
            );
            y += 1;

            // Min/Max bitrate (on one line)
            let minmax_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(Rect {
                    x: inner.x,
                    y,
                    width: inner.width,
                    height: 1,
                });

            state.video_min_bitrate_area = Some(minmax_chunks[0]);
            let min_base = if state.video_min_bitrate == 0 {
                "None".to_string()
            } else {
                format!("{} kbps", state.video_min_bitrate)
            };
            let min_text = if state.focus == ConfigFocus::VideoMinBitrateInput {
                Self::insert_cursor(&min_base, state.cursor_pos)
            } else {
                min_base
            };
            let min_style = if state.focus == ConfigFocus::VideoMinBitrateInput {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            };
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw("Min: "),
                    Span::raw("["),
                    Span::styled(min_text, min_style),
                    Span::raw("]"),
                ])),
                minmax_chunks[0],
            );

            state.video_max_bitrate_area = Some(minmax_chunks[1]);
            let max_base = if state.video_max_bitrate == 0 {
                "None".to_string()
            } else {
                format!("{} kbps", state.video_max_bitrate)
            };
            let max_text = if state.focus == ConfigFocus::VideoMaxBitrateInput {
                Self::insert_cursor(&max_base, state.cursor_pos)
            } else {
                max_base
            };
            let max_style = if state.focus == ConfigFocus::VideoMaxBitrateInput {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            };
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw("Max: "),
                    Span::raw("["),
                    Span::styled(max_text, max_style),
                    Span::raw("]"),
                ])),
                minmax_chunks[1],
            );
            y += 2;
        }

        // CBR mode: Min/Max bitrate only
        if matches!(state.rate_control_mode, RateControlMode::CBR) {
            let minmax_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(Rect {
                    x: inner.x,
                    y,
                    width: inner.width,
                    height: 1,
                });

            state.video_min_bitrate_area = Some(minmax_chunks[0]);
            let min_base = if state.video_min_bitrate == 0 {
                "None".to_string()
            } else {
                format!("{} kbps", state.video_min_bitrate)
            };
            let min_text = if state.focus == ConfigFocus::VideoMinBitrateInput {
                Self::insert_cursor(&min_base, state.cursor_pos)
            } else {
                min_base
            };
            let min_style = if state.focus == ConfigFocus::VideoMinBitrateInput {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            };
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw("Min Bitrate: "),
                    Span::raw("["),
                    Span::styled(min_text, min_style),
                    Span::raw("]"),
                ])),
                minmax_chunks[0],
            );

            state.video_max_bitrate_area = Some(minmax_chunks[1]);
            let max_base = if state.video_max_bitrate == 0 {
                "None".to_string()
            } else {
                format!("{} kbps", state.video_max_bitrate)
            };
            let max_text = if state.focus == ConfigFocus::VideoMaxBitrateInput {
                Self::insert_cursor(&max_base, state.cursor_pos)
            } else {
                max_base
            };
            let max_style = if state.focus == ConfigFocus::VideoMaxBitrateInput {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            };
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw("Max Bitrate: "),
                    Span::raw("["),
                    Span::styled(max_text, max_style),
                    Span::raw("]"),
                ])),
                minmax_chunks[1],
            );
            y += 2;
        }

        // Max bitrate for CQCap mode
        if matches!(state.rate_control_mode, RateControlMode::CQCap) {
            let max_area = Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            };
            state.video_max_bitrate_area = Some(max_area);
            let max_base = if state.video_max_bitrate == 0 {
                "None".to_string()
            } else {
                format!("{} kbps", state.video_max_bitrate)
            };
            let max_text = if state.focus == ConfigFocus::VideoMaxBitrateInput {
                Self::insert_cursor(&max_base, state.cursor_pos)
            } else {
                max_base
            };
            let max_style = if state.focus == ConfigFocus::VideoMaxBitrateInput {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            };
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw("Max Bitrate Cap: "),
                    Span::raw("["),
                    Span::styled(max_text, max_style),
                    Span::raw("]"),
                ])),
                max_area,
            );
            y += 2;
        }

        // Quality mode and speed controls (only for software encoding)
        if !state.use_hardware_encoding {
            // Quality mode dropdown
            let quality_area = Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            };
            state.quality_mode_area = Some(quality_area);
            let selected_index = state.quality_mode_state.selected().unwrap_or(0);
            let selected_value = QUALITY_MODES.get(selected_index).unwrap_or(&"good");
            let quality_style = if state.focus == ConfigFocus::QualityMode {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            };
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw("Quality: "),
                    Span::styled(*selected_value, quality_style),
                    Span::raw(" ▼"),
                ])),
                quality_area,
            );
            y += 1;

            // Speed preset - show single slider if not 2-pass, or per-pass sliders if 2-pass
            if state.two_pass {
                // Pass 1 Speed
                let pass1_area = Rect {
                    x: inner.x,
                    y,
                    width: inner.width,
                    height: 2,
                };
                state.cpu_used_pass1_slider_area = Some(pass1_area);
                let pass1_slider = Slider::new("Pass 1 Speed (cpu-used)", 0, 8)
                    .value(state.cpu_used_pass1 as u32)
                    .focused(state.focus == ConfigFocus::CpuUsedPass1Slider);
                frame.render_widget(pass1_slider, pass1_area);
                y += 3;

                // Pass 2 Speed
                let pass2_area = Rect {
                    x: inner.x,
                    y,
                    width: inner.width,
                    height: 2,
                };
                state.cpu_used_pass2_slider_area = Some(pass2_area);
                let pass2_slider = Slider::new("Pass 2 Speed (cpu-used)", 0, 8)
                    .value(state.cpu_used_pass2 as u32)
                    .focused(state.focus == ConfigFocus::CpuUsedPass2Slider);
                frame.render_widget(pass2_slider, pass2_area);
            } else {
                // Single-pass speed
                let cpu_area = Rect {
                    x: inner.x,
                    y,
                    width: inner.width,
                    height: 2,
                };
                state.cpu_used_slider_area = Some(cpu_area);
                let cpu_slider = Slider::new("Speed Preset (cpu-used)", 0, 8)
                    .value(state.cpu_used as u32)
                    .focused(state.focus == ConfigFocus::CpuUsedSlider);
                frame.render_widget(cpu_slider, cpu_area);
            }
        }
    }

    pub(super) fn render_parallelism(frame: &mut Frame, area: Rect, state: &mut ConfigState) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Parallelism")
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Show message when hardware encoding is enabled (parallelism settings don't apply)
        if state.use_hardware_encoding {
            let msg_area = Rect {
                x: inner.x,
                y: inner.y,
                width: inner.width,
                height: 3,
            };
            frame.render_widget(
                Paragraph::new("Parallelism settings\n(tile-based, threading)\nonly apply to software\nencoding")
                    .style(Style::default().fg(Color::Yellow)),
                msg_area,
            );
            return;
        }

        // Row MT checkbox
        let row_mt_area = Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width / 2,
            height: 1,
        };
        state.row_mt_checkbox_area = Some(row_mt_area);
        render_checkbox(
            "Row MT",
            state.row_mt,
            state.focus == ConfigFocus::RowMtCheckbox,
            row_mt_area,
            frame.buffer_mut(),
        );

        // Frame parallel checkbox
        let frame_parallel_area = Rect {
            x: inner.x + inner.width / 2,
            y: inner.y,
            width: inner.width / 2,
            height: 1,
        };
        state.frame_parallel_checkbox_area = Some(frame_parallel_area);
        render_checkbox(
            "Frame Parallel",
            state.frame_parallel,
            state.focus == ConfigFocus::FrameParallelCheckbox,
            frame_parallel_area,
            frame.buffer_mut(),
        );

        // Tile columns slider
        let tile_cols_area = Rect {
            x: inner.x,
            y: inner.y + 1,
            width: inner.width,
            height: 2,
        };
        state.tile_columns_slider_area = Some(tile_cols_area);
        let tile_cols_slider = Slider::new("Tile Columns (log2)", 0, 6)
            .value(state.tile_columns as u32)
            .focused(state.focus == ConfigFocus::TileColumnsSlider);
        frame.render_widget(tile_cols_slider, tile_cols_area);

        // Tile rows slider
        let tile_rows_area = Rect {
            x: inner.x,
            y: inner.y + 3,
            width: inner.width / 2,
            height: 1,
        };
        state.tile_rows_slider_area = Some(tile_rows_area);
        let tile_rows_line = Line::from(vec![
            Span::raw("Tile Rows: "),
            Span::styled(
                format!("{}", state.tile_rows),
                if state.focus == ConfigFocus::TileRowsSlider {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Green)
                },
            ),
        ]);
        frame.render_widget(Paragraph::new(tile_rows_line), tile_rows_area);

        // Threads
        let threads_area = Rect {
            x: inner.x + inner.width / 2,
            y: inner.y + 3,
            width: inner.width / 2,
            height: 1,
        };
        state.threads_area = Some(threads_area);
        let threads_base = if state.threads == 0 {
            "Auto".to_string()
        } else {
            state.threads.to_string()
        };
        let threads_text = if state.focus == ConfigFocus::ThreadsInput {
            Self::insert_cursor(&threads_base, state.cursor_pos)
        } else {
            threads_base
        };
        let threads_line = Line::from(vec![
            Span::raw("Threads: "),
            Span::raw("["),
            Span::styled(
                threads_text,
                if state.focus == ConfigFocus::ThreadsInput {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Green)
                },
            ),
            Span::raw("]"),
        ]);
        frame.render_widget(Paragraph::new(threads_line), threads_area);

        // Max Workers
        let max_workers_area = Rect {
            x: inner.x,
            y: inner.y + 4,
            width: inner.width / 2,
            height: 1,
        };
        state.max_workers_area = Some(max_workers_area);
        let max_workers_base = format!("{}", state.max_workers);
        let max_workers_text = if state.focus == ConfigFocus::MaxWorkersInput {
            Self::insert_cursor(&max_workers_base, state.cursor_pos)
        } else {
            max_workers_base
        };
        let max_workers_line = Line::from(vec![
            Span::raw("Max Workers: "),
            Span::raw("["),
            Span::styled(
                max_workers_text,
                if state.focus == ConfigFocus::MaxWorkersInput {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Green)
                },
            ),
            Span::raw("]"),
        ]);
        frame.render_widget(Paragraph::new(max_workers_line), max_workers_area);
    }

    pub(super) fn render_gop_keyframes(frame: &mut Frame, area: Rect, state: &mut ConfigState) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("GOP & Keyframes")
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // GOP length
        let gop_area = Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width / 2,
            height: 1,
        };
        state.gop_length_area = Some(gop_area);
        let gop_base = format!("{} frames", state.gop_length);
        let gop_text = if state.focus == ConfigFocus::GopLengthInput {
            Self::insert_cursor(&gop_base, state.cursor_pos)
        } else {
            gop_base
        };
        let gop_line = Line::from(vec![
            Span::raw("GOP Length: "),
            Span::raw("["),
            Span::styled(
                gop_text,
                if state.focus == ConfigFocus::GopLengthInput {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Green)
                },
            ),
            Span::raw("]"),
        ]);
        frame.render_widget(Paragraph::new(gop_line), gop_area);

        // Fixed GOP checkbox
        let fixed_gop_area = Rect {
            x: inner.x + inner.width / 2,
            y: inner.y,
            width: inner.width / 2,
            height: 1,
        };
        state.fixed_gop_checkbox_area = Some(fixed_gop_area);
        render_checkbox(
            "Fixed GOP",
            state.fixed_gop,
            state.focus == ConfigFocus::FixedGopCheckbox,
            fixed_gop_area,
            frame.buffer_mut(),
        );

        // Keyint Min input
        let keyint_min_area = Rect {
            x: inner.x,
            y: inner.y + 1,
            width: inner.width,
            height: 1,
        };
        state.keyint_min_area = Some(keyint_min_area);
        let keyint_min_base = if state.keyint_min == 0 {
            "Auto".to_string()
        } else {
            format!("{} frames", state.keyint_min)
        };
        let keyint_min_text = if state.focus == ConfigFocus::KeyintMinInput {
            Self::insert_cursor(&keyint_min_base, state.cursor_pos)
        } else {
            keyint_min_base
        };
        let keyint_min_line = Line::from(vec![
            Span::raw("Min Keyframe Interval: "),
            Span::raw("["),
            Span::styled(
                keyint_min_text,
                if state.focus == ConfigFocus::KeyintMinInput {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Green)
                },
            ),
            Span::raw("]"),
        ]);
        frame.render_widget(Paragraph::new(keyint_min_line), keyint_min_area);

        // Lag in frames slider
        let lag_area = Rect {
            x: inner.x,
            y: inner.y + 2,
            width: inner.width,
            height: 2,
        };
        state.lag_in_frames_slider_area = Some(lag_area);
        let lag_slider = Slider::new("Lag-in-frames", 0, 25)
            .value(state.lag_in_frames)
            .focused(state.focus == ConfigFocus::LagInFramesSlider);
        frame.render_widget(lag_slider, lag_area);

        // Auto alt-ref checkbox
        let alt_ref_area = Rect {
            x: inner.x,
            y: inner.y + 4,
            width: inner.width,
            height: 1,
        };
        state.auto_alt_ref_checkbox_area = Some(alt_ref_area);
        render_checkbox(
            "Auto Alt-Ref",
            state.auto_alt_ref,
            state.focus == ConfigFocus::AutoAltRefCheckbox,
            alt_ref_area,
            frame.buffer_mut(),
        );
    }

    #[allow(dead_code)]
    pub(super) fn render_aq_denoising(frame: &mut Frame, area: Rect, state: &mut ConfigState) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("AQ & Denoising")
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // AQ mode dropdown
        let aq_area = Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: 1,
        };
        state.aq_mode_area = Some(aq_area);
        let selected_index = state.aq_mode_state.selected().unwrap_or(1);
        let selected_value = AQ_MODES.get(selected_index).unwrap_or(&"Variance");
        let aq_style = if state.focus == ConfigFocus::AqModeDropdown {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        let aq_line = Line::from(vec![
            Span::raw("AQ Mode: "),
            Span::styled(*selected_value, aq_style),
            Span::raw(" ▼"),
        ]);
        frame.render_widget(Paragraph::new(aq_line), aq_area);

        // ARNR sliders (compact, single line each)
        let arnr_max_area = Rect {
            x: inner.x,
            y: inner.y + 1,
            width: inner.width / 2,
            height: 1,
        };
        state.arnr_max_frames_slider_area = Some(arnr_max_area);
        let arnr_max_line = Line::from(vec![
            Span::raw("ARNR Frames: "),
            Span::styled(
                format!("{}", state.arnr_max_frames),
                if state.focus == ConfigFocus::ArnrMaxFramesSlider {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Green)
                },
            ),
        ]);
        frame.render_widget(Paragraph::new(arnr_max_line), arnr_max_area);

        let arnr_strength_area = Rect {
            x: inner.x + inner.width / 2,
            y: inner.y + 1,
            width: inner.width / 2,
            height: 1,
        };
        state.arnr_strength_slider_area = Some(arnr_strength_area);
        let arnr_strength_line = Line::from(vec![
            Span::raw("ARNR Strength: "),
            Span::styled(
                format!("{}", state.arnr_strength),
                if state.focus == ConfigFocus::ArnrStrengthSlider {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Green)
                },
            ),
        ]);
        frame.render_widget(Paragraph::new(arnr_strength_line), arnr_strength_area);
    }

    #[allow(dead_code)]
    pub(super) fn render_advanced_tuning(frame: &mut Frame, area: Rect, state: &mut ConfigState) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Advanced Tuning")
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Tune content dropdown
        let tune_area = Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width / 2,
            height: 1,
        };
        state.tune_content_area = Some(tune_area);
        let selected_index = state.tune_content_state.selected().unwrap_or(0);
        let selected_value = TUNE_CONTENTS.get(selected_index).unwrap_or(&"default");
        let tune_style = if state.focus == ConfigFocus::TuneContentDropdown {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        let tune_line = Line::from(vec![
            Span::raw("Tune: "),
            Span::styled(*selected_value, tune_style),
            Span::raw(" ▼"),
        ]);
        frame.render_widget(Paragraph::new(tune_line), tune_area);

        // Enable TPL checkbox
        let tpl_area = Rect {
            x: inner.x + inner.width / 2,
            y: inner.y,
            width: inner.width / 2,
            height: 1,
        };
        state.enable_tpl_checkbox_area = Some(tpl_area);
        render_checkbox(
            "Enable TPL",
            state.enable_tpl,
            state.focus == ConfigFocus::EnableTplCheckbox,
            tpl_area,
            frame.buffer_mut(),
        );

        // Sharpness
        let sharpness_area = Rect {
            x: inner.x,
            y: inner.y + 1,
            width: inner.width / 2,
            height: 1,
        };
        state.sharpness_slider_area = Some(sharpness_area);
        let sharpness_text = if state.sharpness == -1 {
            "Auto".to_string()
        } else {
            state.sharpness.to_string()
        };
        let sharpness_line = Line::from(vec![
            Span::raw("Sharpness: "),
            Span::styled(
                sharpness_text,
                if state.focus == ConfigFocus::SharpnessSlider {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Green)
                },
            ),
        ]);
        frame.render_widget(Paragraph::new(sharpness_line), sharpness_area);

        // Noise sensitivity
        let noise_area = Rect {
            x: inner.x + inner.width / 2,
            y: inner.y + 1,
            width: inner.width / 2,
            height: 1,
        };
        state.noise_sensitivity_slider_area = Some(noise_area);
        let noise_line = Line::from(vec![
            Span::raw("Noise Sens: "),
            Span::styled(
                format!("{}", state.noise_sensitivity),
                if state.focus == ConfigFocus::NoiseSensitivitySlider {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Green)
                },
            ),
        ]);
        frame.render_widget(Paragraph::new(noise_line), noise_area);
    }

    #[allow(dead_code)]
    pub(super) fn render_audio_settings(frame: &mut Frame, area: Rect, state: &mut ConfigState) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Audio Settings")
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Codec dropdown - show selected value only
        let codec_area = Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width / 2,
            height: 1,
        };
        state.codec_list_area = Some(codec_area);

        let selected_index = state.codec_list_state.selected().unwrap_or(0);
        let selected_value = AUDIO_CODECS.get(selected_index).unwrap_or(&"None");

        let codec_style = if state.focus == ConfigFocus::AudioCodec {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };

        let codec_line = Line::from(vec![
            Span::raw("Codec: "),
            Span::styled(*selected_value, codec_style),
            Span::raw(" ▼"),
        ]);

        frame.render_widget(Paragraph::new(codec_line), codec_area);

        // Audio Bitrate Slider
        let bitrate_area = Rect {
            x: inner.x,
            y: inner.y + 1,
            width: inner.width,
            height: 2,
        };
        state.audio_bitrate_slider_area = Some(bitrate_area);

        let bitrate_slider = Slider::new("Bitrate (kbps)", 32, 512)
            .value(state.audio_bitrate)
            .focused(state.focus == ConfigFocus::AudioBitrateSlider);

        frame.render_widget(bitrate_slider, bitrate_area);
    }

    pub(super) fn render_tuning_filters(frame: &mut Frame, area: Rect, state: &mut ConfigState) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Tuning & Filters")
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut y = inner.y;

        // AQ mode dropdown
        let aq_area = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        };
        state.aq_mode_area = Some(aq_area);
        let selected_index = state.aq_mode_state.selected().unwrap_or(1);
        let selected_value = AQ_MODES.get(selected_index).unwrap_or(&"Off");
        let aq_style = if state.focus == ConfigFocus::AqModeDropdown {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("AQ Mode: "),
                Span::styled(*selected_value, aq_style),
                Span::raw(" ▼"),
            ])),
            aq_area,
        );
        y += 1;

        // ARNR Frames and Strength on one line
        let arnr_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            });

        state.arnr_max_frames_slider_area = Some(arnr_chunks[0]);
        let arnr_frames_style = if state.focus == ConfigFocus::ArnrMaxFramesSlider {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("ARNR Frames: "),
                Span::styled(format!("{}", state.arnr_max_frames), arnr_frames_style),
            ])),
            arnr_chunks[0],
        );

        state.arnr_strength_slider_area = Some(arnr_chunks[1]);
        let arnr_str_style = if state.focus == ConfigFocus::ArnrStrengthSlider {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("ARNR Strength: "),
                Span::styled(format!("{}", state.arnr_strength), arnr_str_style),
            ])),
            arnr_chunks[1],
        );
        y += 2;

        // Tune content dropdown and Sharpness on one line
        let tune_sharp_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            });

        state.tune_content_area = Some(tune_sharp_chunks[0]);
        let selected_index = state.tune_content_state.selected().unwrap_or(0);
        let selected_value = TUNE_CONTENTS.get(selected_index).unwrap_or(&"default");
        let tune_style = if state.focus == ConfigFocus::TuneContentDropdown {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("Tune: "),
                Span::styled(*selected_value, tune_style),
                Span::raw(" ▼"),
            ])),
            tune_sharp_chunks[0],
        );

        state.sharpness_slider_area = Some(tune_sharp_chunks[1]);
        let sharpness_text = if state.sharpness == -1 {
            "Auto".to_string()
        } else {
            state.sharpness.to_string()
        };
        let sharp_style = if state.focus == ConfigFocus::SharpnessSlider {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("Sharpness: "),
                Span::styled(sharpness_text, sharp_style),
            ])),
            tune_sharp_chunks[1],
        );
        y += 1;

        // Enable TPL and Noise Sens on one line
        let tpl_noise_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            });

        state.enable_tpl_checkbox_area = Some(tpl_noise_chunks[0]);
        render_checkbox(
            "Enable TPL",
            state.enable_tpl,
            state.focus == ConfigFocus::EnableTplCheckbox,
            tpl_noise_chunks[0],
            frame.buffer_mut(),
        );

        state.noise_sensitivity_slider_area = Some(tpl_noise_chunks[1]);
        let noise_style = if state.focus == ConfigFocus::NoiseSensitivitySlider {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("Noise Sens: "),
                Span::styled(format!("{}", state.noise_sensitivity), noise_style),
            ])),
            tpl_noise_chunks[1],
        );
        y += 1;

        // ARNR Type dropdown (full width)
        let arnr_type_area = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        };
        state.arnr_type_area = Some(arnr_type_area);
        let selected_index = state.arnr_type_state.selected().unwrap_or(0);
        let selected_value = ARNR_TYPES.get(selected_index).unwrap_or(&"Auto");
        let arnr_type_style = if state.focus == ConfigFocus::ArnrTypeDropdown {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("ARNR Type: "),
                Span::styled(*selected_value, arnr_type_style),
                Span::raw(" ▼"),
            ])),
            arnr_type_area,
        );
        y += 1;

        // Static Thresh and Max Intra Rate on one line
        let adv_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            });

        state.static_thresh_area = Some(adv_chunks[0]);
        let static_base = if state.static_thresh == 0 {
            "Off".to_string()
        } else {
            state.static_thresh.to_string()
        };
        let static_text = if state.focus == ConfigFocus::StaticThreshInput {
            Self::insert_cursor(&static_base, state.cursor_pos)
        } else {
            static_base
        };
        let static_style = if state.focus == ConfigFocus::StaticThreshInput {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("Static Thresh: "),
                Span::raw("["),
                Span::styled(static_text, static_style),
                Span::raw("]"),
            ])),
            adv_chunks[0],
        );

        state.max_intra_rate_area = Some(adv_chunks[1]);
        let max_intra_base = if state.max_intra_rate == 0 {
            "Off".to_string()
        } else {
            format!("{}%", state.max_intra_rate)
        };
        let max_intra_text = if state.focus == ConfigFocus::MaxIntraRateInput {
            Self::insert_cursor(&max_intra_base, state.cursor_pos)
        } else {
            max_intra_base
        };
        let max_intra_style = if state.focus == ConfigFocus::MaxIntraRateInput {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("Max I-Rate: "),
                Span::raw("["),
                Span::styled(max_intra_text, max_intra_style),
                Span::raw("]"),
            ])),
            adv_chunks[1],
        );
        y += 1;

        // Colorspace and Color Primaries on one line
        let color1_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            });

        state.colorspace_area = Some(color1_chunks[0]);
        let selected_index = state.colorspace_state.selected().unwrap_or(0);
        let selected_value = COLORSPACES.get(selected_index).unwrap_or(&"Auto");
        let colorspace_style = if state.focus == ConfigFocus::ColorspaceDropdown {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("Colorspace: "),
                Span::styled(*selected_value, colorspace_style),
                Span::raw(" ▼"),
            ])),
            color1_chunks[0],
        );

        state.color_primaries_area = Some(color1_chunks[1]);
        let selected_index = state.color_primaries_state.selected().unwrap_or(0);
        let selected_value = COLOR_PRIMARIES.get(selected_index).unwrap_or(&"Auto");
        let primaries_style = if state.focus == ConfigFocus::ColorPrimariesDropdown {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("Primaries: "),
                Span::styled(*selected_value, primaries_style),
                Span::raw(" ▼"),
            ])),
            color1_chunks[1],
        );
        y += 1;

        // Color TRC and Color Range on one line
        let color2_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            });

        state.color_trc_area = Some(color2_chunks[0]);
        let selected_index = state.color_trc_state.selected().unwrap_or(0);
        let selected_value = COLOR_TRCS.get(selected_index).unwrap_or(&"Auto");
        let trc_style = if state.focus == ConfigFocus::ColorTrcDropdown {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("TRC: "),
                Span::styled(*selected_value, trc_style),
                Span::raw(" ▼"),
            ])),
            color2_chunks[0],
        );

        state.color_range_area = Some(color2_chunks[1]);
        let selected_index = state.color_range_state.selected().unwrap_or(0);
        let selected_value = COLOR_RANGES.get(selected_index).unwrap_or(&"Auto");
        let range_style = if state.focus == ConfigFocus::ColorRangeDropdown {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("Range: "),
                Span::styled(*selected_value, range_style),
                Span::raw(" ▼"),
            ])),
            color2_chunks[1],
        );
        y += 1;

        // VAAPI Hardware Encoding Parameters (Intel Quick Sync)
        if state.use_hardware_encoding {
            y += 1; // Spacer

            // B-frames textbox (full width)
            let bf_area = Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            };
            state.vaapi_b_frames_area = Some(bf_area);
            let bf_base = &state.vaapi_b_frames;
            let bf_text = if state.focus == ConfigFocus::VaapiBFramesInput {
                Self::insert_cursor(bf_base, state.cursor_pos)
            } else {
                bf_base.to_string()
            };
            let bf_style = if state.focus == ConfigFocus::VaapiBFramesInput {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            };
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw("VAAPI B-frames (0-4): "),
                    Span::raw("["),
                    Span::styled(bf_text, bf_style),
                    Span::raw("]"),
                ])),
                bf_area,
            );
            y += 1;

            // Loop filter level and sharpness on one line
            let loop_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(Rect {
                    x: inner.x,
                    y,
                    width: inner.width,
                    height: 1,
                });

            state.vaapi_loop_filter_level_area = Some(loop_chunks[0]);
            let lfl_base = &state.vaapi_loop_filter_level;
            let lfl_text = if state.focus == ConfigFocus::VaapiLoopFilterLevelInput {
                Self::insert_cursor(lfl_base, state.cursor_pos)
            } else {
                lfl_base.to_string()
            };
            let lfl_style = if state.focus == ConfigFocus::VaapiLoopFilterLevelInput {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            };
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw("LF Level (0-63): "),
                    Span::raw("["),
                    Span::styled(lfl_text, lfl_style),
                    Span::raw("]"),
                ])),
                loop_chunks[0],
            );

            state.vaapi_loop_filter_sharpness_area = Some(loop_chunks[1]);
            let lfs_base = &state.vaapi_loop_filter_sharpness;
            let lfs_text = if state.focus == ConfigFocus::VaapiLoopFilterSharpnessInput {
                Self::insert_cursor(lfs_base, state.cursor_pos)
            } else {
                lfs_base.to_string()
            };
            let lfs_style = if state.focus == ConfigFocus::VaapiLoopFilterSharpnessInput {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            };
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw("LF Sharp (0-15): "),
                    Span::raw("["),
                    Span::styled(lfs_text, lfs_style),
                    Span::raw("]"),
                ])),
                loop_chunks[1],
            );
        }
    }

    pub(super) fn render_popup_dropdown(frame: &mut Frame, state: &mut ConfigState) {
        let dropdown_type = match state.active_dropdown {
            Some(ref d) => d,
            None => return,
        };

        // Determine which dropdown to show and render it
        match dropdown_type {
            ConfigFocus::ProfileList => {
                // Build profile list: built-in + saved + Custom (if modified) + Create New...
                use crate::engine::Profile;
                let mut items = Vec::new();

                // Add built-in profiles
                items.extend(Profile::builtin_names());

                // Add saved profiles (excluding built-ins to avoid duplicates)
                for saved_profile in &state.available_profiles {
                    if !Profile::builtin_names().contains(saved_profile) {
                        items.push(saved_profile.clone());
                    }
                }

                // Add "Custom" if modified
                if state.is_modified {
                    items.push("Custom".to_string());
                }

                // Always add "Create New..."
                items.push("Create New...".to_string());

                let area = state.profile_list_area.unwrap_or(Rect::default());
                let selected = state.profile_list_state.selected().unwrap_or(0);
                if selected >= items.len() {
                    state.profile_list_state.select(Some(0));
                }
                let items_str: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
                Self::render_popup_list(frame, &items_str, area, &mut state.profile_list_state);
            }
            ConfigFocus::QualityMode => {
                let area = state.quality_mode_area.unwrap_or(Rect::default());
                let selected = state.quality_mode_state.selected().unwrap_or(0);
                if selected >= QUALITY_MODES.len() {
                    state.quality_mode_state.select(Some(0));
                }
                Self::render_popup_list(frame, QUALITY_MODES, area, &mut state.quality_mode_state);
            }
            ConfigFocus::ProfileDropdown => {
                let area = state.vp9_profile_list_area.unwrap_or(Rect::default());
                let selected = state.profile_dropdown_state.selected().unwrap_or(0);
                if selected >= VP9_PROFILES.len() {
                    state.profile_dropdown_state.select(Some(0));
                }
                Self::render_popup_list(
                    frame,
                    VP9_PROFILES,
                    area,
                    &mut state.profile_dropdown_state,
                );
            }
            ConfigFocus::PixFmtDropdown => {
                let area = state.pix_fmt_area.unwrap_or(Rect::default());
                let selected = state.pix_fmt_state.selected().unwrap_or(0);
                if selected >= PIX_FMTS_DISPLAY.len() {
                    state.pix_fmt_state.select(Some(0));
                }
                Self::render_popup_list(frame, PIX_FMTS_DISPLAY, area, &mut state.pix_fmt_state);
            }
            ConfigFocus::AqModeDropdown => {
                let area = state.aq_mode_area.unwrap_or(Rect::default());
                let selected = state.aq_mode_state.selected().unwrap_or(0);
                if selected >= AQ_MODES.len() {
                    state.aq_mode_state.select(Some(0));
                }
                Self::render_popup_list(frame, AQ_MODES, area, &mut state.aq_mode_state);
            }
            ConfigFocus::TuneContentDropdown => {
                let area = state.tune_content_area.unwrap_or(Rect::default());
                let selected = state.tune_content_state.selected().unwrap_or(0);
                if selected >= TUNE_CONTENTS.len() {
                    state.tune_content_state.select(Some(0));
                }
                Self::render_popup_list(frame, TUNE_CONTENTS, area, &mut state.tune_content_state);
            }
            ConfigFocus::AudioCodec => {
                let area = state.codec_list_area.unwrap_or(Rect::default());
                let selected = state.codec_list_state.selected().unwrap_or(0);
                if selected >= AUDIO_CODECS.len() {
                    state.codec_list_state.select(Some(0));
                }
                Self::render_popup_list(frame, AUDIO_CODECS, area, &mut state.codec_list_state);
            }
            ConfigFocus::ArnrTypeDropdown => {
                let area = state.arnr_type_area.unwrap_or(Rect::default());
                let selected = state.arnr_type_state.selected().unwrap_or(0);
                if selected >= ARNR_TYPES.len() {
                    state.arnr_type_state.select(Some(0));
                }
                Self::render_popup_list(frame, ARNR_TYPES, area, &mut state.arnr_type_state);
            }
            ConfigFocus::ColorspaceDropdown => {
                let area = state.colorspace_area.unwrap_or(Rect::default());
                let selected = state.colorspace_state.selected().unwrap_or(0);
                if selected >= COLORSPACES.len() {
                    state.colorspace_state.select(Some(0));
                }
                Self::render_popup_list(frame, COLORSPACES, area, &mut state.colorspace_state);
            }
            ConfigFocus::ColorPrimariesDropdown => {
                let area = state.color_primaries_area.unwrap_or(Rect::default());
                let selected = state.color_primaries_state.selected().unwrap_or(0);
                if selected >= COLOR_PRIMARIES.len() {
                    state.color_primaries_state.select(Some(0));
                }
                Self::render_popup_list(
                    frame,
                    COLOR_PRIMARIES,
                    area,
                    &mut state.color_primaries_state,
                );
            }
            ConfigFocus::ColorTrcDropdown => {
                let area = state.color_trc_area.unwrap_or(Rect::default());
                let selected = state.color_trc_state.selected().unwrap_or(0);
                if selected >= COLOR_TRCS.len() {
                    state.color_trc_state.select(Some(0));
                }
                Self::render_popup_list(frame, COLOR_TRCS, area, &mut state.color_trc_state);
            }
            ConfigFocus::ColorRangeDropdown => {
                let area = state.color_range_area.unwrap_or(Rect::default());
                let selected = state.color_range_state.selected().unwrap_or(0);
                if selected >= COLOR_RANGES.len() {
                    state.color_range_state.select(Some(0));
                }
                Self::render_popup_list(frame, COLOR_RANGES, area, &mut state.color_range_state);
            }
            ConfigFocus::FpsDropdown => {
                let area = state.fps_area.unwrap_or(Rect::default());
                let selected = state.fps_dropdown_state.selected().unwrap_or(0);
                if selected >= FPS_OPTIONS_DISPLAY.len() {
                    state.fps_dropdown_state.select(Some(0));
                }
                Self::render_popup_list(
                    frame,
                    FPS_OPTIONS_DISPLAY,
                    area,
                    &mut state.fps_dropdown_state,
                );
            }
            ConfigFocus::ResolutionDropdown => {
                let area = state.scale_width_area.unwrap_or(Rect::default());
                let selected = state.resolution_dropdown_state.selected().unwrap_or(0);
                if selected >= RESOLUTION_OPTIONS_DISPLAY.len() {
                    state.resolution_dropdown_state.select(Some(0));
                }
                Self::render_popup_list(
                    frame,
                    RESOLUTION_OPTIONS_DISPLAY,
                    area,
                    &mut state.resolution_dropdown_state,
                );
            }
            ConfigFocus::ContainerDropdown => {
                let area = state.container_dropdown_area.unwrap_or(Rect::default());
                let selected = state.container_dropdown_state.selected().unwrap_or(0);
                if selected >= constants::CONTAINER_FORMATS.len() {
                    state.container_dropdown_state.select(Some(0));
                }
                Self::render_popup_list(
                    frame,
                    constants::CONTAINER_FORMATS,
                    area,
                    &mut state.container_dropdown_state,
                );
            }
            _ => {} // Not a dropdown focus type
        }
    }

    pub fn calculate_popup_area(trigger_area: Rect, item_count: usize, viewport: Rect) -> Rect {
        // Calculate desired popup height
        let desired_height = (item_count as u16).min(10) + 2; // +2 for borders

        // Check available space below trigger
        let space_below = viewport.height.saturating_sub(trigger_area.y + 1);

        // Decide popup position and height
        let (popup_y, popup_height) = if space_below >= desired_height {
            // Enough space below - render below trigger
            (trigger_area.y.saturating_add(1), desired_height)
        } else if trigger_area.y >= desired_height {
            // Not enough space below but enough above - render above trigger
            (
                trigger_area.y.saturating_sub(desired_height),
                desired_height,
            )
        } else {
            // Not enough space either way - use whatever space is available below and clip
            (trigger_area.y.saturating_add(1), space_below.max(3)) // At least 3 lines (border + 1 item)
        };

        // Ensure popup fits within viewport bounds
        Rect {
            x: trigger_area.x,
            y: popup_y,
            width: trigger_area.width.max(30),
            height: popup_height.min(viewport.height.saturating_sub(popup_y)),
        }
    }

    pub(super) fn render_popup_list(
        frame: &mut Frame,
        items: &[&str],
        trigger_area: Rect,
        list_state: &mut ListState,
    ) {
        let popup_area = Self::calculate_popup_area(trigger_area, items.len(), frame.area());

        // Clear the area for popup
        frame.render_widget(Clear, popup_area);

        // Create the list items
        let list_items: Vec<ListItem> = items.iter().map(|item| ListItem::new(*item)).collect();

        // Render the popup list
        let popup_list = List::new(list_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .style(Style::default().bg(Color::Black)),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(popup_list, popup_area, list_state);
    }

    pub(super) fn render_name_input_dialog(frame: &mut Frame, state: &mut ConfigState) {
        let name = state.name_input_dialog.as_ref().unwrap();

        // Create centered popup area
        let area = frame.area();
        let popup_width = 50;
        let popup_height = 7;
        let popup_area = Rect {
            x: (area.width.saturating_sub(popup_width)) / 2,
            y: (area.height.saturating_sub(popup_height)) / 2,
            width: popup_width,
            height: popup_height,
        };

        // Clear the area
        frame.render_widget(Clear, popup_area);

        // Render the dialog box
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .style(Style::default().bg(Color::Black))
            .title("Save Profile");

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        // Split inner area into lines
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Prompt
                Constraint::Length(1), // Spacer
                Constraint::Length(1), // Input box
                Constraint::Length(1), // Spacer
                Constraint::Length(1), // Instructions
            ])
            .split(inner);

        // Render prompt
        let prompt = Paragraph::new("Enter profile name:").style(Style::default().fg(Color::White));
        frame.render_widget(prompt, chunks[0]);

        // Render input box with cursor
        let input_text = if name.is_empty() {
            Span::styled("_", Style::default().fg(Color::Gray))
        } else {
            Span::styled(format!("{}_", name), Style::default().fg(Color::Cyan))
        };
        let input = Paragraph::new(Line::from(vec![input_text]))
            .style(Style::default().bg(Color::DarkGray));
        frame.render_widget(input, chunks[2]);

        // Render instructions
        let instructions = Paragraph::new("Enter: Save | Esc: Cancel")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        frame.render_widget(instructions, chunks[4]);
    }

    pub(super) fn render_status_message(frame: &mut Frame, state: &mut ConfigState) {
        let (message, _) = state.status_message.as_ref().unwrap();

        // Create popup at top-center of screen
        let area = frame.area();
        let popup_width = (message.len() as u16 + 4).min(area.width.saturating_sub(4));
        let popup_height = 3;
        let popup_area = Rect {
            x: (area.width.saturating_sub(popup_width)) / 2,
            y: 2, // Near top of screen
            width: popup_width,
            height: popup_height,
        };

        // Clear the area
        frame.render_widget(Clear, popup_area);

        // Render the status message box
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green))
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        // Render message text
        let text = Paragraph::new(message.as_str())
            .style(
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center);
        frame.render_widget(text, inner);
    }
}
