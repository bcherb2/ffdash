use anyhow::Result;
use chrono::Local;
use std::io::Write;

/// Write debug log to ffdash.log in current directory
/// Appends to file, creating it if needed
pub fn write_debug_log(message: &str) -> Result<()> {
    use std::fs::OpenOptions;

    let log_path = std::env::current_dir()?.join("ffdash.log");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    writeln!(file, "[{}] {}", timestamp, message)?;
    Ok(())
}
