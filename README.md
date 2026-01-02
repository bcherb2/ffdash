# ffdash

A terminal UI for batch VP9/AV1 video encoding with hardware acceleration, real-time progress monitoring, and full control over quality settings. Made to work as a dashboard over SSH.

https://github.com/user-attachments/assets/f7551d5f-fe6c-4a13-81e2-92a5e2bb42c2

[![GitHub Release](https://img.shields.io/github/v/release/bcherb2/ffdash)](https://github.com/bcherb2/ffdash/releases)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey.svg)

## Why ffdash?

- **Batch processing** - Encode entire directories with parallel workers, adjustable on the fly
- **Hardware acceleration** - QSV, VA-API (Intel/AMD) and NVENC (NVIDIA >= Ada Lovelace) for fast encodes
- **Auto-VMAF calibration** - Automatically tune quality to hit a target VMAF score
- **Encoding profiles** - Save and load encoding presets for different use cases
- **Live dashboard** - Real-time ETA, throughput, queue progress, and system stats
- **Full tunability** - Rate control modes, quality presets, filters, GOP settings, audio options
- **Keyboard-first** - Fast navigation, built-in help, SSH-friendly, mouse support
- **Dry-run preview** - See exact FFmpeg commands before encoding

![stats](https://github.com/user-attachments/assets/44fa1a89-e541-4557-9dc0-81463c5567e5)

### Why VP9/AV1?

**VP9** delivers 20-50% smaller files than H.264 at equivalent quality. **AV1** is the next-generation codec offering slightly better compression than VP9. Both are open-source, royalty-free, and natively supported by YouTube, all modern browsers, and media servers like Plex and Jellyfin.

Typical file sizes:
- **1080p TV episode**: ~200-400 MB
- **4K HDR Blu-ray film**: ~10-15 GB (higher for grain-heavy content)

## Prerequisites

### Required

**Rust 1.85+** (edition 2024):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
rustc --version  # Should show 1.85.0 or higher
```

**FFmpeg** with VP9/AV1 support (tested with v8.0):
```bash
# Ubuntu/Debian
sudo apt update && sudo apt install ffmpeg

# macOS (or Linux with brew)
brew install ffmpeg
```

**Or install both automatically:**
```bash
make deps
```

### Optional (Hardware Encoding)

| Platform | Requirements |
|----------|-------------|
| **Intel QSV** | Linux, `/dev/dri` device mapped, Intel HD 5000+ or Arc GPU |
| **Intel/AMD VA-API** | Linux, `/dev/dri` device mapped, Mesa VAAPI drivers |
| **NVIDIA NVENC** | Linux, CUDA drivers, Ada Lovelace+ (RTX 4000 series or newer) |

Verify hardware support:
```bash
ffdash check-vaapi                    # Intel/AMD VA-API
ffmpeg -encoders | grep -E "vp9|av1"  # List available VP9/AV1 encoders
```

## Installation

### Pre-built binaries

Download from [GitHub Releases](https://github.com/bcherb2/ffdash/releases):
- **Linux**: `ffdash-x86_64-unknown-linux-gnu.tar.gz`
- **macOS (Intel)**: `ffdash-x86_64-apple-darwin.tar.gz`
- **macOS (Apple Silicon)**: `ffdash-aarch64-apple-darwin.tar.gz`
- **Windows**: `ffdash-x86_64-pc-windows-msvc.zip`

### Package managers

```bash
# Arch Linux (AUR)
yay -S ffdash-bin

# macOS or Linux (Homebrew)
brew tap bcherb2/ffdash
brew install ffdash
```

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

# Preview FFmpeg commands without encoding
ffdash dry-run /path/to/video.mp4
```

### TUI navigation

| Key | Action |
|-----|--------|
| `S` | Start encoding |
| `SPACE` | Toggle pending / skipped |
| `C` | Open config |
| `H` | Show help |
| `T` | Toggle stats view |
| `R` | Rescan directory |
| `Q` | Quit (confirms if encoding) |
| `X` | Clear state and exit |
| `↑↓` | Navigate queue |
| `Tab` | Cycle active jobs |
| `[`/`]` | Adjust workers |

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
profile = "1080p Shrinker"      # Default encoding profile
max_workers = 1                 # Concurrent encode jobs
```

See [CONFIG.md](CONFIG.md) for all options.

## Auto-VMAF Quality Calibration

Automatically calibrate encoder quality to achieve a target VMAF perceptual quality score.

**How it works:**
1. Before encoding, analyze 1-5 short sample windows (~10s each) from your video
2. Measure VMAF quality and adjust encoder settings iteratively
3. Encode the full file once quality meets your target

**Configuration** (in TUI Config screen):
- **Enable Auto-VMAF**: Checkbox to enable calibration
- **Target VMAF**: Desired quality score (default: 93.0, range: 0-100)
  - 80+ = broadcast quality
  - 90+ = high quality (recommended)
  - 95+ = near-transparent
- **Quality Step**: CRF/quality adjustment per iteration (default: 2)
- **Max Attempts**: Maximum calibration iterations (default: 3)

**Supported modes:**
- ✅ Software: CQ (constant quality), CQCap (CQ + max bitrate)
- ✅ Hardware: CQP mode only
- ❌ VBR/CBR modes not supported

**Performance:** Adds ~10% overhead via sparse sampling (60s total analysis budget, evaluates 1 frame/sec).

**Status display:** Shows "Calibrating" during analysis, displays achieved VMAF in results column.

Please be familiar with VMAF and how it works.  There can be some differences between scores of older or highly compressed / low-res footage - I assume this is because of the objectively low quality being used as a reference.

See [docs/AUTO_VMAF.md](docs/AUTO_VMAF.md) for detailed algorithm and advanced settings.

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

**I changed settings but they didn't apply to my encode?**

If a queue is already built, you need to rescan (`R`) to apply new settings. This is similar to how Handbrake's queue works.

**How do I verify hardware encoding is active?**
- Help screen shows what hardware acceleration is available
- Logs are written to directory where `ffdash` is launched 
- Use tools like `nvidia-smi` or `intel_gpu_top` to verify utilization
- It should be *very* obvious - CPU encoding will take HOURS for most films


**What quality settings should I use?**

I almost always use AV1 with QSV encoder on slowest (1) with a quality of between 80 and 140, depending on the resolution, quality, etc.

**Can I pause and resume encoding?**

Yes. Press `Q` to quit - if encodes are running, you'll be asked to confirm. Progress is saved to `.enc_state` in each directory. Run `ffdash` again to resume where you left off.

**Why aren't my video files showing up?**

Supported formats: `.mp4`, `.mkv`, `.avi`, `.mov`, `.webm`, `.flv`, `.m4v`

Press `R` to rescan the directory after adding files.

**Why am I having issues encoding this Blu-ray?**

- Use mkv filetypes for Blu-rays
- In my experience most issues are either audio related, or color depth.  Start in passthru and change settings from there.
- Logs will pass through ffmpeg errors

**How much faster is hardware encoding?**

Way faster than software encoding. Quality may be slightly lower at equivalent bitrates, so consider increasing bitrate by ~20% when using hardware encoding.
In rare cases, you may want to use software encoding for AV1.  You may want to do this for maximum compression, or to use the software-only film-grain options.

**Getting errors on 5+ channel audio?**

Likely need to increase the audio bitrate, or force downmixing into stereo.  Passthru may also be your best option if using something like TrueHD audio.

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
| Slow performance | Press `]` to add workers, or enable hardware encoding |

Validate FFmpeg capabilities:
```bash
# VP9 encoders
ffmpeg -h encoder=libvpx-vp9    # Software VP9
ffmpeg -h encoder=vp9_vaapi     # VA-API VP9

# AV1 encoders
ffmpeg -h encoder=libsvtav1     # Software AV1 (SVT-AV1)
ffmpeg -h encoder=av1_qsv       # Intel Quick Sync AV1
ffmpeg -h encoder=av1_nvenc     # NVIDIA NVENC AV1
ffmpeg -h encoder=av1_vaapi     # VA-API AV1
```



#### Encoding
- [x] VP9 software encoding (libvpx-vp9)
- [x] AV1 software encoding (SVT-AV1)
- [x] Intel QSV hardware encoding (VP9, AV1)
- [x] Intel/AMD VA-API hardware encoding (VP9, AV1)
- [x] NVIDIA NVENC hardware encoding (AV1)
- [x] Auto-VMAF quality calibration
- [x] Two-pass encoding
- [x] HDR passthrough and tonemapping (uses FFmpeg Hable)
- [ ] AMD AMF encoding (not tested)

#### Audio
- [x] Audio passthrough (copy)
- [x] Opus, AAC, AC3, Vorbis encoding
- [x] Stereo downmix option
- [x] Secondary AC3 track for compatibility

#### Interface
- [x] Real-time progress dashboard
- [x] Parallel encoding workers
- [x] Save/load encoding profiles
- [x] Dry-run command preview
- [x] Built-in help system
- [x] Mouse support
- [ ] Notification webhooks
- [ ] Web TUI

#### Workflow
- [x] Batch directory scanning
- [x] Resume interrupted encodes
- [x] Skip/unskip files in queue
- [x] Per-directory state persistence
- [ ] Watch mode (auto-encode new files)
- [ ] Post-encode scripts/hooks
- [ ] Better way to build queue




## Known Limitations

- Hardware encoding (QSV/VA-API/NVENC) requires Linux; Win/macOS are untested
- AMD VA-API support depends on Mesa driver version and GPU generation (AMD is untested)
- NVENC behavior varies by driver version and GPU architecture
- Blu-ray MKVs with complex stream layouts may show inaccurate ETA/progress (file size is accurate)

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Run `cargo test` before submitting
4. Open a pull request

## License

MIT. See [LICENSE](LICENSE).
