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

| Parameter        | libx265                    |
|------------------|----------------------------|
| Quality knob     | crf 0–51 (default 28)      |
| Speed knob       | preset ultrafast–veryslow  |
| Tuning           | tune none/psnr/ssim/grain/zerolatency/fastdecode/animation |
| Bit depth        | 8/10 (10 = Main10)         |
| HDR10            | colour flags + `-x265-params hdr10=1:hdr10-opt=1:repeat-headers=1`; mastering display and content light level carried from the source |
| MP4              | always `-tag:v hvc1`       |
| Container        | MKV default, MP4 selectable, WebM not allowed |
| Auto-VMAF        | CRF searched downward from the configured value to 10 |

The hardware HEVC Encoders (`hevc_nvenc`, `hevc_qsv`, `hevc_vaapi`, `hevc_amf`, `hevc_videotoolbox`) get a column each as they are added.

## VP9 Encoders

| Parameter        | libvpx-vp9            | vp9_vaapi                       | vp9_qsv                         |
|------------------|-----------------------|---------------------------------|---------------------------------|
| Quality knob     | crf 0–63              | global_quality 1–255            | global_quality 1–255            |
| Speed knob       | cpu-used 0–5          | compression_level 0–7           | preset 1–7                      |
| Lookahead        | lag-in-frames 0–25    | n/a                             | look_ahead_depth 0–120          |
| Tiling           | tile-cols/rows (log2) | n/a                             | n/a                             |
| Bit depth        | 8/10                  | 8/10                            | 8/10                            |

Note: VP9 NVENC is not available (NVIDIA removed VP9 encoding support in RTX 3000+ GPUs).
