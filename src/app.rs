use crate::cli::{Cli, Commands};
use ffdash::{config, engine, ui};
use std::process;

fn default_profile_name() -> String {
    config::Config::load()
        .map(|c| c.defaults.profile)
        .unwrap_or_else(|_| "1080p Shrinker".to_string())
}

pub fn run(cli: Cli) {
    // Handle subcommands first
    if let Some(command) = cli.command {
        match command {
            Commands::CheckFfmpeg => handle_check_ffmpeg(),
            Commands::CheckVaapi { test_encode } => handle_check_vaapi(test_encode),
            Commands::Probe { file } => handle_probe(file),
            Commands::Scan {
                directory,
                overwrite,
            } => handle_scan(directory, overwrite),
            Commands::DryRun {
                directory,
                overwrite,
            } => handle_dry_run(directory, overwrite),
            Commands::EncodeOne {
                directory,
                overwrite,
            } => handle_encode_one(directory, overwrite),
            Commands::InitConfig => handle_init_config(),
        }
        return;
    }

    // Determine startup behavior from CLI flags and config
    let config = config::Config::load().unwrap_or_default();

    let autostart = if cli.autostart {
        Some(true)
    } else if cli.no_autostart {
        Some(false)
    } else {
        None // Use config default
    };

    let scan_on_launch = if cli.scan {
        Some(true)
    } else if cli.no_scan {
        Some(false)
    } else {
        None // Use config default
    };

    // Launch TUI (default behavior)
    if let Err(e) = ui::run_ui_with_options(cli.directory, autostart, scan_on_launch, &config) {
        eprintln!("Error running UI: {}", e);
        process::exit(1);
    }
}

fn handle_check_ffmpeg() {
    match engine::ffmpeg_version() {
        Ok(version) => {
            println!("ffmpeg found: {}", version);
            match engine::ffprobe_version() {
                Ok(probe_version) => {
                    println!("ffprobe found: {}", probe_version);
                    process::exit(0);
                }
                Err(e) => {
                    eprintln!("Error: {:#}", e);
                    process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {:#}", e);
            process::exit(1);
        }
    }
}

fn handle_check_vaapi(test_encode: bool) {
    use engine::hardware;
    use std::process::Command;

    println!("=== VAAPI Hardware Encoding Diagnostics ===\n");

    // 1. Check environment variable
    println!("1. Environment Check:");
    match std::env::var("LIBVA_DRIVERS_PATH") {
        Ok(path) => println!("   LIBVA_DRIVERS_PATH={} (set externally)", path),
        Err(_) => println!("   LIBVA_DRIVERS_PATH not set (will auto-detect)"),
    }
    println!();

    // 2. Run preflight checks
    println!("2. Preflight Checks:");
    let preflight = hardware::run_preflight();
    println!(
        "   Platform (Linux): {}",
        if preflight.platform_ok { "OK" } else { "FAIL" }
    );
    println!(
        "   GPU detected: {}",
        if preflight.gpu_detected { "OK" } else { "FAIL" }
    );
    if let Some(ref model) = preflight.gpu_model {
        println!("   GPU model: {}", model);
    }
    println!(
        "   VAAPI VP9: {}",
        if preflight.vaapi_ok { "OK" } else { "FAIL" }
    );
    println!(
        "   FFmpeg vp9_vaapi: {}",
        if preflight.encoder_ok { "OK" } else { "FAIL" }
    );
    println!();

    // 3. Show hints about HuC firmware
    println!("3. HuC Firmware:");
    let huc_loaded = hardware::check_huc_loaded();
    println!(
        "   HuC loaded: {}",
        if huc_loaded {
            "YES (OK)"
        } else {
            "NO (see docs)"
        }
    );
    println!();

    // 4. Optional test encode
    if test_encode {
        println!("4. Test Encode:");
        let sample_input = std::env::var("FFDASH_SAMPLE_INPUT")
            .unwrap_or_else(|_| "/workspace/samples/input.mp4".to_string());

        let output_path = "/tmp/ffdash_vaapi_test.webm";

        println!("   Running test encode on {}", sample_input);
        let status = Command::new("ffmpeg")
            .args([
                "-y",
                "-hwaccel",
                "vaapi",
                "-hwaccel_output_format",
                "vaapi",
                "-i",
                &sample_input,
                "-c:v",
                "vp9_vaapi",
                "-b:v",
                "0",
                "-rc_mode",
                "CQP",
                "-qp",
                "30",
                output_path,
            ])
            .status();

        match status {
            Ok(s) if s.success() => println!("   Test encode succeeded, output: {}", output_path),
            Ok(s) => println!("   Test encode failed with status: {}", s),
            Err(e) => println!("   Failed to run ffmpeg: {}", e),
        }
    }
}

fn handle_probe(file: std::path::PathBuf) {
    match engine::probe_duration(&file) {
        Ok(duration) => {
            println!("Duration: {:.2} seconds", duration);
        }
        Err(e) => {
            eprintln!("Error: {:#}", e);
            process::exit(1);
        }
    }
}

fn handle_scan(directory: Option<std::path::PathBuf>, overwrite: bool) {
    let dir = directory.unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    });
    println!("Scanning directory: {}", dir.display());

    match engine::scan(&dir) {
        Ok(files) => {
            let profile = default_profile_name();
            let custom_output_dir: Option<&str> = None;
            let custom_pattern: Option<&str> = None;
            let custom_container: Option<&str> = None;

            let jobs = engine::build_job_queue(
                files,
                &profile,
                overwrite,
                custom_output_dir,
                custom_pattern,
                custom_container,
            );

            for job in &jobs {
                println!(
                    "- {} -> {}",
                    job.input_path.display(),
                    job.output_path.display()
                );
            }
            println!("Total jobs: {}", jobs.len());
        }
        Err(e) => {
            eprintln!("Error scanning directory: {:#}", e);
            process::exit(1);
        }
    }
}

fn handle_dry_run(directory: Option<std::path::PathBuf>, overwrite: bool) {
    let dir = directory.unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    });
    println!("Dry run: building ffmpeg commands for {}", dir.display());

    match engine::scan(&dir) {
        Ok(files) => {
            let profile = default_profile_name();
            let custom_output_dir: Option<&str> = None;
            let custom_pattern: Option<&str> = None;
            let custom_container: Option<&str> = None;

            let jobs = engine::build_job_queue(
                files,
                &profile,
                overwrite,
                custom_output_dir,
                custom_pattern,
                custom_container,
            );
            for job in &jobs {
                let cmd = engine::build_ffmpeg_cmd(job, None);
                println!("{:?}", cmd);
            }
        }
        Err(e) => {
            eprintln!("Error scanning directory: {:#}", e);
            process::exit(1);
        }
    }
}

fn handle_encode_one(directory: Option<std::path::PathBuf>, overwrite: bool) {
    let dir = directory.unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    });

    match engine::scan(&dir) {
        Ok(files) => {
            if files.is_empty() {
                eprintln!("No video files found in {}", dir.display());
                process::exit(0);
            }

            let profile_name = default_profile_name();
            let custom_output_dir: Option<&str> = None;
            let custom_pattern: Option<&str> = None;
            let custom_container: Option<&str> = None;

            let mut jobs = engine::build_job_queue(
                files,
                &profile_name,
                overwrite,
                custom_output_dir,
                custom_pattern,
                custom_container,
            );

            if let Some(first_job) = jobs.get_mut(0) {
                match engine::encode_job(first_job) {
                    Ok(_) => println!("Encoded: {}", first_job.output_path.display()),
                    Err(e) => eprintln!("Encoding failed: {:#}", e),
                }
            }
        }
        Err(e) => {
            eprintln!("Error scanning directory: {:#}", e);
            process::exit(1);
        }
    }
}

fn handle_init_config() {
    match config::Config::load() {
        Ok(cfg) => {
            match config::Config::config_path() {
                Ok(path) => println!("Config loaded successfully from {}", path.display()),
                Err(e) => println!("Config loaded, but config path unknown: {:#}", e),
            }
            println!("{:#?}", cfg);
        }
        Err(e) => {
            println!("Config missing or invalid: {:#}", e);
            println!("Creating default config...");

            let cfg = config::Config::default();
            if let Err(err) = cfg.save() {
                eprintln!("Failed to save default config: {:#}", err);
                process::exit(1);
            } else {
                match config::Config::config_path() {
                    Ok(path) => println!("Default config saved to {}", path.display()),
                    Err(e) => println!("Default config saved (path unknown): {:#}", e),
                }
            }
        }
    }
}
