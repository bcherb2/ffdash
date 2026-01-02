// Terminal UI using Ratatui

pub mod components;
pub mod config;
pub mod constants;
pub mod dashboard;
pub mod events;
pub mod focus;
pub mod help;
pub mod options;
pub mod quit_modal;
pub mod state;
pub mod stats;
pub mod widgets;

pub use config::ConfigScreen;
pub use dashboard::Dashboard;
pub use events::{run_ui, run_ui_with_options};
pub use help::{HelpModal, HelpModalState, HelpSection};
pub use quit_modal::QuitModal;
pub use state::AppState;
pub use stats::StatsScreen;
