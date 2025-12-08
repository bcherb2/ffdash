#!/bin/bash
# Quick installation script for ffdash VP9 Encoder
# Supports Ubuntu and macOS

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  ffdash VP9 Encoder - Quick Installer${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Detect OS
OS="$(uname -s)"
case "$OS" in
    Linux*)
        PLATFORM="Ubuntu/Linux"
        PKG_MANAGER="apt-get"
        ;;
    Darwin*)
        PLATFORM="macOS"
        PKG_MANAGER="brew"
        ;;
    *)
        echo -e "${RED}✗ Unsupported operating system: $OS${NC}"
        exit 1
        ;;
esac

echo -e "${GREEN}✓ Detected platform: ${PLATFORM}${NC}"
echo ""

# Check for make
if ! command -v make &> /dev/null; then
    echo -e "${YELLOW}⚠ 'make' not found. Installing build essentials...${NC}"
    if [[ "$OS" == "Linux"* ]]; then
        sudo apt-get update
        sudo apt-get install -y build-essential
    else
        xcode-select --install 2>/dev/null || echo "Xcode command line tools already installed"
    fi
fi

# Check for Rust
if ! command -v cargo &> /dev/null; then
    echo -e "${YELLOW}⚠ Rust not found. Installing Rust toolchain...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo -e "${GREEN}✓ Rust installed${NC}"
else
    echo -e "${GREEN}✓ Rust already installed ($(rustc --version))${NC}"
fi

# Check for FFmpeg
if ! command -v ffmpeg &> /dev/null; then
    echo -e "${YELLOW}⚠ FFmpeg not found. Installing FFmpeg...${NC}"
    if [[ "$OS" == "Linux"* ]]; then
        sudo apt-get update
        sudo apt-get install -y ffmpeg
    else
        if ! command -v brew &> /dev/null; then
            echo -e "${RED}✗ Homebrew not found. Please install Homebrew first:${NC}"
            echo -e "  /bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
            exit 1
        fi
        brew install ffmpeg
    fi
    echo -e "${GREEN}✓ FFmpeg installed${NC}"
else
    echo -e "${GREEN}✓ FFmpeg already installed${NC}"
fi

echo ""
echo -e "${BLUE}Building ffdash...${NC}"

# Check if we have private git dependencies (waveformchart)
if grep -q "git.*waveformchart" Cargo.toml 2>/dev/null; then
    echo -e "${YELLOW}⚠ Detected private git dependencies, using system git CLI...${NC}"
    export CARGO_NET_GIT_FETCH_WITH_CLI=true
    make release USE_GIT_CLI=1
else
    make release
fi

echo ""
echo -e "${BLUE}Installing ffdash...${NC}"
PREFIX="${PREFIX:-$HOME/.local}"
echo -e "${GREEN}Target prefix: ${PREFIX}${NC}"

if [[ "$EUID" -eq 0 ]]; then
    make install PREFIX="${PREFIX}"
elif [[ "${PREFIX}" == /usr/local* ]]; then
    sudo make install PREFIX="${PREFIX}"
else
    make install PREFIX="${PREFIX}"
fi

echo ""
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✓ Installation complete!${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "Run ${YELLOW}ffdash --help${NC} to get started"
echo -e "Press ${YELLOW}H${NC} within the app to view the help screen"
echo ""
