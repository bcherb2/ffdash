# Dolby Vision is preserved or the job is skipped, never silently downgraded

A source with Dolby Vision is re-encoded with its Dolby Vision metadata carried into the output (HEVC or AV1). When this machine can't do that — no Encoder passes the Dolby Vision Probe Encode, the Container can't carry it, or the Dolby Vision Profile can't be converted — the job is skipped with a visible reason instead of being encoded as plain HDR10/HLG or, for profile 5, with broken colours. Preservation is always on; there is no setting to strip it. Hardware Encoders can't carry Dolby Vision, so these jobs fall to the Software Fallback, as 10-bit jobs do.

## Considered options

- **Strip Dolby Vision and keep the base layer** (what most tools do): rejected — loses the user's HDR silently, and turns profile 5 into a visibly broken file.
- **Make preservation a setting**: rejected for now — one behaviour is simpler, and stripping can be added later without changing this default.
