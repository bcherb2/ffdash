# Hardware encoder settings live in each codec's config; there is no shared hardware config

`HwEncodingConfig` looked generic but held VP9-on-VAAPI settings (loop filter, compression level). It was also overloaded: whether it was present meant "use hardware", and it carried the Auto-VMAF quality override. AV1 hardware encoders ignored it entirely. We removed it. Every Encoder's settings now live in its codec's config (`Vp9Config`, `Av1Config`, `HevcConfig`). The command builder, worker, and Auto-VMAF receive a codec-neutral **Encode Plan** (the selected Encoder plus an optional Quality Override) and nothing else. The Backend comes from the Encoder, not from whether a config object exists.

## Considered Options

- **A shared struct of common hardware settings (rate control, quality, b-frames, lookahead) plus per-codec extras**: rejected. The settings share names but differ in range and meaning for every Encoder (e.g. QSV `global_quality` is 1–255, NVENC `cq` is 0–63, VideoToolbox `q:v` is 1–100), so a shared struct would only hide those differences. Please don't reintroduce one.

## Consequences

- A clean break for saved profiles: the old top-level `hw_*` fields are not migrated. Profiles that used them load with defaults for the VP9 VAAPI settings.
- The Quality Override is applied to the active Encoder's own quality field, which fixes AV1 Auto-VMAF writing to the legacy `hw_cq` field.
