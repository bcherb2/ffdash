# Makefile for ffdash VP9 Encoder
# Supports both Ubuntu (Linux) and macOS

# Project metadata
PROJECT_NAME := ffdash
VERSION := $(shell grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)
TARGET_DIR := target
RELEASE_DIR := $(TARGET_DIR)/release
DEBUG_DIR := $(TARGET_DIR)/debug

# Installation paths
PREFIX ?= /usr/local
BINDIR := $(PREFIX)/bin

# Detect OS
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Linux)
    OS := linux
    PKG_MANAGER := apt
    INSTALL_CMD := sudo apt-get install -y
    FFMPEG_PKG := ffmpeg
else ifeq ($(UNAME_S),Darwin)
    OS := macos
    PKG_MANAGER := brew
    INSTALL_CMD := brew install
    FFMPEG_PKG := ffmpeg
else
    $(error Unsupported operating system: $(UNAME_S))
endif

# Rust toolchain
CARGO := cargo
RUSTC := rustc

# Build flags
CARGO_BUILD_FLAGS :=
CARGO_RELEASE_FLAGS := --release

# Git configuration (set to 1 to use system git for private repos)
USE_GIT_CLI ?= 0
ifeq ($(USE_GIT_CLI),1)
    export CARGO_NET_GIT_FETCH_WITH_CLI := true
endif

# Colors for output
COLOR_RESET := \033[0m
COLOR_BOLD := \033[1m
COLOR_GREEN := \033[32m
COLOR_YELLOW := \033[33m
COLOR_BLUE := \033[34m

.PHONY: all build release debug install uninstall clean test check fmt clippy help deps install-rust install-ffmpeg docker-build docker-run docker-push check-ui

# Default target
all: release

# Help target
help:
	@echo "$(COLOR_BOLD)ffdash VP9 Encoder - Build System$(COLOR_RESET)"
	@echo ""
	@echo "$(COLOR_BOLD)Platform:$(COLOR_RESET) $(OS) ($(UNAME_S))"
	@echo "$(COLOR_BOLD)Version:$(COLOR_RESET) $(VERSION)"
	@echo ""
	@echo "$(COLOR_BOLD)Available targets:$(COLOR_RESET)"
	@echo "  $(COLOR_GREEN)make build$(COLOR_RESET)          - Build debug version"
	@echo "  $(COLOR_GREEN)make release$(COLOR_RESET)        - Build release version (optimized)"
	@echo "  $(COLOR_GREEN)make debug$(COLOR_RESET)          - Build debug version (alias)"
	@echo "  $(COLOR_GREEN)make install$(COLOR_RESET)        - Install to $(BINDIR)"
	@echo "  $(COLOR_GREEN)make uninstall$(COLOR_RESET)      - Uninstall from $(BINDIR)"
	@echo "  $(COLOR_GREEN)make clean$(COLOR_RESET)          - Remove build artifacts"
	@echo "  $(COLOR_GREEN)make test$(COLOR_RESET)           - Run tests"
	@echo "  $(COLOR_GREEN)make check$(COLOR_RESET)          - Check code without building"
	@echo "  $(COLOR_GREEN)make check-ui$(COLOR_RESET)       - Check UI interaction coverage"
	@echo "  $(COLOR_GREEN)make fmt$(COLOR_RESET)            - Format code with rustfmt"
	@echo "  $(COLOR_GREEN)make clippy$(COLOR_RESET)         - Lint code with clippy"
	@echo "  $(COLOR_GREEN)make deps$(COLOR_RESET)           - Install all dependencies"
	@echo "  $(COLOR_GREEN)make install-rust$(COLOR_RESET)   - Install Rust toolchain"
	@echo "  $(COLOR_GREEN)make install-ffmpeg$(COLOR_RESET) - Install FFmpeg"
	@echo "  $(COLOR_GREEN)make docker-build$(COLOR_RESET)   - Build Docker image"
	@echo "  $(COLOR_GREEN)make docker-run$(COLOR_RESET)     - Run Docker container"
	@echo "  $(COLOR_GREEN)make help$(COLOR_RESET)           - Show this help message"
	@echo ""
	@echo "$(COLOR_BOLD)Examples:$(COLOR_RESET)"
	@echo "  make deps                    # Install all dependencies"
	@echo "  make release                 # Build optimized binary"
	@echo "  sudo make install            # Install to system"
	@echo "  make release USE_GIT_CLI=1   # Build using system git (for private repos)"
	@echo ""
	@echo "$(COLOR_BOLD)Private Git Dependencies:$(COLOR_RESET)"
	@echo "  If you have private git dependencies, use:"
	@echo "  $(COLOR_YELLOW)USE_GIT_CLI=1 make release$(COLOR_RESET)"
	@echo ""

# Build targets
build: debug

debug:
	@echo "$(COLOR_BLUE)Building $(PROJECT_NAME) (debug)...$(COLOR_RESET)"
	$(CARGO) build $(CARGO_BUILD_FLAGS)
	@echo "$(COLOR_GREEN)✓ Debug build complete: $(DEBUG_DIR)/$(PROJECT_NAME)$(COLOR_RESET)"

release:
	@echo "$(COLOR_BLUE)Building $(PROJECT_NAME) (release)...$(COLOR_RESET)"
	$(CARGO) build $(CARGO_RELEASE_FLAGS)
	@echo "$(COLOR_GREEN)✓ Release build complete: $(RELEASE_DIR)/$(PROJECT_NAME)$(COLOR_RESET)"

# Install targets
install:
	@if [ ! -f "$(RELEASE_DIR)/$(PROJECT_NAME)" ]; then \
		if command -v cargo >/dev/null 2>&1; then \
			echo "$(COLOR_BLUE)Binary not found, building first...$(COLOR_RESET)"; \
			$(MAKE) release; \
		else \
			echo "$(COLOR_RED)Error: $(RELEASE_DIR)/$(PROJECT_NAME) not found$(COLOR_RESET)"; \
			echo "$(COLOR_YELLOW)Run 'make release' first (without sudo), then 'sudo make install'$(COLOR_RESET)"; \
			exit 1; \
		fi \
	fi
	@echo "$(COLOR_BLUE)Installing $(PROJECT_NAME) to $(BINDIR)...$(COLOR_RESET)"
	@mkdir -p $(BINDIR)
	install -m 755 $(RELEASE_DIR)/$(PROJECT_NAME) $(BINDIR)/$(PROJECT_NAME)
	@echo "$(COLOR_GREEN)✓ Installed to $(BINDIR)/$(PROJECT_NAME)$(COLOR_RESET)"
	@echo ""
	@echo "$(COLOR_BOLD)Installation complete!$(COLOR_RESET)"
	@echo "Run '$(PROJECT_NAME) --help' to get started."

uninstall:
	@echo "$(COLOR_BLUE)Uninstalling $(PROJECT_NAME)...$(COLOR_RESET)"
	rm -f $(BINDIR)/$(PROJECT_NAME)
	@echo "$(COLOR_GREEN)✓ Uninstalled$(COLOR_RESET)"

# Clean targets
clean:
	@echo "$(COLOR_BLUE)Cleaning build artifacts...$(COLOR_RESET)"
	$(CARGO) clean
	@echo "$(COLOR_GREEN)✓ Clean complete$(COLOR_RESET)"

# Test targets
test:
	@echo "$(COLOR_BLUE)Running tests...$(COLOR_RESET)"
	$(CARGO) test

check:
	@echo "$(COLOR_BLUE)Checking code...$(COLOR_RESET)"
	$(CARGO) check

check-ui:
	@echo "$(COLOR_BLUE)Checking UI interaction coverage...$(COLOR_RESET)"
	@./scripts/check-ui-logic.sh

# Code quality targets
fmt:
	@echo "$(COLOR_BLUE)Formatting code...$(COLOR_RESET)"
	$(CARGO) fmt
	@echo "$(COLOR_GREEN)✓ Code formatted$(COLOR_RESET)"

clippy:
	@echo "$(COLOR_BLUE)Running clippy...$(COLOR_RESET)"
	$(CARGO) clippy -- -D warnings

# Dependency installation
deps: install-rust install-ffmpeg
	@echo "$(COLOR_GREEN)✓ All dependencies installed$(COLOR_RESET)"

install-rust:
	@echo "$(COLOR_BLUE)Checking Rust installation...$(COLOR_RESET)"
	@if command -v $(CARGO) >/dev/null 2>&1; then \
		echo "$(COLOR_GREEN)✓ Rust already installed ($(shell $(RUSTC) --version))$(COLOR_RESET)"; \
	else \
		echo "$(COLOR_YELLOW)Installing Rust toolchain...$(COLOR_RESET)"; \
		curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; \
		echo "$(COLOR_GREEN)✓ Rust installed$(COLOR_RESET)"; \
		echo "$(COLOR_YELLOW)Please run: source $$HOME/.cargo/env$(COLOR_RESET)"; \
	fi

install-ffmpeg:
	@echo "$(COLOR_BLUE)Checking FFmpeg installation...$(COLOR_RESET)"
	@if command -v ffmpeg >/dev/null 2>&1; then \
		echo "$(COLOR_GREEN)✓ FFmpeg already installed ($(shell ffmpeg -version | head -1))$(COLOR_RESET)"; \
	else \
		echo "$(COLOR_YELLOW)Installing FFmpeg...$(COLOR_RESET)"; \
		if [ "$(OS)" = "linux" ]; then \
			sudo apt-get update && $(INSTALL_CMD) $(FFMPEG_PKG); \
		else \
			$(INSTALL_CMD) $(FFMPEG_PKG); \
		fi; \
		echo "$(COLOR_GREEN)✓ FFmpeg installed$(COLOR_RESET)"; \
	fi

# Version info
version:
	@echo "$(PROJECT_NAME) version $(VERSION)"
	@echo "OS: $(OS)"
	@echo "Rust: $(shell $(RUSTC) --version 2>/dev/null || echo 'not installed')"
	@echo "FFmpeg: $(shell ffmpeg -version 2>/dev/null | head -1 || echo 'not installed')"

# Development helpers
dev: debug
	@echo "$(COLOR_BLUE)Running in development mode...$(COLOR_RESET)"
	$(DEBUG_DIR)/$(PROJECT_NAME)

watch:
	@echo "$(COLOR_BLUE)Watching for changes...$(COLOR_RESET)"
	$(CARGO) watch -x build

# Benchmark target
bench:
	@echo "$(COLOR_BLUE)Running benchmarks...$(COLOR_RESET)"
	$(CARGO) bench

# Documentation
docs:
	@echo "$(COLOR_BLUE)Building documentation...$(COLOR_RESET)"
	$(CARGO) doc --no-deps --open

# Update dependencies
update:
	@echo "$(COLOR_BLUE)Updating dependencies...$(COLOR_RESET)"
	$(CARGO) update
	@echo "$(COLOR_GREEN)✓ Dependencies updated$(COLOR_RESET)"

# Docker targets
DOCKER_IMAGE_NAME := ffdash
DOCKER_TAG := latest

docker-build:
	@echo "$(COLOR_BLUE)Building Docker image...$(COLOR_RESET)"
	$(MAKE) release
	docker build -f docker/Dockerfile -t $(DOCKER_IMAGE_NAME):$(DOCKER_TAG) .
	@echo "$(COLOR_GREEN)✓ Docker image built: $(DOCKER_IMAGE_NAME):$(DOCKER_TAG)$(COLOR_RESET)"

docker-run:
	@echo "$(COLOR_BLUE)Running Docker container...$(COLOR_RESET)"
	docker run -it --rm \
		--device /dev/dri:/dev/dri \
		-v /path/to/videos:/videos \
		$(DOCKER_IMAGE_NAME):$(DOCKER_TAG)

docker-push:
	@echo "$(COLOR_BLUE)Pushing Docker image to registry...$(COLOR_RESET)"
	@echo "$(COLOR_YELLOW)Note: GitHub Actions handles this automatically$(COLOR_RESET)"
	@echo "For manual push, tag and push to your registry:"
	@echo "  docker tag $(DOCKER_IMAGE_NAME):$(DOCKER_TAG) ghcr.io/YOUR_USERNAME/$(DOCKER_IMAGE_NAME):$(DOCKER_TAG)"
	@echo "  docker push ghcr.io/YOUR_USERNAME/$(DOCKER_IMAGE_NAME):$(DOCKER_TAG)"
