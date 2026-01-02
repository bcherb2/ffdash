# Building from Source

For most users, [pre-built binaries](https://github.com/bcherb2/ffdash/releases) or package managers (AUR, Homebrew) are easier. This guide is for building from source.

## Quick Build

```bash
# Install dependencies (Rust + FFmpeg)
make deps

# Build optimized binary
make release

# Install to /usr/local/bin
sudo make install

# Verify
ffdash --help
```

## Makefile Targets

Run `make help` to see all targets.

### Build
| Target | Description |
|--------|-------------|
| `make release` | Build optimized release binary |
| `make debug` | Build debug binary (faster compile) |
| `make install` | Install to `/usr/local/bin` (requires sudo) |
| `make uninstall` | Remove from system |
| `make clean` | Remove build artifacts |

### Development
| Target | Description |
|--------|-------------|
| `make test` | Run test suite |
| `make check` | Check code without building |
| `make fmt` | Format code with rustfmt |
| `make clippy` | Lint with clippy |
| `make docs` | Build and open documentation |

### Dependencies
| Target | Description |
|--------|-------------|
| `make deps` | Install Rust + FFmpeg |
| `make install-rust` | Install Rust only |
| `make install-ffmpeg` | Install FFmpeg only |

## Custom Install Location

```bash
# Install to ~/.local/bin (no sudo needed)
make install PREFIX=$HOME/.local

# Install to /opt/bin
sudo make install PREFIX=/opt
```

## Development Setup

```bash
make deps          # Install dependencies
make debug         # Build debug version
make test          # Run tests
make fmt           # Format code
make clippy        # Lint
```

## Troubleshooting

**Rust not found after installation**
```bash
source $HOME/.cargo/env
# Or add to ~/.bashrc: export PATH="$HOME/.cargo/bin:$PATH"
```

**FFmpeg not found**
```bash
# Ubuntu/Debian
sudo apt-get update && sudo apt-get install -y ffmpeg

# macOS
brew install ffmpeg
```

**Build errors**
```bash
rustup update      # Update Rust
make clean         # Clean build artifacts
make release       # Rebuild
```

**Permission denied on install**
```bash
sudo make install
```
