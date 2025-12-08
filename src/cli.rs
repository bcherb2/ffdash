use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "ffdash")]
#[command(about = "VP9 batch encoder with TUI", long_about = None)]
pub struct Cli {
    /// Root directory to scan for video files (defaults to current directory)
    #[arg(value_name = "DIRECTORY")]
    pub directory: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Launch the TUI (default if no other flags provided)
    #[arg(long)]
    pub tui: bool,

    /// Automatically start encoding when TUI launches (overrides config)
    #[arg(long, conflicts_with = "no_autostart")]
    pub autostart: bool,

    /// Don't automatically start encoding when TUI launches (overrides config)
    #[arg(long, conflicts_with = "autostart")]
    pub no_autostart: bool,

    /// Scan for files on TUI launch (overrides config)
    #[arg(long, conflicts_with = "no_scan")]
    pub scan: bool,

    /// Don't scan for files on TUI launch (overrides config)
    #[arg(long, conflicts_with = "scan")]
    pub no_scan: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Check if ffmpeg and ffprobe are installed
    CheckFfmpeg,

    /// Check VAAPI hardware encoding availability (diagnose Intel Arc issues)
    CheckVaapi {
        /// Run a quick test encode to verify VAAPI works end-to-end
        #[arg(long)]
        test_encode: bool,
    },

    /// Probe a video file to get its duration
    Probe {
        /// Path to the video file
        file: PathBuf,
    },

    /// Scan directory and show jobs without encoding
    Scan {
        /// Directory to scan (defaults to current directory)
        directory: Option<PathBuf>,

        /// Include files that already have output files (re-encode)
        #[arg(long)]
        overwrite: bool,
    },

    /// Show ffmpeg commands without executing (dry run)
    DryRun {
        /// Directory to scan (defaults to current directory)
        directory: Option<PathBuf>,

        /// Include files that already have output files (re-encode)
        #[arg(long)]
        overwrite: bool,
    },

    /// Encode only the first pending job
    EncodeOne {
        /// Directory to scan (defaults to current directory)
        directory: Option<PathBuf>,

        /// Include files that already have output files (re-encode)
        #[arg(long)]
        overwrite: bool,
    },

    /// Show config status and location, or create default config if missing
    InitConfig,
}

pub fn parse() -> Cli {
    Cli::parse()
}
