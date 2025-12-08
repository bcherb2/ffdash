// Help modal implementation

use super::navigation::{HelpModalState, HelpSection};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

pub struct HelpModal;

impl HelpModal {
    pub fn render(frame: &mut Frame, state: &mut HelpModalState) {
        let area = frame.area();

        // Calculate modal size (80% width, 90% height)
        let modal_width = (area.width * 80) / 100;
        let modal_height = (area.height * 90) / 100;

        // Ensure minimum size
        let modal_width = modal_width.max(60);
        let modal_height = modal_height.max(20);

        let modal_area = Rect {
            x: (area.width.saturating_sub(modal_width)) / 2,
            y: (area.height.saturating_sub(modal_height)) / 2,
            width: modal_width,
            height: modal_height,
        };

        // Clear background
        frame.render_widget(Clear, modal_area);

        // Render bordered box
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(format!("Help - {}", state.current_section.title()))
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(modal_area);
        frame.render_widget(block, modal_area);

        // Layout: tabs + content + footer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Section tabs
                Constraint::Min(10),   // Content area
                Constraint::Length(1), // Footer/navigation hints
            ])
            .split(inner);

        // Render section tabs
        Self::render_tabs(frame, chunks[0], state.current_section);

        // Render content for current section
        let content = Self::get_section_content(state);
        let content_height = content.len() as u16;
        let viewport_height = chunks[1].height;

        // Calculate max scroll
        state.max_scroll = content_height.saturating_sub(viewport_height);
        state.scroll_offset = state.scroll_offset.min(state.max_scroll);

        // Render scrollable content
        let visible_content: Vec<Line> = content
            .into_iter()
            .skip(state.scroll_offset as usize)
            .take(viewport_height as usize)
            .collect();

        let paragraph = Paragraph::new(visible_content)
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, chunks[1]);

        // Render footer with navigation hints
        Self::render_footer(frame, chunks[2], state);
    }

    fn render_tabs(frame: &mut Frame, area: Rect, current: HelpSection) {
        let sections = HelpSection::all_sections();
        let mut spans = Vec::new();

        for (i, section) in sections.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw(" "));
            }

            let style = if *section == current {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            spans.push(Span::styled(section.title(), style));
        }

        let tabs = Paragraph::new(Line::from(spans)).alignment(Alignment::Left);

        frame.render_widget(tabs, area);
    }

    fn render_footer(frame: &mut Frame, area: Rect, state: &HelpModalState) {
        let mut hints = vec![
            Span::styled("[Tab/Arrows]", Style::default().fg(Color::Yellow)),
            Span::raw(" Switch  "),
            Span::styled("[↑↓/jk]", Style::default().fg(Color::Yellow)),
            Span::raw(" Scroll  "),
            Span::styled("[Esc/H]", Style::default().fg(Color::Yellow)),
            Span::raw(" Close"),
        ];

        // Add scroll indicators
        if state.scroll_offset > 0 {
            hints.insert(0, Span::styled("↑ ", Style::default().fg(Color::Cyan)));
        }
        if state.scroll_offset < state.max_scroll {
            hints.push(Span::styled(" ↓", Style::default().fg(Color::Cyan)));
        }

        let footer = Paragraph::new(Line::from(hints))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Gray));

        frame.render_widget(footer, area);
    }

    fn get_section_content(state: &HelpModalState) -> Vec<Line<'static>> {
        match state.current_section {
            HelpSection::About => Self::about_content(state),
            HelpSection::GeneralSettings => Self::general_settings_content(),
            HelpSection::HardwareEncoding => Self::hardware_encoding_content(),
            HelpSection::RateControl => Self::rate_control_content(),
            HelpSection::Parallelism => Self::parallelism_content(),
            HelpSection::GopKeyframes => Self::gop_keyframes_content(),
            HelpSection::AdvancedTuning => Self::advanced_tuning_content(),
            HelpSection::AudioSettings => Self::audio_settings_content(),
            HelpSection::KeyboardShortcuts => Self::keyboard_shortcuts_content(),
        }
    }

    fn about_content(state: &HelpModalState) -> Vec<Line<'static>> {
        let ffmpeg_ver = state.ffmpeg_version.as_deref().unwrap_or("Checking...");
        let ffprobe_ver = state.ffprobe_version.as_deref().unwrap_or("Checking...");

        let app_version = state.app_version.clone();
        let ffmpeg_version = ffmpeg_ver.to_string();
        let ffprobe_version = ffprobe_ver.to_string();
        let vmaf_line = if state.vmaf_available {
            "VMAF:    ✓ libvmaf filter available"
        } else {
            "VMAF:    ✗ libvmaf filter missing (install ffmpeg with libvmaf)"
        };

        let mut lines = vec![
            Line::from(vec![Span::styled(
                "ffdash - VP9 Encoder Dashboard",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(format!("Version: {}", app_version)),
            Line::from(format!("FFmpeg:  {}", ffmpeg_version)),
            Line::from(format!("FFprobe: {}", ffprobe_version)),
            Line::from(vmaf_line),
        ];

        // Hardware Encoding Status section
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Hardware Encoding Status:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]));

        if let Some(hw_result) = &state.hw_preflight_result {
            // Overall status
            let status_line = if hw_result.available {
                format!("  Overall:        Available")
            } else {
                format!("  Overall:        Not Available")
            };
            lines.push(Line::from(status_line));

            // Platform check
            let platform_line = if hw_result.platform_ok {
                format!("  Platform:       ✓ Linux")
            } else {
                format!("  Platform:       ✗ Unsupported OS")
            };
            lines.push(Line::from(platform_line));

            // GPU detection check
            let gpu_line = if hw_result.gpu_detected {
                if let Some(model) = &hw_result.gpu_model {
                    format!("  GPU:            ✓ {}", model)
                } else {
                    format!("  GPU:            ✓ Detected")
                }
            } else {
                format!("  GPU:            ✗ Not detected")
            };
            lines.push(Line::from(gpu_line));

            // Driver path check
            let driver_line = if let Some(path) = &hw_result.driver_path {
                format!("  Driver Path:    ✓ {}", path)
            } else {
                format!("  Driver Path:    ✗ iHD driver not found")
            };
            lines.push(Line::from(driver_line));

            // VA-API check
            let vaapi_line = if hw_result.vaapi_ok {
                format!("  VA-API VP9:     ✓ Supported")
            } else {
                format!("  VA-API VP9:     ✗ Not available")
            };
            lines.push(Line::from(vaapi_line));

            // FFmpeg encoder check
            let encoder_line = if hw_result.encoder_ok {
                format!("  FFmpeg Encoder: ✓ vp9_vaapi available")
            } else {
                format!("  FFmpeg Encoder: ✗ vp9_vaapi not found")
            };
            lines.push(Line::from(encoder_line));

            // GPU metrics dependency
            let gpu_stats_line = if state.gpu_metrics_available {
                "  GPU Metrics:     ✓ xpu-smi available (GPU/VRAM graphs enabled)".to_string()
            } else {
                "  GPU Metrics:     ✗ xpu-smi not found (install Intel xpu-smi for GPU graphs)"
                    .to_string()
            };
            lines.push(Line::from(gpu_stats_line));

            // HuC firmware check (required for VBR/CBR modes)
            if let Some(huc_loaded) = state.huc_available {
                let huc_line = if huc_loaded {
                    format!("  HuC Firmware:   ✓ Loaded (VBR/CBR available)")
                } else {
                    format!("  HuC Firmware:   ✗ Not loaded (CQP only)")
                };
                lines.push(Line::from(huc_line));
            } else {
                lines.push(Line::from("  HuC Firmware:   ? Checking..."));
            }

            // Error message if not available
            if !hw_result.available {
                if let Some(err_msg) = &hw_result.error_message {
                    lines.push(Line::from(format!("  Note:           {}", err_msg)));
                }
            }
        } else {
            // Fallback if checks haven't run
            lines.push(Line::from("  Checking..."));
        }

        // Continue with existing content
        lines.extend(vec![
            Line::from(""),
            Line::from("A terminal UI for batch VP9 video encoding with advanced"),
            Line::from("parameter control and real-time monitoring."),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Quick Navigation:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("  H", Style::default().fg(Color::Cyan)),
                Span::raw("     - Toggle this help screen"),
            ]),
            Line::from(vec![
                Span::styled("  Tab", Style::default().fg(Color::Cyan)),
                Span::raw("   - Next help section"),
            ]),
            Line::from(vec![
                Span::styled("  Q", Style::default().fg(Color::Cyan)),
                Span::raw("     - Quit application"),
            ]),
            Line::from(vec![
                Span::styled("  C", Style::default().fg(Color::Cyan)),
                Span::raw("     - Configuration screen"),
            ]),
            Line::from(vec![
                Span::styled("  T", Style::default().fg(Color::Cyan)),
                Span::raw("     - Statistics screen"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Use Tab/Arrows to browse help sections →",
                Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::ITALIC),
            )]),
        ]);

        lines
    }

    fn general_settings_content() -> Vec<Line<'static>> {
        vec![
            Line::from(vec![Span::styled(
                "OUTPUT DIRECTORY",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Path where encoded files are saved"),
            Line::from("  Default: Current directory"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "FILENAME PATTERN",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Template for output filenames"),
            Line::from("  Supports: {filename}, {basename}, {profile}, {ext}"),
            Line::from("  Default: {basename}"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "CONTAINER",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Output container format"),
            Line::from("  Options: webm, mp4, mkv, avi"),
            Line::from("  Recommended: webm for VP9 • Impact: Compatibility"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "FPS (Frame Rate Limit)",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Maximum output frame rate"),
            Line::from("  0 = source (no limit), >0 = cap at specified fps"),
            Line::from("  Impact: Lower fps = smaller files, less smooth motion"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "SCALE WIDTH/HEIGHT",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Maximum output resolution"),
            Line::from("  -2 = source, -1 = auto aspect, >0 = max dimension"),
            Line::from("  Impact: Lower resolution = much smaller files, less detail"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "OVERWRITE",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Overwrite existing output files"),
            Line::from("  Impact: On = replace files, Off = skip existing"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "MAX WORKERS",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Number of concurrent encoding jobs"),
            Line::from("  Recommended: # of CPU cores / 2"),
            Line::from("  Impact: Higher = faster batch, more CPU/RAM usage"),
        ]
    }

    fn hardware_encoding_content() -> Vec<Line<'static>> {
        vec![
            Line::from(vec![Span::styled(
                "INTEL QSV HARDWARE ENCODING (Linux Only)",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from("Hardware-accelerated VP9 encoding using Intel Quick Sync Video"),
            Line::from("on Intel Arc GPUs. Provides significantly faster encoding with"),
            Line::from("lower CPU usage, at the cost of some quality/compression."),
            Line::from(""),
            Line::from(vec![Span::styled(
                "SYSTEM REQUIREMENTS",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("  Platform:  ", Style::default().fg(Color::Gray)),
                Span::raw("Linux only (requires VA-API + iHD driver)"),
            ]),
            Line::from(vec![
                Span::styled("  GPU:       ", Style::default().fg(Color::Gray)),
                Span::raw("Intel Arc (DG2) or newer"),
            ]),
            Line::from(vec![
                Span::styled("  Drivers:   ", Style::default().fg(Color::Gray)),
                Span::raw("libva, libva-intel-driver or intel-media-driver"),
            ]),
            Line::from(vec![
                Span::styled("  FFmpeg:    ", Style::default().fg(Color::Gray)),
                Span::raw("Built with --enable-libmfx or --enable-qsv"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Pre-flight checks run automatically when enabled.",
                Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::ITALIC),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "VAAPI QUALITY (-global_quality)",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Quality level for CQP (Constant Quality Parameter) mode"),
            Line::from("  Range: 1-255 • Lower = better quality, larger files"),
            Line::from("  Recommended: 40-60 (high), 70-100 (good), 120-150 (medium)"),
            Line::from("  Note: Passed directly to FFmpeg (no mapping/transformation)"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "QSV PRESET",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Encoding speed vs quality tradeoff"),
            Line::from("  Options: veryfast, faster, fast, medium, slow, slower, veryslow"),
            Line::from("  Impact: Slower = better quality/compression"),
            Line::from("  Recommended: medium (balanced), slow (quality)"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "LOOK-AHEAD (-look_ahead)",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Enable lookahead for better bitrate allocation"),
            Line::from("  Impact: Improved quality, slightly slower encode"),
            Line::from("  Recommended: On for quality encodes"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "LOOK-AHEAD DEPTH (-look_ahead_depth)",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Number of frames to analyze ahead (requires look-ahead=on)"),
            Line::from("  Range: 10-100 frames"),
            Line::from("  Impact: Higher = better decisions, more latency"),
            Line::from("  Recommended: 40 (balanced), 60-100 (quality)"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "GOP SIZE LIMIT",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  QSV GOP is automatically capped at 240 frames to prevent"),
            Line::from("  long first-frame delays. Larger GOP values will be reduced."),
            Line::from(""),
            Line::from(vec![Span::styled(
                "GPU MONITORING",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  When hardware encoding is active, dashboard shows GPU usage"),
            Line::from("  Requires: xpu-smi (Intel GPU monitoring tool)"),
            Line::from(vec![
                Span::styled("  Install: ", Style::default().fg(Color::Gray)),
                Span::raw("https://github.com/intel/xpumanager"),
            ]),
            Line::from("  Graph colors: Yellow (GPU utilization), Cyan (CPU)"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "QUALITY VS SPEED TRADEOFF",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Hardware: 5-10× faster, ~10-20% larger files vs software"),
            Line::from("  Software: Slower, best compression and quality control"),
            Line::from("  Use hardware for: Quick previews, batch processing"),
            Line::from("  Use software for: Archival, distribution, max quality"),
        ]
    }

    fn rate_control_content() -> Vec<Line<'static>> {
        vec![
            Line::from(vec![Span::styled(
                "RATE CONTROL MODE",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Choose encoding quality strategy"),
            Line::from("  CQ: Best for VOD, quality-based (b:v 0)"),
            Line::from("  CQ+Cap: CQ with maxrate bitrate limit"),
            Line::from("  2-Pass VBR: Best quality/size ratio for distribution"),
            Line::from("  CBR: Constant bitrate for live streaming"),
            Line::from("  Recommended: CQ for archival, 2-Pass for general use"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "CRF (Constant Rate Factor) (-crf)",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Controls quality-to-compression ratio"),
            Line::from("  Range: 0-63 • Lower = better quality, larger files"),
            Line::from("  VP9 CRF 31 ≈ x264 CRF 23 (different scales!)"),
            Line::from("  Recommended: 28-30 (high), 30-33 (good), 34-36 (lower)"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "CPU_USED (-speed)",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Encoding speed preset"),
            Line::from("  Range: 0-15 • Lower = slower encoding, better quality"),
            Line::from("  Impact: Each step ~30% faster but ~5-10% larger file"),
            Line::from("  Recommended: 0-2 (archival), 4 (general), 5-8 (fast)"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "TWO-PASS",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Enable two-pass encoding"),
            Line::from("  Pass 1: Fast analysis • Pass 2: Optimized encode"),
            Line::from("  Impact: ~5-10% better quality for target bitrate"),
            Line::from("  Recommended: On for distribution, Off for quick encodes"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "BITRATE CONTROLS (-b:v, -minrate, -maxrate)",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Target bitrate, min/max bounds, buffer size"),
            Line::from("  For CQ mode: Set target to 0 (unconstrained)"),
            Line::from("  For VBR: Set target bitrate, maxrate ~1.5× target"),
            Line::from("  For CBR: Set all three equal for constant bitrate"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "UNDERSHOOT/OVERSHOOT PCT",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  How much to allow deviation from target bitrate"),
            Line::from("  Range: 0-100% • Impact: Affects bitrate consistency"),
            Line::from("  Recommended: 25-50% undershoot, 50-100% overshoot"),
        ]
    }

    fn parallelism_content() -> Vec<Line<'static>> {
        vec![
            Line::from(vec![Span::styled(
                "ROW-MT (Row Multi-Threading) (-row-mt)",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Enable row-based parallel encoding"),
            Line::from("  Impact: ~2-4× faster on multi-core CPUs, no quality loss"),
            Line::from("  Recommended: Always enable (on)"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "TILE COLUMNS (-tile-columns)",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Number of vertical tile divisions (log2)"),
            Line::from("  Range: 0-6 • 0=1 tile, 1=2 tiles, 2=4 tiles, 3=8 tiles"),
            Line::from("  Impact: More tiles = faster encode, slightly lower quality"),
            Line::from("  Recommended: 2 (1080p), 3 (4K), with row-mt enabled"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "TILE ROWS (-tile-rows)",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Number of horizontal tile divisions (log2)"),
            Line::from("  Range: 0-6 • Usually kept at 0 or 1"),
            Line::from("  Impact: Similar to columns but less effective"),
            Line::from("  Recommended: 0 (most cases), 1 (4K+)"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "THREADS (-threads)",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Number of encoding threads"),
            Line::from("  0 = auto-detect optimal count • >0 = specific count"),
            Line::from("  Impact: More threads = faster, diminishing returns >8"),
            Line::from("  Recommended: 0 (auto) or # of CPU cores"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "FRAME PARALLEL (-frame-parallel)",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Enable frame-level parallelism for decoding"),
            Line::from("  Impact: Faster decode on playback, slightly larger file"),
            Line::from("  Recommended: On for streaming, Off for archival"),
        ]
    }

    fn gop_keyframes_content() -> Vec<Line<'static>> {
        vec![
            Line::from(vec![Span::styled(
                "GOP LENGTH (-g)",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Maximum frames between keyframes"),
            Line::from("  Recommended: 10× fps (e.g., 240 for 24fps = 10 sec)"),
            Line::from("  Impact: Larger GOP = better compression, worse seeking"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "KEYINT MIN (-keyint_min)",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Minimum frames between keyframes"),
            Line::from("  Range: 0 to GOP length"),
            Line::from("  Impact: Prevents too-frequent keyframes in static scenes"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "FIXED GOP",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Force keyframes at exact GOP intervals"),
            Line::from("  Impact: Predictable seeking, may reduce quality slightly"),
            Line::from("  Recommended: Off for quality, On for streaming"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "LAG IN FRAMES (-lag-in-frames)",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Lookahead frames for better decisions"),
            Line::from("  Range: 0-25 • Higher = better quality, slower encode"),
            Line::from("  Recommended: 25 (quality), 0 (live/realtime)"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "AUTO ALT-REF (-auto-alt-ref)",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Enable alternate reference frames"),
            Line::from("  Impact: ~10-20% better quality, requires lag > 0"),
            Line::from("  Recommended: On (unless realtime encoding)"),
        ]
    }

    fn advanced_tuning_content() -> Vec<Line<'static>> {
        vec![
            Line::from(vec![Span::styled(
                "AQ MODE (Adaptive Quantization) (-aq-mode)",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  How encoder adjusts quality per-region"),
            Line::from("  0=Off, 1=Variance (default), 2=Complexity, 3=Cyclic"),
            Line::from("  Recommended: 1 (general), 2 (film), 0 (testing)"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "ARNR (Altref Noise Reduction)",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Denoising for alternate reference frames"),
            Line::from("  Max Frames: 0-15 • Strength: 0-6 • Type: -1=Auto"),
            Line::from("  Impact: Reduces noise, may soften detail"),
            Line::from("  Recommended: Defaults (7, 5, auto) or 0 to disable"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "ENABLE TPL (-enable-tpl)",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Temporal dependency modeling"),
            Line::from("  Impact: Better quality for motion, slower encode"),
            Line::from("  Recommended: On (modern VP9 default)"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "SHARPNESS (-sharpness)",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Filter sharpness level"),
            Line::from("  Range: 0-7 • 0=sharpest, 7=smoothest"),
            Line::from("  Impact: Higher = less detail, smaller file"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "NOISE SENSITIVITY (-noise-sensitivity)",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Pre-encode noise filtering"),
            Line::from("  Range: 0-6 • Higher = more aggressive filtering"),
            Line::from("  Recommended: 0 (off) for clean sources"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "STATIC THRESH (-static-thresh)",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Skip encoding blocks below motion threshold"),
            Line::from("  Range: 0-∞ • 0=disabled • Higher = skip more"),
            Line::from("  Impact: Faster encode, may introduce artifacts"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "COLOR SETTINGS",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Colorspace, primaries, transfer, range"),
            Line::from("  -1 = Auto-detect from source"),
            Line::from("  Recommended: Leave at -1 unless converting HDR/SDR"),
        ]
    }

    fn audio_settings_content() -> Vec<Line<'static>> {
        vec![
            Line::from(vec![Span::styled(
                "AUDIO CODEC",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Audio encoder to use"),
            Line::from("  copy: Copy without re-encoding (fast, preserves quality)"),
            Line::from("  libopus: Recommended for webm (efficient, good quality)"),
            Line::from("  aac: Good compatibility for mp4/mkv"),
            Line::from("  Recommended: libopus for webm, aac for mp4"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "AUDIO BITRATE (-b:a)",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  Target audio bitrate in kbps"),
            Line::from("  Range: 32-512 kbps typical"),
            Line::from("  Recommended: 96-128k (Opus), 128-192k (AAC)"),
            Line::from("  Impact: Higher = better quality, larger file"),
        ]
    }

    fn keyboard_shortcuts_content() -> Vec<Line<'static>> {
        vec![
            Line::from(vec![Span::styled(
                "GLOBAL KEYS",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("  H      ", Style::default().fg(Color::Yellow)),
                Span::raw("- Toggle this help screen"),
            ]),
            Line::from(vec![
                Span::styled("  Q      ", Style::default().fg(Color::Yellow)),
                Span::raw("- Quit application"),
            ]),
            Line::from(vec![
                Span::styled("  Esc    ", Style::default().fg(Color::Yellow)),
                Span::raw("- Back / Cancel / Close dialog"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "DASHBOARD SCREEN",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("  C      ", Style::default().fg(Color::Yellow)),
                Span::raw("- Switch to Config screen"),
            ]),
            Line::from(vec![
                Span::styled("  T      ", Style::default().fg(Color::Yellow)),
                Span::raw("- Switch to Stats screen"),
            ]),
            Line::from(vec![
                Span::styled("  S      ", Style::default().fg(Color::Yellow)),
                Span::raw("- Start encoding selected job"),
            ]),
            Line::from(vec![
                Span::styled("  R      ", Style::default().fg(Color::Yellow)),
                Span::raw("- Rescan directory for new files"),
            ]),
            Line::from(vec![
                Span::styled("  D/Del  ", Style::default().fg(Color::Yellow)),
                Span::raw("- Delete selected job"),
            ]),
            Line::from(vec![
                Span::styled("  ↑/↓    ", Style::default().fg(Color::Yellow)),
                Span::raw("- Navigate job list"),
            ]),
            Line::from(vec![
                Span::styled("  Tab    ", Style::default().fg(Color::Yellow)),
                Span::raw("- Cycle foreground job"),
            ]),
            Line::from(vec![
                Span::styled("  [/]    ", Style::default().fg(Color::Yellow)),
                Span::raw("- Decrease/Increase worker count"),
            ]),
            Line::from(vec![
                Span::styled("  Space  ", Style::default().fg(Color::Yellow)),
                Span::raw("- Toggle job status (Pending ↔ Skipped)"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "CONFIG SCREEN",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("  ↑/↓    ", Style::default().fg(Color::Yellow)),
                Span::raw("- Navigate fields"),
            ]),
            Line::from(vec![
                Span::styled("  Space  ", Style::default().fg(Color::Yellow)),
                Span::raw("- Toggle checkbox / radio"),
            ]),
            Line::from(vec![
                Span::styled("  Enter  ", Style::default().fg(Color::Yellow)),
                Span::raw("- Edit field / Open dropdown"),
            ]),
            Line::from(vec![
                Span::styled("  Ctrl+S ", Style::default().fg(Color::Yellow)),
                Span::raw("- Save current profile"),
            ]),
            Line::from(vec![
                Span::styled("  Ctrl+D ", Style::default().fg(Color::Yellow)),
                Span::raw("- Delete current profile"),
            ]),
            Line::from(vec![
                Span::styled("  Esc    ", Style::default().fg(Color::Yellow)),
                Span::raw("- Close dropdown / Cancel edit"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "TEXT EDITING",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("  ←/→    ", Style::default().fg(Color::Yellow)),
                Span::raw("- Move cursor"),
            ]),
            Line::from(vec![
                Span::styled("  Home/End", Style::default().fg(Color::Yellow)),
                Span::raw("- Jump to start/end"),
            ]),
            Line::from(vec![
                Span::styled("  Backspace", Style::default().fg(Color::Yellow)),
                Span::raw("- Delete character before cursor"),
            ]),
            Line::from(vec![
                Span::styled("  Delete ", Style::default().fg(Color::Yellow)),
                Span::raw("- Delete character after cursor"),
            ]),
            Line::from(vec![
                Span::styled("  Ctrl+Backspace", Style::default().fg(Color::Yellow)),
                Span::raw("- Delete word before cursor"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "STATS SCREEN",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("  ↑/↓    ", Style::default().fg(Color::Yellow)),
                Span::raw("- Navigate job list"),
            ]),
            Line::from(vec![
                Span::styled("  Space  ", Style::default().fg(Color::Yellow)),
                Span::raw("- Toggle pause on selected job"),
            ]),
            Line::from(vec![
                Span::styled("  D      ", Style::default().fg(Color::Yellow)),
                Span::raw("- Delete selected job"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "HELP SCREEN",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("  Tab/→  ", Style::default().fg(Color::Yellow)),
                Span::raw("- Next help section"),
            ]),
            Line::from(vec![
                Span::styled("  ⇧Tab/← ", Style::default().fg(Color::Yellow)),
                Span::raw("- Previous help section"),
            ]),
            Line::from(vec![
                Span::styled("  ↑/↓,jk ", Style::default().fg(Color::Yellow)),
                Span::raw("- Scroll content line by line"),
            ]),
            Line::from(vec![
                Span::styled("  PgUp/Dn", Style::default().fg(Color::Yellow)),
                Span::raw("- Scroll by page (10 lines)"),
            ]),
            Line::from(vec![
                Span::styled("  Home   ", Style::default().fg(Color::Yellow)),
                Span::raw("- Jump to top of section"),
            ]),
            Line::from(vec![
                Span::styled("  End    ", Style::default().fg(Color::Yellow)),
                Span::raw("- Jump to bottom of section"),
            ]),
            Line::from(vec![
                Span::styled("  Esc/H  ", Style::default().fg(Color::Yellow)),
                Span::raw("- Close help"),
            ]),
        ]
    }
}
