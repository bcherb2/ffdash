# Auto-VMAF

Automatic quality calibration using VMAF (Video Multi-Method Assessment Fusion) scores. Instead of guessing at CRF values, Auto-VMAF encodes sample windows, measures perceptual quality, and adjusts settings to meet your target score.  FFmpeg 8 built with VMAF is required.

## How It Works

1. Encodes short sample windows from your video (start, middle, end)
2. Measures VMAF score for each window
3. Adjusts quality settings if target not met
4. Repeats until target achieved or max attempts reached
5. Encodes full video with calibrated quality (or final quality, if iterations exhausted)

## Configuration

### TUI Settings

| Setting | Default | Description |
|---------|---------|-------------|
| Enable Auto-VMAF | Off | Enable/disable calibration |
| VMAF Target | 93.0 | Target VMAF score (80-100) |
| Quality Step | 2 | Quality adjustment per iteration |
| Max Attempts | 3 | Maximum calibration iterations |

### Advanced (Profile/Config File)

| Setting | Default | Description |
|---------|---------|-------------|
| `vmaf_window_duration_sec` | 10 | Sample window length in seconds |
| `vmaf_analysis_budget_sec` | 60 | Total sampling time budget |
| `vmaf_n_subsample` | 30 | Frame stride (evaluates every Nth frame) |

## VMAF Score Guidelines

| Score | Quality | Use Case |
|-------|---------|----------|
| 80-90 | Good | Streaming, bandwidth-constrained |
| 90-93 | High | General use, balanced |
| 93-95 | Very High | Near-transparent quality |
| 95+ | Excellent | Archival (diminishing returns) |

*Note: this can be very inconsistent for highly compressed video, film grain, low-res, etc.  Please test each time you run into a new type of material to encode.*

## Requirements

- FFmpeg compiled with `libvmaf` filter support

## Temporary Files

Sample windows and VMAF logs are stored in `.ffdash_tmp/<job_id>/` under the input file's directory. These are cleaned up after calibration completes.
