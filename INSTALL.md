# Installation Guide

This guide covers installing ffdash on Ubuntu and macOS.

## Prerequisites

- **Operating System**: Ubuntu 20.04+ or macOS 10.15+
- **Internet connection** for downloading dependencies

## Quick Install

### Option 1: Using Make (Recommended)

The Makefile handles all dependencies and builds automatically.

```bash
# Install all dependencies (Rust + FFmpeg)
make deps

# Build and install
make release
sudo make install

# Verify installation
ffdash --help
```

### Option 2: Manual Installation

#### 1. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

#### 2. Install FFmpeg

**Ubuntu:**
```bash
sudo apt-get update
sudo apt-get install -y ffmpeg
```

**macOS:**
```bash
brew install ffmpeg
```

#### 3. Build ffdash

```bash
cargo build --release
```

#### 4. Install (Optional)

```bash
# Install to /usr/local/bin
sudo install -m 755 target/release/ffdash /usr/local/bin/ffdash

# Or add to PATH
export PATH="$PATH:$(pwd)/target/release"
```

## Makefile Targets

Run `make help` to see all available targets:

### Build Targets
- `make build` or `make debug` - Build debug version
- `make release` - Build optimized release version
- `make install` - Install to `/usr/local/bin` (requires sudo)
- `make uninstall` - Remove from system

### Development Targets
- `make test` - Run test suite
- `make check` - Check code without building
- `make fmt` - Format code with rustfmt
- `make clippy` - Lint code with clippy
- `make clean` - Remove build artifacts

### Dependency Targets
- `make deps` - Install all dependencies
- `make install-rust` - Install Rust toolchain only
- `make install-ffmpeg` - Install FFmpeg only

### Other Targets
- `make version` - Show version info
- `make docs` - Build and open documentation
- `make update` - Update Rust dependencies

## Platform-Specific Notes

### Ubuntu

The Makefile uses `apt-get` to install dependencies. You may need sudo privileges:

```bash
# Install dependencies
make deps  # Will prompt for sudo password

# Build
make release

# Install
sudo make install
```

### macOS

The Makefile uses Homebrew. Install Homebrew first if needed:

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

Then proceed with installation:

```bash
# Install dependencies
make deps

# Build
make release

# Install
sudo make install
```

## Verifying Installation

After installation, verify ffdash is working:

```bash
# Check version
ffdash --help

# Check FFmpeg is accessible
ffmpeg -version
ffprobe -version
```

## Custom Installation Prefix

To install to a custom location:

```bash
make release
sudo make install PREFIX=/opt
# Installs to /opt/bin/ffdash
```

Or for user-only installation:

```bash
make release
make install PREFIX=$HOME/.local
# Installs to ~/.local/bin/ffdash
# Add ~/.local/bin to your PATH
```

## Troubleshooting

### Git authentication errors with private dependencies

If you see errors like:
```
failed to authenticate when downloading repository
no authentication methods succeeded
```

**Solution 1: Automatic (Recommended)**

The project is pre-configured to use system git CLI. Just ensure your SSH keys are set up:

```bash
# Test SSH access to GitHub
ssh -T git@github.com

# If that fails, set up SSH keys:
ssh-keygen -t ed25519 -C "your_email@example.com"
eval "$(ssh-agent -s)"
ssh-add ~/.ssh/id_ed25519

# Add the public key to GitHub:
cat ~/.ssh/id_ed25519.pub
# Copy and add to https://github.com/settings/keys
```

**Solution 2: Use Make with git CLI flag**

```bash
USE_GIT_CLI=1 make release
```

**Solution 3: Use environment variable**

```bash
export CARGO_NET_GIT_FETCH_WITH_CLI=true
cargo build --release
```

**Solution 4: Use HTTPS instead of SSH**

Edit your `~/.gitconfig`:
```toml
[url "https://github.com/"]
    insteadOf = git@github.com:
```

### Rust not found after installation

Run:
```bash
source $HOME/.cargo/env
```

Or add to your shell profile (~/.bashrc, ~/.zshrc):
```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

### FFmpeg not found

**Ubuntu:**
```bash
sudo apt-get update
sudo apt-get install -y ffmpeg
```

**macOS:**
```bash
brew install ffmpeg
```

### Build errors

1. Update Rust:
   ```bash
   rustup update
   ```

2. Clean and rebuild:
   ```bash
   make clean
   make release
   ```

3. Check dependencies:
   ```bash
   make version
   ```

### Permission denied on install

Use sudo:
```bash
sudo make install
```

## Uninstalling

To remove ffdash from your system:

```bash
sudo make uninstall
```

To also remove build artifacts:

```bash
make clean
```

## Development Setup

For development work:

```bash
# Install dependencies
make deps

# Build in debug mode (faster compilation)
make debug

# Run tests
make test

# Format code
make fmt

# Lint code
make clippy

# Build documentation
make docs
```

## Getting Help

- Run `make help` for available commands
- Check the main [README.md](README.md) for usage instructions
- View help within the application by pressing `H`

# Docker
