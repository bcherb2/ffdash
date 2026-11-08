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

## VP9 Encoders

| Parameter        | libvpx-vp9            | vp9_vaapi                       | vp9_qsv                         |
|------------------|-----------------------|---------------------------------|---------------------------------|
| Quality knob     | crf 0–63              | global_quality 1–255            | global_quality 1–255            |
| Speed knob       | cpu-used 0–5          | compression_level 0–7           | preset 1–7                      |
| Lookahead        | lag-in-frames 0–25    | n/a                             | look_ahead_depth 0–120          |
| Tiling           | tile-cols/rows (log2) | n/a                             | n/a                             |
| Bit depth        | 8/10                  | 8/10                            | 8/10                            |

Note: VP9 NVENC is not available (NVIDIA removed VP9 encoding support in RTX 3000+ GPUs).
