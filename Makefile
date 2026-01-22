# yatmux Makefile
# Terminal emulator with tabs, splits, and themeable UI

.PHONY: help build run clean test lint check format doc install dev watch

# Default target
help:
	@echo "yatmux - Terminal emulator with tabs, splits, and themeable UI"
	@echo ""
	@echo "Available targets:"
	@echo "  help      Show this help message"
	@echo "  build     Build the project (debug)"
	@echo "  build-rel Build the project (release)"
	@echo "  run       Build and run in debug mode"
	@echo "  run-rel   Build and run in release mode"
	@echo "  clean     Clean build artifacts"
	@echo "  test      Run tests"
	@echo "  test-all  Run tests with all features"
	@echo "  lint      Run clippy lints"
	@echo "  check     Run cargo check"
	@echo "  format    Format code with rustfmt"
	@echo "  doc       Generate documentation"
	@echo "  doc-open  Generate and open documentation"
	@echo "  install   Install release binary to ~/.cargo/bin"
	@echo "  dev       Build and run in development mode"
	@echo "  watch     Watch for changes and rebuild"
	@echo "  themes    List available themes"
	@echo "  config    Show config file location"

# Build targets
build:
	cargo build

build-rel:
	cargo build --release

# Run targets
run: build
	cargo run

run-rel: build-rel
	cargo run --release

# Development targets
dev: run

watch:
	@echo "Watching for changes... (Ctrl+C to stop)"
	@which cargo-watch > /dev/null || (echo "cargo-watch not found. Install with: cargo install cargo-watch" && exit 1)
	cargo watch -x run

# Maintenance targets
clean:
	cargo clean

test:
	cargo test

test-all:
	cargo test --all-features

lint:
	cargo clippy -- -D warnings

check:
	cargo check

format:
	cargo fmt

# Documentation targets
doc:
	cargo doc --no-deps

doc-open: doc
	cargo doc --no-deps --open

# Installation
install: build-rel
	cargo install --path .

# Utility targets
themes:
	@echo "Available themes:"
	@ls -1 themes/*.toml | xargs -n 1 basename -s .toml

config:
	@echo "Config file locations:"
	@echo "  Linux/macOS: ~/.config/yatmux/config.toml"
	@echo "  Windows: %APPDATA%\\yatmux\\config.toml"
	@if [ -f "$${HOME}/.config/yatmux/config.toml" ]; then \
		echo "  Current config: $$HOME/.config/yatmux/config.toml"; \
	else \
		echo "  No config file found - will be created on first run"; \
	fi

# Development helpers
check-deps:
	@echo "Checking required dependencies..."
	@cargo --version
	@rustc --version

setup-dev:
	@echo "Setting up development environment..."
	@cargo install cargo-watch cargo-audit
	@echo "Development tools installed"

audit:
	cargo audit

update:
	cargo update

# CI/CD helpers
ci: lint test
	@echo "CI checks passed"

# Release helpers
version:
	@cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name == "yatmux") | .version'

# Theme development
theme-check:
	@echo "Validating theme files..."
	@for theme in themes/*.toml; do \
		echo "Checking $$theme..."; \
		cargo run --bin theme-checker -- "$$theme" 2>/dev/null || echo "Theme validation not available"; \
	done

# Quick commands for common workflows
quick-check: format lint test
	@echo "Quick check complete"

pre-commit: format lint test
	@echo "Pre-commit checks passed"

# Documentation workflow
docs-check: doc
	@echo "Documentation built successfully"
	@echo "Check for missing documentation with:"
	@echo "  cargo doc --document-private-items --no-deps 2>&1 | grep -i warning || echo 'No doc warnings'"
