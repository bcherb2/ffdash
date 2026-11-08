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
| `use_hardware_encoding` | `false` | Enable hardware encoding (VAAPI/QSV/NVENC/AMF) on Linux, Windows and macOS, when an Encoder for the selected codec passes its Probe Encode |
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

### Upgrade note: VP9 VAAPI hardware settings reset

Saved profiles still load, but the old top-level `hw_*` fields (`hw_rc_mode`, `hw_global_quality`, `hw_b_frames`, `hw_loop_filter_level`, `hw_loop_filter_sharpness`, `hw_compression_level`) are now ignored. The VP9 VAAPI settings live in the VP9 codec config, so on profiles saved before this change they reset to their defaults. Re-apply any custom values in the config panel and save the profile again.

## HEVC

Pick **HEVC** in the Video Codec dropdown (or the built-in `hevc-x265` profile) to encode with `libx265`.

- **Quality**: x265 CRF, 0-51 (default 28), set with the CRF slider. HEVC hardware Encoders keep their own quality values, each on its own scale.
- **Intel Quick Sync (`hevc_qsv`, built-in profile `hevc-qsv`)**: Linux and Windows. Quality is `-q:v` (CQP, 1-51, default 26); Preset 1-7, Look-ahead depth (0 = off) and B-frames apply too. Used when its Probe Encode passes.
- **AMD AMF** (`hevc-amf`): QP 0-51 (default 26, lower is better) and the hardware preset (1-2 quality, 3-5 balanced, 6-7 speed). AMF has no B-frames or lookahead here.
- **Apple VideoToolbox** (`hevc-videotoolbox`): quality 1-100 (default 60, **higher is better**). Intel Macs reject constant quality and fall back to `libx265`.
- Both are fed from CPU decode on every host and are used only where their Probe Encode passes.
- **Preset / Tune**: x265 preset (`ultrafast` to `veryslow`, default `medium`) and optional tune. These controls apply to `libx265` only and are greyed out when a hardware Encoder is selected.
- **NVIDIA NVENC** (`hevc_nvenc`, built-in profile `hevc-nvenc`): with hardware encoding on and an NVIDIA GPU, the panel shows NVENC Preset (1 = best quality, 7 = fastest), NVENC CQ (0-51, default 28, lower = better), Lookahead (0-32) and B-Frames (0-4). It is used only when its Probe Encode passes.
- **Main10 / HDR10**: choose a 10-bit Bit Depth. With an HDR10 colour space the colour primaries, transfer and matrix are written to the stream, and mastering display / content light level are carried over from the source.
- **VAAPI (Linux)**: the built-in `hevc-vaapi` profile uses `hevc_vaapi` with constant QP (1-51, default 26) and B-frames (0-4, default 2). There is no preset or lookahead for it.
- **Container**: MKV is the default and MP4 is selectable (always tagged `hvc1` so QuickTime and Apple players open it). WebM cannot hold HEVC and is not selectable.

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

- ✅ **Software VP9/AV1/HEVC**: CQ (constant quality), CQCap (CQ with max bitrate ceiling)
- ✅ **Hardware VP9/AV1/HEVC**: constant-quality modes only (CQP)
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
