//! Intel Arc GPU hardware encoding detection and monitoring

use std::process::Command;

/// QSV preset options (veryfast to veryslow)
pub const QSV_PRESETS: &[&str] = &[
    "veryfast", "faster", "fast", "medium", "slow", "slower", "veryslow",
];

/// VA-API driver information detected at runtime
#[derive(Debug, Clone)]
pub struct VaapiDriver {
    pub path: String,      // e.g., /usr/lib/x86_64-linux-gnu/dri
    pub name: String,      // e.g., iHD, i965, radeonsi
    pub full_path: String, // e.g., /usr/lib/x86_64-linux-gnu/dri/iHD_drv_video.so
}

/// Complete VA-API configuration for FFmpeg
#[derive(Debug, Clone)]
pub struct VaapiConfig {
    pub driver: VaapiDriver,
    pub render_device: String, // e.g., /dev/dri/renderD128
}

/// Result of hardware encoding pre-flight checks
#[derive(Debug, Clone)]
pub struct HwPreflightResult {
    pub available: bool,
    pub platform_ok: bool,
    pub gpu_detected: bool,
    pub vaapi_ok: bool,
    pub encoder_ok: bool,
    pub gpu_model: Option<String>,
    pub driver_path: Option<String>,
    pub error_message: Option<String>,
}

/// Run all pre-flight checks for VAAPI hardware encoding
pub fn run_preflight() -> HwPreflightResult {
    let platform_ok = cfg!(target_os = "linux");
    let gpu_model = detect_intel_arc();
    let gpu_detected = gpu_model.is_some();
    let driver_path = detect_vaapi_driver_path();
    let vaapi_ok = platform_ok && check_vaapi_vp9();
    let encoder_ok = platform_ok && check_ffmpeg_vaapi();

    let available = platform_ok && gpu_detected && vaapi_ok && encoder_ok;

    let error_message = if !platform_ok {
        Some("Linux only".to_string())
    } else if !gpu_detected {
        Some("No Intel Arc GPU".to_string())
    } else if driver_path.is_none() {
        Some("VAAPI driver not found".to_string())
    } else if !vaapi_ok {
        Some("VA-API VP9 unavailable".to_string())
    } else if !encoder_ok {
        Some("FFmpeg vp9_vaapi not found".to_string())
    } else {
        None
    };

    HwPreflightResult {
        available,
        platform_ok,
        gpu_detected,
        vaapi_ok,
        encoder_ok,
        gpu_model,
        driver_path,
        error_message,
    }
}

/// Detect Intel Arc GPU using lspci
fn detect_intel_arc() -> Option<String> {
    let output = Command::new("lspci").output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let lower = line.to_lowercase();
        if lower.contains("intel") && (lower.contains("arc") || lower.contains("dg2")) {
            // Extract GPU model name (everything after the last colon)
            return Some(line.split(':').last()?.trim().to_string());
        }
    }
    None
}

// ==================== NEW VAAPI DETECTION SYSTEM ====================

/// Extract driver name from filename
/// Example: "iHD_drv_video.so" -> Some("iHD")
pub fn extract_driver_name(filename: &str) -> Option<&str> {
    filename.strip_suffix("_drv_video.so")
}

/// Detect the multi-arch tuple for this system
/// Returns "x86_64-linux-gnu", "aarch64-linux-gnu", etc.
fn detect_multiarch_tuple() -> Option<String> {
    // Method 1: Check dpkg architecture (Debian/Ubuntu)
    if let Ok(output) = Command::new("dpkg-architecture")
        .arg("-qDEB_HOST_MULTIARCH")
        .output()
    {
        if output.status.success() {
            let tuple = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !tuple.is_empty() {
                return Some(tuple);
            }
        }
    }

    // Method 2: Use compile-time target
    #[cfg(target_arch = "x86_64")]
    {
        return Some("x86_64-linux-gnu".to_string());
    }

    #[cfg(target_arch = "aarch64")]
    {
        return Some("aarch64-linux-gnu".to_string());
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        None
    }
}

/// Get library paths to search for VA-API drivers
/// Dynamically builds paths based on system architecture
fn get_vaapi_search_paths() -> Vec<std::path::PathBuf> {
    use std::path::PathBuf;
    let mut paths = Vec::new();

    // 1. Check LIBVA_DRIVERS_PATH env var first (user override)
    if let Ok(env_path) = std::env::var("LIBVA_DRIVERS_PATH") {
        paths.push(PathBuf::from(env_path));
    }

    // 2. Detect multi-arch tuple dynamically
    let multiarch = detect_multiarch_tuple();

    // 3. Build standard paths (priority order)
    if let Some(ref ma) = multiarch {
        paths.push(PathBuf::from(format!("/usr/lib/{}/dri", ma)));
        paths.push(PathBuf::from(format!("/usr/local/lib/{}/dri", ma)));
    }

    // Generic fallbacks
    paths.push(PathBuf::from("/usr/lib/dri"));
    paths.push(PathBuf::from("/usr/local/lib/dri"));
    paths.push(PathBuf::from("/usr/lib64/dri"));
    paths.push(PathBuf::from("/usr/local/lib64/dri"));

    paths
}

/// Find all VA-API drivers in a directory
/// Returns list of (driver_name, full_path) tuples
fn find_drivers_in_path(dir: &std::path::Path) -> Vec<(String, std::path::PathBuf)> {
    let mut drivers = Vec::new();

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                // Match pattern: *_drv_video.so
                if filename.ends_with("_drv_video.so") {
                    // Extract driver name: "iHD_drv_video.so" -> "iHD"
                    if let Some(name) = filename.strip_suffix("_drv_video.so") {
                        drivers.push((name.to_string(), path));
                    }
                }
            }
        }
    }

    drivers
}

/// Detect GPU info from lspci (any vendor)
fn detect_gpu_info() -> Option<String> {
    let output = Command::new("lspci").output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let lower = line.to_lowercase();
        // Look for VGA or Display controllers
        if lower.contains("vga") || lower.contains("display") || lower.contains("3d") {
            return Some(line.to_string());
        }
    }
    None
}

/// Map detected GPU vendor to preferred driver
/// Returns driver names in priority order for the vendor
fn get_preferred_drivers_for_gpu() -> Vec<&'static str> {
    // Detect GPU vendor from lspci
    if let Some(gpu_info) = detect_gpu_info() {
        let lower = gpu_info.to_lowercase();

        if lower.contains("intel") {
            // Intel: iHD (modern), i965 (legacy)
            return vec!["iHD", "i965"];
        } else if lower.contains("amd") || lower.contains("radeon") {
            // AMD: radeonsi (modern), r600 (legacy)
            return vec!["radeonsi", "r600"];
        } else if lower.contains("nvidia") {
            // NVIDIA: nouveau (open source)
            return vec!["nouveau"];
        }
    }

    // Default priority if GPU not detected
    vec!["iHD", "i965", "radeonsi", "nouveau"]
}

/// Detect available render device
/// Scans /dev/dri/renderD* and returns the first available
fn detect_render_device() -> Option<String> {
    use std::path::Path;
    let dri_path = Path::new("/dev/dri");

    if !dri_path.exists() {
        return None;
    }

    // Collect all renderD* devices
    let mut devices: Vec<_> = std::fs::read_dir(dri_path)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(|n| n.starts_with("renderD"))
                .unwrap_or(false)
        })
        .map(|e| e.path())
        .collect();

    // Sort to get renderD128 before renderD129, etc.
    devices.sort();

    devices.first().map(|p| p.to_string_lossy().to_string())
}

/// Log helper for VAAPI detection
fn log_to_file(msg: &str) {
    use std::fs::OpenOptions;
    use std::io::Write;

    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(cwd.join("ffdash.log"))
        {
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
            let _ = writeln!(file, "[{}] {}", timestamp, msg);
        }
    }
}

/// Implementation of VA-API config detection (without caching)
fn detect_vaapi_config_impl() -> Option<VaapiConfig> {
    use std::path::Path;

    log_to_file("[VAAPI] Starting driver detection...");

    // 1. Check if user has set both env vars (full override)
    if let (Ok(path), Ok(name)) = (
        std::env::var("LIBVA_DRIVERS_PATH"),
        std::env::var("LIBVA_DRIVER_NAME"),
    ) {
        let full_path = format!("{}/{}_drv_video.so", path, name);
        if Path::new(&full_path).exists() {
            log_to_file(&format!("[VAAPI] Using env override: {} at {}", name, path));
            return Some(VaapiConfig {
                driver: VaapiDriver {
                    path: path.clone(),
                    name: name.clone(),
                    full_path,
                },
                render_device: detect_render_device()
                    .unwrap_or_else(|| "/dev/dri/renderD128".to_string()),
            });
        }
    }

    // 2. Get preferred drivers based on GPU
    let preferred = get_preferred_drivers_for_gpu();
    log_to_file(&format!(
        "[VAAPI] Preferred drivers for this GPU: {:?}",
        preferred
    ));

    // 3. Search paths for drivers
    let search_paths = get_vaapi_search_paths();
    log_to_file(&format!(
        "[VAAPI] Search paths: {} paths",
        search_paths.len()
    ));

    // 4. Find all available drivers
    let mut all_drivers: Vec<(String, std::path::PathBuf, std::path::PathBuf)> = Vec::new();

    for dir in &search_paths {
        for (name, full_path) in find_drivers_in_path(dir) {
            log_to_file(&format!(
                "[VAAPI]   Found: {} at {}",
                name,
                full_path.display()
            ));
            all_drivers.push((name, dir.clone(), full_path));
        }
    }

    if all_drivers.is_empty() {
        log_to_file("[VAAPI] ERROR: No VA-API drivers found!");
        return None;
    }

    // 5. Select best driver based on GPU preference
    let selected = preferred
        .iter()
        .find_map(|pref| {
            all_drivers
                .iter()
                .find(|(name, _, _)| name == *pref)
                .cloned()
        })
        .or_else(|| all_drivers.first().cloned());

    let (name, dir, full_path) = selected?;

    // 6. Detect render device
    let render_device = detect_render_device().unwrap_or_else(|| "/dev/dri/renderD128".to_string());

    log_to_file(&format!(
        "[VAAPI] Selected driver: {} at {}",
        name,
        dir.display()
    ));
    log_to_file(&format!("[VAAPI] Render device: {}", render_device));

    Some(VaapiConfig {
        driver: VaapiDriver {
            path: dir.to_string_lossy().to_string(),
            name,
            full_path: full_path.to_string_lossy().to_string(),
        },
        render_device,
    })
}

/// Detect VA-API driver and render device
/// Results are cached after first detection
pub fn detect_vaapi_config() -> Option<VaapiConfig> {
    use std::sync::OnceLock;
    static VAAPI_CONFIG: OnceLock<Option<VaapiConfig>> = OnceLock::new();

    VAAPI_CONFIG
        .get_or_init(|| detect_vaapi_config_impl())
        .clone()
}

// ==================== END NEW VAAPI DETECTION SYSTEM ====================

/// Internal function: detect VAAPI driver in custom search paths (testable)
fn detect_vaapi_driver_in_paths(search_paths: &[&str], log: bool) -> Option<String> {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::path::Path;

    // Log to ffdash.log
    let log_to_file = |msg: &str| {
        if !log {
            return;
        }
        if let Ok(cwd) = std::env::current_dir() {
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(cwd.join("ffdash.log"))
            {
                let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
                let _ = writeln!(file, "[{}] {}", timestamp, msg);
            }
        }
    };

    log_to_file("[VAAPI] Detecting driver path...");

    for path in search_paths {
        let driver_path = Path::new(path).join("iHD_drv_video.so");
        let exists = driver_path.exists();

        log_to_file(&format!(
            "[VAAPI]   Checking {}: {}",
            driver_path.display(),
            if exists { "✓ FOUND" } else { "✗ not found" }
        ));

        if exists {
            log_to_file(&format!("[VAAPI] Selected driver path: {}", path));
            return Some(path.to_string());
        }
    }

    log_to_file("[VAAPI] WARNING: No driver found in any search path!");
    None
}

/// Auto-detect VAAPI driver path by searching common locations
/// Priority: 1) LIBVA_DRIVERS_PATH env var, 2) common system paths
pub fn detect_vaapi_driver_path() -> Option<String> {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::path::Path;

    // Log helper
    let log_to_file = |msg: &str| {
        if let Ok(cwd) = std::env::current_dir() {
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(cwd.join("ffdash.log"))
            {
                let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
                let _ = writeln!(file, "[{}] {}", timestamp, msg);
            }
        }
    };

    // First, check if LIBVA_DRIVERS_PATH is already set
    if let Ok(env_path) = std::env::var("LIBVA_DRIVERS_PATH") {
        let driver_file = Path::new(&env_path).join("iHD_drv_video.so");
        log_to_file(&format!(
            "[VAAPI] LIBVA_DRIVERS_PATH={} (from env)",
            env_path
        ));
        if driver_file.exists() {
            log_to_file(&format!(
                "[VAAPI] Driver found at env path: {}",
                driver_file.display()
            ));
            return Some(env_path);
        } else {
            log_to_file(&format!(
                "[VAAPI] WARNING: Driver NOT found at env path: {}",
                driver_file.display()
            ));
            log_to_file("[VAAPI] Falling back to auto-detection...");
        }
    }

    // Common VAAPI driver locations (in priority order)
    let search_paths = [
        "/usr/lib/x86_64-linux-gnu/dri",       // Ubuntu/Debian
        "/usr/local/lib/x86_64-linux-gnu/dri", // Custom builds (Ubuntu-based)
        "/usr/lib/dri",                        // Some distros
        "/usr/local/lib/dri",                  // Custom builds
        "/usr/lib64/dri",                      // RHEL/Fedora/CentOS
        "/usr/local/lib64/dri",                // Custom builds (RHEL-based)
    ];

    detect_vaapi_driver_in_paths(&search_paths, true)
}

/// Detect driver with custom search paths (for testing)
pub fn detect_vaapi_driver_path_custom(search_paths: &[&str]) -> Option<String> {
    detect_vaapi_driver_in_paths(search_paths, false)
}

/// Detect driver with env var override and custom search paths (for testing env priority)
pub fn detect_vaapi_driver_path_with_env(
    env_path: Option<&str>,
    search_paths: &[&str],
) -> Option<String> {
    use std::path::Path;

    // Check env path first (simulating LIBVA_DRIVERS_PATH)
    if let Some(path) = env_path {
        let driver_file = Path::new(path).join("iHD_drv_video.so");
        if driver_file.exists() {
            return Some(path.to_string());
        }
        // Env path set but driver not found - fall through to search
    }

    // Fall back to search paths
    detect_vaapi_driver_in_paths(search_paths, false)
}

/// Check VA-API for VP9 encoding support
fn check_vaapi_vp9() -> bool {
    let mut cmd = Command::new("vainfo");

    // Set BOTH env vars if we can detect driver
    if let Some(config) = detect_vaapi_config() {
        cmd.env("LIBVA_DRIVERS_PATH", &config.driver.path);
        cmd.env("LIBVA_DRIVER_NAME", &config.driver.name);
    }

    cmd.output()
        .ok()
        .map(|o| {
            let s = String::from_utf8_lossy(&o.stdout);
            // Look for VP9 profile with encode slice support
            s.contains("VAProfileVP9") && s.contains("EntrypointEnc")
        })
        .unwrap_or(false)
}

/// Check if FFmpeg has vp9_vaapi encoder (VAAPI is more reliable than QSV with libvpl)
fn check_ffmpeg_vaapi() -> bool {
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-hide_banner", "-encoders"]);

    // Set BOTH env vars if we can detect driver
    if let Some(config) = detect_vaapi_config() {
        cmd.env("LIBVA_DRIVERS_PATH", &config.driver.path);
        cmd.env("LIBVA_DRIVER_NAME", &config.driver.name);
    }

    cmd.output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("vp9_vaapi"))
        .unwrap_or(false)
}

/// Check if HuC firmware is loaded (required for VBR/CBR modes on Intel Arc)
/// Returns true if HuC is authenticated, false otherwise
pub fn check_huc_loaded() -> bool {
    // Check dmesg for HuC authentication message
    // Note: This requires read access to kernel logs (may need sudo on some systems)
    Command::new("dmesg")
        .output()
        .ok()
        .and_then(|o| {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);

            // Check both stdout and stderr for HuC messages
            let combined = format!("{}{}", stdout, stderr);

            // Look for HuC authentication success message
            // Examples:
            // - "i915 0000:03:00.0: [drm] HuC authenticated"
            // - "i915 0000:03:00.0: [drm] GuC firmware i915/dg2_guc_70.bin version 70.5"
            if combined.to_lowercase().contains("huc")
                && combined.to_lowercase().contains("authenticated")
            {
                return Some(true);
            }

            // Also check for explicit HuC firmware loading
            if combined.to_lowercase().contains("huc")
                && (combined.to_lowercase().contains("loaded")
                    || combined.to_lowercase().contains("version"))
            {
                return Some(true);
            }

            Some(false)
        })
        .unwrap_or(false)
}

// GPU Monitoring (xpu-smi only)

/// GPU usage statistics from xpu-smi
#[derive(Debug, Clone, Default)]
pub struct GpuStats {
    pub utilization: f32,
    pub memory_percent: f32,
}

/// Check if xpu-smi is available
pub fn xpu_smi_available() -> bool {
    Command::new("xpu-smi")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get GPU stats from xpu-smi (JSON output)
pub fn get_gpu_stats() -> Option<GpuStats> {
    let output = Command::new("xpu-smi")
        .args(["stats", "-d", "0", "-j"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    // Parse JSON output
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;

    Some(GpuStats {
        utilization: json["gpu_utilization"].as_f64()? as f32,
        memory_percent: json
            .get("memory_used")
            .and_then(|u| u.as_f64())
            .and_then(|used| {
                json.get("memory_total")
                    .and_then(|t| t.as_f64())
                    .map(|total| (used / total) * 100.0)
            })
            .unwrap_or(0.0) as f32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qsv_presets_count() {
        assert_eq!(QSV_PRESETS.len(), 7);
        assert_eq!(QSV_PRESETS[0], "veryfast");
        assert_eq!(QSV_PRESETS[3], "medium");
        assert_eq!(QSV_PRESETS[6], "veryslow");
    }

    #[test]
    fn test_platform_detection() {
        let is_linux = cfg!(target_os = "linux");
        let result = run_preflight();
        assert_eq!(result.platform_ok, is_linux);
    }

    #[test]
    fn test_preflight_on_non_linux() {
        if !cfg!(target_os = "linux") {
            let result = run_preflight();
            assert!(!result.available);
            assert!(!result.platform_ok);
            assert_eq!(result.error_message, Some("Linux only".to_string()));
        }
    }
}
