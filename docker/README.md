# Docker Deployment for ffdash (runtime)

Slim runtime container with SSH (password-only) and VA-API/NVENC friendly defaults.

## Build & Run

1) Build the binary locally:
```bash
cargo build --release
```

2) Build the image:
```bash
make docker-build
```

3) Run with Intel/AMD (VA-API):
```bash
docker run -it --rm \
  --device /dev/dri:/dev/dri \
  -v /mnt/user/media:/videos \
  -p 2223:22 \
  ghcr.io/YOUR_USERNAME/ffdash:latest
```

4) Run with NVIDIA:
```bash
docker run -it --rm \
  --gpus all \
  -e NVIDIA_DRIVER_CAPABILITIES=compute,utility,video \
  -v /mnt/user/media:/videos \
  -p 2223:22 \
  ghcr.io/YOUR_USERNAME/ffdash:latest
```

SSH login: `root` / password `${SSH_PASSWORD:-docker}`

Default command: `ffdash /videos` (override by passing your own command, e.g., `/bin/bash`).

## Notes
- No public key fetch; password only (set `SSH_PASSWORD`).
- Image is runtime-only (no dev toolchain). Build the binary on the host before `docker build`.
- For Unraid, map `/dev/dri` and `/mnt/user/media` similarly; set port mapping if you need SSH access.
