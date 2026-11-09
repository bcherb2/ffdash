# ffdash

A terminal dashboard for batch re-encoding video libraries with FFmpeg, choosing between software and GPU encoders.

## Language

### Encoding

**Codec**:
The compressed video format an output is written in: VP9, AV1, or HEVC.
_Avoid_: Format, encoder (when meaning the format)

**HEVC**:
The codec standardised as H.265; always called HEVC in ffdash.
_Avoid_: H.265, H265, x265 (x265 is one Encoder of HEVC)

**Encoder**:
One concrete implementation of a Codec, e.g. `libx265`, `hevc_nvenc`, `av1_qsv`.
_Avoid_: Codec (when meaning the implementation), driver

**Backend**:
Whether an Encoder runs on the CPU (Software) or a GPU (Hardware, of a given vendor).
_Avoid_: Mode, accelerator

**Software Fallback**:
The Software Encoder a Codec uses when no usable Hardware Encoder is available or the requested output cannot be produced by one.
_Avoid_: Default encoder

**Hardware API**:
The vendor or OS interface a Hardware Encoder is reached through: VAAPI, QSV, NVENC, AMF, or VideoToolbox.
_Avoid_: Driver, platform

**Encoder Availability**:
Whether a given Encoder can actually be used on this machine, judged by what FFmpeg offers and which GPUs are present, never by operating system alone.
_Avoid_: Platform support

**Probe Encode**:
A few-frame test encode with a candidate Encoder whose success establishes Encoder Availability.
_Avoid_: Smoke test (that is the separate end-to-end profile check), capability check

**Encode Plan**:
What a single encode will run with: the selected Encoder and, optionally, a Quality Override.
_Avoid_: Hardware config, hw config

**Quality Override**:
A quality value that replaces the active Encoder's configured quality for one encode, as set by Auto-VMAF.
_Avoid_: Global quality, CQ override

### Output


**Container**:
The file format wrapping the encoded streams: WebM, MKV, or MP4.
_Avoid_: Format, extension

**Main10**:
10-bit HEVC output, required for HDR10.
_Avoid_: 10-bit mode, HDR mode

**Dolby Vision**:
An HDR format, independent of Codec, made of a base layer plus dynamic metadata.
_Avoid_: DV, DoVi (aliases only), RPU (in user-facing text)

**Dolby Vision Profile**:
The variant of Dolby Vision a stream uses (e.g. 5, 7, 8.1, 8.4, 10), which decides whether its base layer is usable without the dynamic metadata.
_Avoid_: DV mode, DV type

**Dolby Vision Conversion**:
Rewriting a source's Dolby Vision to another Dolby Vision Profile before encoding, e.g. 7 to 8.1.
_Avoid_: DV fix, RPU conversion (in user-facing text)
