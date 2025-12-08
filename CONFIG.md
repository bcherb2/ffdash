# Configuration Guide

## Config File

**Location:** `~/.config/ffdash/config.toml`

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
profile = "vp9-good"

# Concurrent encoding jobs (1 = sequential)
max_workers = 1
```

### Option Details

| Option | Default | Description |
|--------|---------|-------------|
| `autostart` | `false` | Start encoding immediately after scanning |
| `scan_on_launch` | `true` | Scan directory when TUI opens |
| `profile` | `"vp9-good"` | Default profile (`"vp9-good"`, `"YouTube 4K"`, or custom) |
| `max_workers` | `1` | Parallel encode jobs (higher = more CPU/RAM) |

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
