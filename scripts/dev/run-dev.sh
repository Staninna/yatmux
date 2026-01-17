#!/bin/bash
# Build and run yatmux with useful development options

set -e

echo "🚀 Starting yatmux in development mode..."

# Build first to catch compilation errors early
echo "🔨 Building..."
cargo build

# Set development environment variables
export RUST_LOG=debug
export RUST_BACKTRACE=1

echo "▶️  Starting yatmux..."
echo "   Press Ctrl+C to exit"
echo "   Press Ctrl+Shift+R to reload config"
echo ""

cargo run