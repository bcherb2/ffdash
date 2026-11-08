# Encoder Availability is decided by a probe encode, not by OS or GPU detection

Hardware encoding used to be gated on `target_os = "linux"` and on GPU detection via `lspci`, which does not exist on Windows or macOS. Opening hardware encoding to all platforms with that approach would hide AMF on Windows and offer QSV on NVIDIA-only machines (common FFmpeg builds ship `*_qsv` regardless of hardware). We decided instead that an Encoder is available only if a short probe encode (a few frames of `lavfi testsrc` to a null output, with the same rate-control mode ffdash will use) succeeds. Results are cached for the session. A separate Main10 probe decides whether 10-bit output can use that Encoder or must take the Software Fallback.

## Considered Options

- **OS-specific GPU enumeration** (WMI, `system_profiler`, `lspci`): rejected. It needs three implementations, and it only tells you a GPU exists, not that the driver, runtime, and FFmpeg build can actually encode on it.
- **Enumeration to shortlist, probe to confirm**: rejected as extra complexity. Probing every Encoder FFmpeg lists costs about a second in total.

## Consequences

- Startup (or the first time hardware is selected) pays for one probe per candidate Encoder.
- VideoToolbox on Intel Macs fails the constant-quality probe and drops to software with no Mac-specific code.
- `lspci` / `nvidia-smi` stay in use only for showing GPU stats on the dashboard, never for deciding Encoder Availability.
