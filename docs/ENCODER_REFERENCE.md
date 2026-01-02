# FFmpeg Encoder Parameter Reference

## AV1 Encoders

| Parameter        | libsvtav1             | av1_nvenc                       | av1_qsv                         | av1_vaapi                       |
|------------------|-----------------------|---------------------------------|---------------------------------|---------------------------------|
| Quality knob     | crf 1–63              | cq 0–63                         | global_quality 1–51             | global_quality 1–255            |
| Speed knob       | preset 0–13           | preset p1–p7                    | preset 1–7                      | compression_level 0–7           |
| Lookahead        | (encoder-side)        | rc-lookahead 0–32               | look_ahead_depth 0–100          | n/a                             |
| Tiling           | auto (internal)       | rows/cols -1–64                 | rows/cols 0–65535               | n/a                             |
| Film grain       | film-grain 0–50       | n/a                             | n/a                             | n/a                             |
| Bit depth        | 8/10                  | 8/10                            | 8/10                            | 8/10                            |

## VP9 Encoders

| Parameter        | libvpx-vp9            | vp9_vaapi                       | vp9_qsv                         |
|------------------|-----------------------|---------------------------------|---------------------------------|
| Quality knob     | crf 0–63              | global_quality 1–255            | global_quality 1–255            |
| Speed knob       | cpu-used 0–5          | compression_level 0–7           | preset 1–7                      |
| Lookahead        | lag-in-frames 0–25    | n/a                             | look_ahead_depth 0–120          |
| Tiling           | tile-cols/rows (log2) | n/a                             | n/a                             |
| Bit depth        | 8/10                  | 8/10                            | 8/10                            |

Note: VP9 NVENC is not available (NVIDIA removed VP9 encoding support in RTX 3000+ GPUs).
