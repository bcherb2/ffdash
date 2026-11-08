# FFmpeg Encoder Parameter Reference

Hardware Encoders are available on Linux (VAAPI, QSV, NVENC), Windows (NVENC, QSV, AMF) and macOS. Whether one is usable is decided at runtime by a short Probe Encode, not by operating system. The tables below list parameters per Encoder.

## AV1 Encoders

| Parameter        | libsvtav1             | av1_nvenc                       | av1_qsv                         | av1_vaapi                       |
|------------------|-----------------------|---------------------------------|---------------------------------|---------------------------------|
| Quality knob     | crf 1–63              | cq 0–63                         | global_quality 1–51             | global_quality 1–255            |
| Speed knob       | preset 0–13           | preset p1–p7                    | preset 1–7                      | compression_level 0–7           |
| Lookahead        | (encoder-side)        | rc-lookahead 0–32               | look_ahead_depth 0–100          | n/a                             |
| Tiling           | auto (internal)       | rows/cols -1–64                 | rows/cols 0–65535               | n/a                             |
| Film grain       | film-grain 0–50       | n/a                             | n/a                             | n/a                             |
| Bit depth        | 8/10                  | 8/10                            | 8/10                            | 8/10                            |

## HEVC Encoders

Called "HEVC" everywhere (H.265 is an alias). Each Encoder has its own quality field on its own scale; Auto-VMAF replaces only the active Encoder's field (the Quality Override).

| Parameter        | libx265                    | hevc_qsv                         |
|------------------|----------------------------|----------------------------------|
| Quality knob     | crf 0–51 (default 28)      | `-q:v` CQP 1–51 (default 26)     |
| Speed knob       | preset ultrafast–veryslow  | preset 1–7 (sent as veryslow–veryfast) |
| Lookahead        | (encoder-side)             | look_ahead_depth 0–100 (0 = off) |
| B-frames         | (encoder-side)             | bf 0–16 (default 2)              |
| Tuning           | tune none/psnr/ssim/grain/zerolatency/fastdecode/animation | n/a |
| Bit depth        | 8/10 (10 = Main10)         | 8/10 (10 = p010 + `-profile:v main10`) |
| HDR10            | colour flags + `-x265-params hdr10=1:hdr10-opt=1:repeat-headers=1`; mastering display and content light level carried from the source | colour metadata through `vpp_qsv` options (see [color metadata](#color-metadata)) |
| Decoding         | CPU                        | Linux: QSV hardware decode on the VAAPI-derived device; Windows: CPU decode, frames uploaded with `hwupload` |
| Platforms        | all                        | Linux and Windows                |
| MP4              | always `-tag:v hvc1`       | always `-tag:v hvc1`             |
| Container        | MKV default, MP4 selectable, WebM not allowed | same |
| Auto-VMAF        | CRF searched downward from the configured value to 10 | QP searched downward from the configured value to the hardware floor (5) |

`hevc_qsv` is only used when its Probe Encode passes. On Linux a failed QSV initialisation retries with `hevc_vaapi`, like the other QSV Encoders. The built-in profile is `hevc-qsv`.

### hevc_nvenc (NVIDIA NVENC, Linux and Windows)

| Parameter        | hevc_nvenc                 |
|------------------|----------------------------|
| Quality knob     | `-rc vbr -cq 0–51 -b:v 0` (default 28, lower = better; `-b:v 0` removes the bitrate cap so `-cq` alone decides quality) |
| Speed knob       | `-preset p1`–`p7` (p1 fastest, p7 best); the shared hardware preset 1–7 is inverted onto it (default 4 = p4) |
| Lookahead        | `-rc-lookahead` 0–32 (0 omits the flag) |
| B-frames         | `-bf` 0–4 (default 2) |
| Bit depth        | 8/10: 10-bit adds `-pix_fmt p010le -profile:v main10` |
| HDR10            | colour flags (`-colorspace`, `-color_primaries`, `-color_trc`, `-color_range`); mastering display and content light level carried from the source |
| MP4              | always `-tag:v hvc1`       |
| Decoding         | Linux: `-hwaccel cuda`; Windows: CPU decode |
| Availability     | Probe Encode (`-rc vbr -cq 30 -b:v 0`, plus a Main10 probe) |
| Auto-VMAF        | CQ searched downward from the configured value to 5 |

The other hardware HEVC Encoders (`hevc_vaapi`, `hevc_amf`, `hevc_videotoolbox`) get a section each as they are added.

### hevc_amf and hevc_videotoolbox

Both are fed from CPU decode on every host (no `-hwaccel`, no device init) and are used only where their constant-quality Probe Encode passes. Built-in profiles: `hevc-amf`, `hevc-videotoolbox`.

| Parameter        | hevc_amf (AMD, Windows/Linux)          | hevc_videotoolbox (macOS)              |
|------------------|----------------------------------------|----------------------------------------|
| Quality knob     | `-rc cqp -qp_i/-qp_p` 0–51 (default 26, lower=better) | `-q:v` 1–100 (default 60, **higher=better**) |
| Speed knob       | `-quality` speed/balanced/quality from the hardware preset (1–2 quality, 3–5 balanced, 6–7 speed) | n/a |
| Lookahead        | n/a                                    | n/a                                    |
| B-frames         | n/a                                    | n/a                                    |
| Bit depth        | 8/10 (`p010le`, Main10)                | 8/10 (`p010le`, Main10)                |
| MP4              | always `-tag:v hvc1`                   | always `-tag:v hvc1`                   |
| Auto-VMAF        | QP searched downward to the hardware floor | `q:v` searched upward to 100       |
| Probe Encode     | `-rc cqp -qp_i 30 -qp_p 30`            | `-q:v 30` (Intel Macs fail it and fall back to libx265) |

### hevc_vaapi (VAAPI, Linux)

| Parameter        | hevc_vaapi |
|------------------|------------|
| Quality knob     | QP 1-51 via `-global_quality:v` (default 26, lower = better), rate control always `-rc_mode:v CQP` |
| Speed knob       | none (no preset) |
| Lookahead        | none |
| B-frames         | `-bf:v` 0-4 (default 2); driver support varies |
| Bit depth        | 8/10: `format=nv12` + `-profile:v main`, or `format=p010` + `-profile:v main10` (10-bit explicit, or `auto` on a 10-bit source) |
| Decode           | CPU decode, then `hwupload`; no `-hwaccel` |
| Probe Encode     | `-init_hw_device vaapi=va:<render node>`, 5 frames, CQP at QP 30; Main10 probed separately |
| MP4              | always `-tag:v hvc1` |
| Auto-VMAF        | QP searched downward from the configured value to 5 |
| Built-in profile | `hevc-vaapi` |

The hevc_vaapi preset and lookahead settings (`hw_preset`, `hw_lookahead`) are not used; the config panel shows only Quality (QP) and B-Frames for it.

## VP9 Encoders

| Parameter        | libvpx-vp9            | vp9_vaapi                       | vp9_qsv                         |
|------------------|-----------------------|---------------------------------|---------------------------------|
| Quality knob     | crf 0–63              | global_quality 1–255            | global_quality 1–255            |
| Speed knob       | cpu-used 0–5          | compression_level 0–7           | preset 1–7                      |
| Lookahead        | lag-in-frames 0–25    | n/a                             | look_ahead_depth 0–120          |
| Tiling           | tile-cols/rows (log2) | n/a                             | n/a                             |
| Bit depth        | 8/10                  | 8/10                            | 8/10                            |

Note: VP9 NVENC is not available (NVIDIA removed VP9 encoding support in RTX 3000+ GPUs).
