#!/bin/bash
# Development setup script for yatmux
# Sets up development environment and installs required tools

set -e

echo "🚀 Setting up yatmux development environment..."

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust/Cargo not found. Please install Rust first:"
    echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

echo "✅ Rust/Cargo found: $(cargo --version)"

# Install development tools
echo "📦 Installing development tools..."
cargo install cargo-watch cargo-audit cargo-outdated cargo-deny

# Check if project can build
echo "🔨 Checking if project builds..."
cargo check

echo ""
echo "✅ Development environment setup complete!"
echo ""
echo "📋 Next steps:"
echo "  - Run 'make run' to start yatmux"
echo "  - Run 'make watch' to watch for changes"
echo "  - Run 'make test' to run tests"
echo "  - Run 'make lint' to check code quality"
echo ""
echo "📚 Documentation is available in the docs/ directory"