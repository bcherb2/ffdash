// Event handling and main UI loop

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
        MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use crate::ui::{
    ConfigScreen, Dashboard, HelpModal, StatsScreen,
    focus::ConfigFocus,
    help::{HelpModalState, HelpSection},
    state::{AppState, Screen},
};

mod config;
mod dashboard;
mod help;
mod stats;
mod workers;

struct ScanConfig {
    root: PathBuf,
    profile: String,
    overwrite: bool,
    custom_output_dir: Option<String>,
    custom_pattern: Option<String>,
    custom_container: Option<String>,
}

fn spawn_scan_thread(config: ScanConfig, tx: mpsc::Sender<UiEvent>) {
    thread::spawn(move || {
        let result = crate::engine::scan_streaming(&config.root, |path| {
            let job = crate::engine::build_job_from_path(
                path,
                &config.profile,
                config.overwrite,
                config.custom_output_dir.as_deref(),
                config.custom_pattern.as_deref(),
                config.custom_container.as_deref(),
            );

            let _ = tx.send(UiEvent::ScanJob(job));
        });

        match result {
            Ok(_) => {
                let _ = tx.send(UiEvent::ScanFinished);
            }
            Err(e) => {
                let _ = tx.send(UiEvent::ScanFailed(e.to_string()));
            }
        }
    });
}

// Event types sent from dedicated event thread to main loop
enum UiEvent {
    Input(Event),                     // Keyboard, mouse, or other terminal events
    Tick,                             // Periodic update for rendering and metrics
    ScanJob(crate::engine::VideoJob), // Discovered job during initial scan
    ScanFinished,                     // Initial scan completed
    ScanFailed(String),               // Initial scan failed
}

/// Spawn a dedicated thread for event polling.
fn spawn_event_thread(tx: mpsc::Sender<UiEvent>) {
    let tick_rate = Duration::from_millis(16); // ~60 FPS

    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            // Calculate timeout until next tick
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or(Duration::from_secs(0));

            // Poll for events with adaptive timeout
            if event::poll(timeout).unwrap_or(false) {
                if let Ok(evt) = event::read() {
                    if tx.send(UiEvent::Input(evt)).is_err() {
                        break; // Main thread dropped the receiver
                    }
                }
            }

            // Send tick if enough time elapsed
            if last_tick.elapsed() >= tick_rate {
                if tx.send(UiEvent::Tick).is_err() {
                    break; // Main thread dropped the receiver
                }
                last_tick = Instant::now();
            }
        }
    });
}

pub fn run_ui() -> io::Result<()> {
    run_ui_with_options(None, None, None, &crate::config::Config::default())
}

pub fn run_ui_with_options(
    directory: Option<std::path::PathBuf>,
    autostart: Option<bool>,
    scan_on_launch: Option<bool>,
    config: &crate::config::Config,
) -> io::Result<()> {
    // Setup terminal with alternate screen (full terminal)
    enable_raw_mode()?;
    let mut stdout = io::stdout();

    // Enter alternate screen and enable mouse capture
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app_state = AppState::default();

    // Load default profile on startup (using config preferences)
    config::initialize_default_profile(&mut app_state, config);

    // Run hardware encoding preflight check if enabled in config
    if config.defaults.use_hardware_encoding {
        let result = crate::engine::hardware::run_preflight();
        app_state.config.hw_encoding_available = Some(result.available);
        app_state.config.hw_availability_message = result.error_message.clone();

        if result.available {
            app_state.dashboard.gpu_model = result.gpu_model;
            app_state.dashboard.gpu_available = crate::engine::hardware::xpu_smi_available();
        }
    }

    // Determine root directory
    // Priority: CLI arg > current directory
    // (default_directory from config is ignored - app always works on current dir unless told otherwise)
    let root = directory.unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    });

    // Determine whether to scan on launch (CLI flag > config > default)
    let should_scan = scan_on_launch.unwrap_or(config.startup.scan_on_launch);
    // Determine whether to autostart (CLI flag > config > default)
    let should_autostart = autostart.unwrap_or(config.startup.autostart);
    app_state.config.max_workers = config.defaults.max_workers;
    app_state.root_path = Some(root.clone());

    // Wire up UI event channel (shared with background scan)
    let (event_tx, event_rx) = mpsc::channel();
    spawn_event_thread(event_tx.clone());

    if should_scan {
        let container_options = ["webm", "mp4", "mkv", "avi"];
        let custom_container = app_state
            .config
            .container_dropdown_state
            .selected()
            .and_then(|idx| container_options.get(idx))
            .copied()
            .unwrap_or("webm")
            .to_string();

        let scan_config = ScanConfig {
            root: root.clone(),
            profile: config.defaults.profile.clone(),
            overwrite: app_state.config.overwrite,
            custom_output_dir: if app_state.config.output_dir.is_empty() {
                None
            } else {
                Some(app_state.config.output_dir.clone())
            },
            custom_pattern: app_state.config.filename_pattern.clone(),
            custom_container: Some(custom_container),
        };

        // Initialize enc_state so skip toggles stay in sync while jobs stream in
        app_state.enc_state = Some(crate::engine::EncState::new(
            Vec::new(),
            scan_config.profile.clone(),
            root.clone(),
        ));

        app_state.scan_in_progress = true;
        app_state.pending_autostart = should_autostart;

        spawn_scan_thread(scan_config, event_tx.clone());
    }

    // Main loop
    let result = run_app(&mut terminal, &mut app_state, event_rx);

    // Restore terminal: leave alternate screen and disable mouse capture
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    state: &mut AppState,
    event_rx: Receiver<UiEvent>,
) -> io::Result<()> {
    loop {
        // Collect all pending events so we can coalesce tick bursts and keep inputs snappy
        let mut pending_ticks: u64 = 0;
        let mut pending_inputs: Vec<Event> = Vec::new();
        let mut pending_scan_jobs: Vec<crate::engine::VideoJob> = Vec::new();
        let mut scan_finished = false;
        let mut scan_error: Option<String> = None;

        // Always block for at least one event, then drain the queue
        match event_rx.recv() {
            Ok(evt) => match evt {
                UiEvent::Tick => pending_ticks += 1,
                UiEvent::Input(ev) => pending_inputs.push(ev),
                UiEvent::ScanJob(job) => pending_scan_jobs.push(job),
                UiEvent::ScanFinished => scan_finished = true,
                UiEvent::ScanFailed(err) => scan_error = Some(err),
            },
            Err(_) => {
                // Channel closed, exit
                return Ok(());
            }
        }

        while let Ok(evt) = event_rx.try_recv() {
            match evt {
                UiEvent::Tick => pending_ticks += 1,
                UiEvent::Input(ev) => pending_inputs.push(ev),
                UiEvent::ScanJob(job) => pending_scan_jobs.push(job),
                UiEvent::ScanFinished => scan_finished = true,
                UiEvent::ScanFailed(err) => scan_error = Some(err),
            }
        }

        if !pending_scan_jobs.is_empty() {
            for job in pending_scan_jobs {
                add_scanned_job(state, job);
            }
        }

        if let Some(err) = scan_error {
            state.scan_in_progress = false;
            state.pending_autostart = false;
            eprintln!("Failed to scan directory: {}", err);
        }

        if scan_finished {
            state.scan_in_progress = false;

            // Persist the discovered queue
            if let Some(ref mut enc_state) = state.enc_state {
                enc_state.jobs = state.dashboard.jobs.clone();
                if let Some(root) = &state.root_path {
                    let _ = enc_state.save(root);
                }
            }

            // If autostart was requested, kick it off now that jobs are loaded
            if state.pending_autostart && !state.dashboard.jobs.is_empty() {
                if let Err(e) = workers::start_encoding_from_loaded_jobs(state) {
                    eprintln!("Warning: Failed to auto-start encoding: {}", e);
                    eprintln!(
                        "Jobs are loaded but not started. You can start manually from the dashboard."
                    );
                }
                state.pending_autostart = false;
            } else {
                state.pending_autostart = false;
            }
        }

        // Process input events first so user commands are never stuck behind a tick backlog
        for input in pending_inputs {
            match input {
                Event::Key(key) => {
                    if handle_key(key, state) {
                        return Ok(());
                    }
                }
                Event::Mouse(mouse) => {
                    handle_mouse(mouse, state);
                }
                _ => {
                    // Other events (resize, etc.) - ignore for now
                }
            }
        }

        if pending_ticks > 0 {
            // Update metrics on tick (~60 FPS)
            let now = Instant::now();
            if now.duration_since(state.last_metrics_update) >= Duration::from_millis(500) {
                workers::update_metrics(state);
                state.last_metrics_update = now;
            }
        }

        // Poll worker messages (non-blocking, limit to prevent UI blocking)
        if let Some(pool) = state.worker_pool.clone() {
            // Process at most 10 messages per frame to keep UI responsive
            for _ in 0..10 {
                match pool.receiver().try_recv() {
                    Ok(msg) => workers::handle_worker_message(msg, state),
                    Err(_) => break, // No more messages
                }
            }
        }

        // Render after processing event
        terminal.draw(|frame| {
            match state.current_screen {
                Screen::Dashboard => {
                    state.viewport = frame.area();
                    let target_workers = state.config.max_workers;
                    let active_workers = state
                        .worker_pool
                        .as_ref()
                        .map(|pool| pool.active_count())
                        .unwrap_or(0);
                    Dashboard::render(
                        frame,
                        &mut state.dashboard,
                        target_workers,
                        active_workers,
                        state.config.current_profile_name.as_deref(),
                        state.config.use_hardware_encoding,
                    );
                }
                Screen::Config => {
                    ConfigScreen::render(frame, &mut state.config, &mut state.viewport)
                }
                Screen::Stats => StatsScreen::render(frame, &mut state.stats),
            }

            // Render help modal on top if active
            if let Some(ref mut help_state) = state.help_modal {
                HelpModal::render(frame, help_state);
            }
        })?;
    }
}

fn add_scanned_job(state: &mut AppState, job: crate::engine::VideoJob) {
    let select_first = state.dashboard.jobs.is_empty();
    state.dashboard.jobs.push(job.clone());

    if let Some(ref mut enc_state) = state.enc_state {
        enc_state.jobs.push(job);
    }

    if select_first {
        state.dashboard.table_state.select(Some(0));
    }
}

fn should_quit(key: &KeyEvent, _state: &AppState) -> bool {
    // Quit on 'q' or Ctrl+C
    matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q'))
        || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
}

fn handle_key(key: KeyEvent, state: &mut AppState) -> bool {
    // Check if help modal is open - handle help keys first
    if state.help_modal.is_some() {
        help::handle_help_key(key, state);
        return false;
    }

    // Check input mode - if editing text, don't process global shortcuts (q, h)
    let is_editing = if state.current_screen == Screen::Config {
        use crate::ui::state::InputMode;
        state.config.input_mode == InputMode::Editing
    } else {
        false
    };

    // Only check for quit/help in Normal mode (not while editing text)
    if !is_editing {
        // Check for quit (q or Ctrl+C)
        if should_quit(&key, state) {
            return true;
        }

        // Check for 'H' to open help from any screen
        if matches!(key.code, KeyCode::Char('h') | KeyCode::Char('H')) {
            help::open_help(state);
            return false;
        }
    }

    // Handle screen-specific keys
    match state.current_screen {
        Screen::Dashboard => dashboard::handle_dashboard_key(key, state),
        Screen::Config => config::handle_config_key(key, state),
        Screen::Stats => stats::handle_stats_key(key, state),
    }

    false
}

fn handle_mouse(mouse: MouseEvent, state: &mut AppState) {
    match state.current_screen {
        Screen::Dashboard => dashboard::handle_dashboard_mouse(mouse, state),
        Screen::Config => config::handle_config_mouse(mouse, state),
        Screen::Stats => {} // No mouse handling for stats yet
    }
}
