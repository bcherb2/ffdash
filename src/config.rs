// Global configuration management

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub startup: StartupConfig,

    #[serde(default)]
    pub defaults: DefaultsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupConfig {
    /// Whether to automatically start encoding when TUI launches with scanned files
    #[serde(default)]
    pub autostart: bool,

    /// Whether to scan for files on TUI launch (or just start with empty dashboard)
    #[serde(default = "default_scan_on_launch")]
    pub scan_on_launch: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsConfig {
    /// Default profile to use for new scans
    #[serde(default = "default_profile")]
    pub profile: String,

    /// Default number of concurrent workers
    #[serde(default = "default_max_workers")]
    pub max_workers: u32,

    /// Default overwrite setting (whether to overwrite existing output files)
    #[serde(default)]
    pub overwrite: bool,

    /// Last used profile (for restoring user's selection on restart)
    #[serde(default)]
    pub last_used_profile: Option<String>,

    /// Hardware encoding enabled (VAAPI on Linux)
    #[serde(default)]
    pub use_hardware_encoding: bool,

    /// Filename pattern for output files (global setting, not part of profiles)
    /// Supports: {filename}, {basename}, {profile}, {ext}
    #[serde(default = "default_filename_pattern")]
    pub filename_pattern: String,

    /// Prefer source bit depth for HW encodes (auto-select p010 for 10-bit, nv12 for 8-bit)
    #[serde(default = "default_true_config")]
    pub auto_bit_depth: bool,

    /// Disable VAAPI fallback when QSV fails (fail fast instead of retrying)
    #[serde(default)]
    pub disable_vaapi_fallback: bool,
}

fn default_scan_on_launch() -> bool {
    true
}

fn default_profile() -> String {
    "1080p Shrinker".to_string()
}

fn default_max_workers() -> u32 {
    1
}

fn default_true_config() -> bool {
    true
}

fn default_filename_pattern() -> String {
    "{basename}".to_string()
}

impl Default for StartupConfig {
    fn default() -> Self {
        Self {
            autostart: false,
            scan_on_launch: default_scan_on_launch(),
        }
    }
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            profile: default_profile(),
            max_workers: default_max_workers(),
            overwrite: false,              // Default to not overwriting
            last_used_profile: None,       // No profile used yet
            use_hardware_encoding: false,  // Default to software encoding
            filename_pattern: default_filename_pattern(),
            auto_bit_depth: true,          // Use source bit depth for HW surfaces
            disable_vaapi_fallback: false, // Try VAAPI fallback when QSV fails
        }
    }
}

impl Config {
    /// Get the path to the config file
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = if cfg!(target_os = "macos") {
            dirs::home_dir()
                .context("Could not determine home directory")?
                .join(".config")
                .join("ffdash")
        } else if cfg!(target_os = "windows") {
            dirs::config_dir()
                .context("Could not determine config directory")?
                .join("ffdash")
        } else {
            // Linux and others
            dirs::config_dir()
                .context("Could not determine config directory")?
                .join("ffdash")
        };

        Ok(config_dir.join("config.toml"))
    }

    /// Load config from disk, or create default if it doesn't exist
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let contents = fs::read_to_string(&config_path).with_context(|| {
                format!("Failed to read config file: {}", config_path.display())
            })?;

            let config: Config = toml::from_str(&contents).with_context(|| {
                format!("Failed to parse config file: {}", config_path.display())
            })?;

            Ok(config)
        } else {
            // Create default config and save it
            let config = Config::default();

            // Try to save the default config, but don't fail if we can't
            // (e.g., if the directory isn't writable)
            if let Err(e) = config.save() {
                eprintln!("Warning: Could not create default config file: {}", e);
                eprintln!(
                    "Using built-in defaults. Run 'ffdash init-config' to create a config file."
                );
            }

            Ok(config)
        }
    }

    /// Save config to disk
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }

        let contents = toml::to_string_pretty(self).context("Failed to serialize config")?;

        fs::write(&config_path, contents)
            .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

        Ok(())
    }

    /// Check if config file exists
    pub fn exists() -> bool {
        Self::config_path().map(|p| p.exists()).unwrap_or(false)
    }

    /// Create a default config file if it doesn't exist
    pub fn ensure_default() -> Result<()> {
        if !Self::exists() {
            let config = Config::default();
            config.save()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.startup.autostart, false);
        assert_eq!(config.startup.scan_on_launch, true);
        assert_eq!(config.defaults.profile, "1080p Shrinker");
        assert_eq!(config.defaults.max_workers, 1);
        assert_eq!(config.defaults.overwrite, false);
        assert_eq!(config.defaults.last_used_profile, None);
        assert_eq!(config.defaults.auto_bit_depth, true);
        assert_eq!(config.defaults.disable_vaapi_fallback, false);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();

        // Should be able to deserialize back
        let deserialized: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(deserialized.startup.autostart, config.startup.autostart);
        assert_eq!(deserialized.defaults.profile, config.defaults.profile);
    }

    #[test]
    fn test_filename_pattern_persistence() {
        // Create config with default filename pattern
        let mut config = Config::default();
        assert_eq!(config.defaults.filename_pattern, "{basename}");

        // Set a custom pattern
        config.defaults.filename_pattern = "{basename}_encoded.{ext}".to_string();

        // Serialize to TOML
        let toml_str = toml::to_string(&config).unwrap();

        // Verify the pattern is in the TOML
        assert!(toml_str.contains("filename_pattern"));
        assert!(toml_str.contains("{basename}_encoded.{ext}"));

        // Deserialize back
        let deserialized: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(
            deserialized.defaults.filename_pattern,
            "{basename}_encoded.{ext}"
        );

        // Test with default value
        let config_with_default = Config::default();
        let toml_str2 = toml::to_string(&config_with_default).unwrap();
        let deserialized2: Config = toml::from_str(&toml_str2).unwrap();
        assert_eq!(deserialized2.defaults.filename_pattern, "{basename}");
    }
}
