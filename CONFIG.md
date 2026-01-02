# Configuration Guide

## Config File

**Location:**
- **Linux/macOS**: `~/.config/ffdash/config.toml`
- **Windows**: `%APPDATA%\ffdash\config.toml`

Generate a default config:
```bash
ffdash init-config
```

## Options

```toml
[startup]
# Automatically start encoding when TUI launches
autostart = false

# Scan for video files on launch (false = start with empty dashboard)
scan_on_launch = true

[defaults]
# Default encoding profile
profile = "1080p Shrinker"

# Concurrent encoding jobs (1 = sequential)
max_workers = 1
```

### Option Details

| Option | Default | Description |
|--------|---------|-------------|
| `autostart` | `false` | Start encoding immediately after scanning |
| `scan_on_launch` | `true` | Scan directory when TUI opens |
| `profile` | `"1080p Shrinker"` | Default profile (built-in or custom saved profile) |
| `max_workers` | `1` | Parallel encode jobs (higher = more CPU/RAM) |
| `filename_pattern` | `"{basename}"` | Output filename template (supports `{basename}`, `{profile}`, `{ext}`) |
| `overwrite` | `false` | Overwrite existing output files |
| `use_hardware_encoding` | `false` | Enable hardware encoding (VAAPI/QSV/NVENC) |
| `auto_bit_depth` | `true` | Auto-select pixel format from source (p010 for 10-bit, nv12 for 8-bit) |

## Additional FFmpeg Arguments

Profiles support an `additional_args` field for passing custom FFmpeg arguments that are appended to the command just before the output file. This is useful for:

- Stripping streams: `-an` (no audio), `-sn` (no subtitles)
- Adding metadata: `-metadata title="My Video"`
- Custom filters: `-af "loudnorm"`
- Any other FFmpeg options not exposed in the UI

### Usage

In the Config screen, find the "Additional FFmpeg Args" field at the bottom of the General & Audio pane. Enter arguments exactly as you would on the command line:

```
-an -sn -metadata title="Test Encode"
```

Arguments are parsed using shell-style quoting, so quoted strings with spaces are preserved.

### Profile Persistence

The `additional_args` field is saved with profiles, so you can create specialized profiles with pre-configured extra arguments.

## Auto-VMAF Settings

Auto-VMAF calibrates encoder quality to achieve a target perceptual quality score (VMAF).

### UI-Configurable Settings

These settings are available in the TUI Config screen:

| Setting | Default | Description |
|---------|---------|-------------|
| **Enable Auto-VMAF** | `false` | Checkbox to enable quality calibration |
| **Target VMAF** | `93.0` | Desired VMAF score (0-100). Higher = better quality |
| **Quality Step** | `2` | CRF/quality adjustment per calibration iteration |
| **Max Attempts** | `3` | Maximum calibration iterations before giving up |

### Advanced Settings (Config File Only)

These settings are not exposed in the TUI. Edit `~/.config/ffdash/profiles/*.toml` to customize:

| Setting | Default | Description |
|---------|---------|-------------|
| `vmaf_window_duration_sec` | `10` | Duration of each sample window in seconds |
| `vmaf_analysis_budget_sec` | `60` | Max total analysis time (sum of all windows) |
| `vmaf_n_subsample` | `30` | Frame sampling rate (30 = evaluate every 30th frame) |

### Compatibility

Auto-VMAF only works with quality-based rate control modes:

- ✅ **Software VP9/AV1**: CQ (constant quality), CQCap (CQ with max bitrate ceiling)
- ✅ **Hardware VP9/AV1**: CQP mode only
- ❌ **Not supported**: VBR, CBR bitrate modes

When enabled with an incompatible mode, Auto-VMAF is silently skipped and encoding proceeds with baseline settings.

### VMAF Score Guide

| Score Range | Quality Level | Use Case |
|-------------|---------------|----------|
| 80-90 | Good | Acceptable for most content |
| 90-93 | High | Broadcast quality, recommended default |
| 93-95 | Very High | Visually near-transparent |
| 95-100 | Excellent | Overkill for most use cases |

## Command-Line Overrides

Flags override config file settings for that session:

```bash
ffdash --autostart          # Start encoding immediately
ffdash --no-autostart       # Wait for manual start
ffdash --scan               # Scan on launch
ffdash --no-scan            # Start with empty dashboard
```

## Example Workflows

### Review before encoding (default)
```toml
[startup]
autostart = false
scan_on_launch = true
```
```bash
ffdash /path/to/videos    # Shows files, waits for you to press S
```

### Fully automated
```toml
[startup]
autostart = true
scan_on_launch = true
```
```bash
ffdash /path/to/videos    # Scans and starts encoding immediately
```

### One-off override
```bash
# Normally review first, but this batch is urgent:
ffdash --autostart /path/to/videos
```

## Tips

- **Per-directory state**: Progress is saved to `.enc_state` in each directory
- **Find your config**: `ffdash init-config` shows the path
- **Test changes**: Use `--no-scan` to launch without scanning
