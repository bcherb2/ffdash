# ffdash

A terminal UI for batch VP9 video encoding with hardware acceleration, real-time progress monitoring, and full control over quality settings.  Made to work as a viable encoding dashboard over SSH.

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS-lightgrey.svg)

## Why ffdash?

- **Batch processing** - Encode entire directories of video files with a single command
- **Hardware acceleration** - VA-API (Intel/AMD) and NVENC (NVIDIA) support for 3-10x faster encoding
- **Live dashboard** - Real-time ETA, throughput, queue progress, and system stats
- **Full tunability** - Rate control modes, quality presets, filters, GOP settings, audio options
- **Keyboard-first** - Fast navigation, built-in help (`H`), SSH-friendly, TUI mouse support
- **Dry-run preview** - See exact FFmpeg commands before committing to an encode

### Why VP9?

VP9 delivers 20-50% smaller files than H.264 at equivalent quality. It's open-source, royalty-free, and natively supported by YouTube, all modern browsers, and media servers like Plex and Jellyfin.  Considering support for AV1 in the future.

## Prerequisites

### Required

**Rust 1.85+** (edition 2024):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
rustc --version  # Should show 1.85.0 or higher
```

**FFmpeg** with VP9 support (tested with v8.0):
```bash
# Ubuntu/Debian
sudo apt update && sudo apt install ffmpeg

# macOS
brew install ffmpeg
```

**Or install both automatically:**
```bash
make deps
```

### Optional (Hardware Encoding)

| Platform | Requirements |
|----------|-------------|
| **Intel VA-API** | Linux, `/dev/dri` device mapped, Intel HD 5000+ or Arc GPU |
| **AMD VA-API** | Linux, `/dev/dri` device mapped, Mesa VAAPI drivers |
| **NVIDIA NVENC** | Linux, CUDA drivers, GTX 600+ or newer |

Verify hardware support:
```bash
ffdash check-vaapi      # Intel/AMD
ffmpeg -encoders | grep nvenc  # NVIDIA
```

## Installation

### Build from source

```bash
git clone https://github.com/bcherb2/ffdash.git
cd ffdash
make release
sudo make install   # Installs to /usr/local/bin
```

### Verify installation

```bash
ffdash check-ffmpeg   # Verify FFmpeg is configured correctly
ffdash --help         # Show all commands
```

## Quick Start

### Basic usage

```bash
# Launch TUI and scan current directory
ffdash

# Launch TUI and scan a specific directory
ffdash /path/to/videos

# Preview FFmpeg commands without encoding (dry run)
# this will use the config that is loaded on app startup
ffdash dry-run /path/to/video.mp4
```

### TUI navigation

| Key | Action |
|-----|--------|
| `S` | Start encoding |
| `SPACE` | Mark pending / skipped |
| `C` | Open config |
| `H` | Show help |
| `T` | Toggle stats view |
| `R` | Rescan directory |
| `Q` | Quit (progress is saved) |
| `↑↓` / `Tab` | Navigate |

### Configuration

Generate a config file to customize defaults:
```bash
ffdash init-config
# Creates ~/.config/ffdash/config.toml
```

Example configuration:
```toml
[startup]
autostart = false      # Wait for manual start
scan_on_launch = true  # Scan directory on launch

[defaults]
profile = "vp9-good"   # Default encoding profile
max_workers = 1        # Concurrent encode jobs
```

See [CONFIG.md](CONFIG.md) for all options.

## Command Reference

### TUI Mode (default)

```bash
ffdash [OPTIONS] [DIRECTORY]

Options:
  --autostart       Start encoding immediately after scan
  --no-autostart    Wait for manual start (overrides config)
  --scan            Scan directory on launch (overrides config)
  --no-scan         Start with empty dashboard
```

### Utility Commands

```bash
ffdash check-ffmpeg      # Verify FFmpeg installation
ffdash check-vaapi       # Test VA-API hardware encoding support
ffdash init-config       # Create/show config file location
ffdash probe FILE        # Get video duration and metadata
ffdash scan DIR          # List detected video files
ffdash dry-run FILE|DIR  # Preview FFmpeg commands
ffdash encode-one DIR    # Encode only the first pending file
```

## Docker

Build the image (requires local binary first):
```bash
make docker-build
```

### Run with Intel/AMD VA-API

```bash
docker run -it --rm \
  --device /dev/dri:/dev/dri \
  -v /path/to/videos:/videos \
  ffdash:latest ffdash /videos
```

### Run with NVIDIA NVENC

```bash
docker run -it --rm \
  --gpus all \
  -e NVIDIA_DRIVER_CAPABILITIES=compute,utility,video \
  -v /path/to/videos:/videos \
  ffdash:latest ffdash /videos
```

### SSH access (optional)

Add `-p 2223:22 -e SSH_PASSWORD=yourpassword` to enable SSH into the container.

## FAQ

**How do I verify hardware encoding is active?**
```bash
ffdash dry-run /path/to/video.mp4
# Look for: -c:v vp9_vaapi (Intel/AMD) or -c:v vp9_nvenc (NVIDIA)
# If you see: -c:v libvpx-vp9 → software encoding is being used
```

**What quality settings should I use?**
- **CQ (Constant Quality)**: Start with CQ 28-32 for good balance
- **VBR**: 2-4 Mbps for 1080p, 6-10 Mbps for 4K
- **Hardware encoding**: Start with quality 100-140 and adjust lower for better quality (lower = better)

**Can I pause and resume encoding?**

Yes. Press `Q` to quit - progress is saved to `.enc_state` in each directory. Run `ffdash` again to resume where you left off.
Note: if FFmpeg is still encoding, it will finish in the background and exit.

**Why aren't my video files showing up?**

Supported formats: `.mp4`, `.mkv`, `.avi`, `.mov`, `.webm`, `.flv`, `.m4v`

Press `R` to rescan the directory after adding files.

**How much faster is hardware encoding?**

Typically 3-10x faster than software encoding. Quality may be slightly lower at equivalent bitrates, so consider increasing bitrate by ~20% when using hardware encoding.

**Getting errors on 5+ channel audio**
Likely need to increase the audio bitrate, or force downmixing into stereo.

**The TUI looks broken over SSH**

Ensure your terminal supports 256 colors and your `TERM` variable is set correctly:
```bash
export TERM=xterm-256color
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| "No hardware devices detected" | Verify `/dev/dri` exists (VA-API) or `--gpus all` is set (NVENC) |
| Encoding fails immediately | Run `ffdash dry-run` and test the command manually with `ffmpeg` |
| Wrong encoder selected | Check hardware toggle in Config screen (`C`) |
| Slow performance | Increase workers in config, or enable hardware encoding |

Validate FFmpeg capabilities:
```bash
ffmpeg -h encoder=vp9_vaapi     # VA-API options
ffmpeg -h encoder=libvpx-vp9    # Software VP9 options
ffmpeg -h encoder=vp9_nvenc     # NVENC options (if available)
```

## Known Limitations

- Hardware encoding (VA-API/NVENC) requires Linux; macOS uses software encoding only
- AMD VA-API support depends on Mesa driver version and GPU generation
- NVENC behavior varies by driver version and GPU architecture

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Run `make check-ui` and `cargo test` before submitting
4. Open a pull request

## License

MIT. See [LICENSE](LICENSE).
