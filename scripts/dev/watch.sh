#!/bin/bash
# Watch for changes and rebuild automatically
# Uses cargo-watch for file watching

set -e

if ! command -v cargo-watch &> /dev/null; then
    echo "❌ cargo-watch not found. Install with:"
    echo "   cargo install cargo-watch"
    exit 1
fi

echo "👀 Watching for changes... (Ctrl+C to stop)"
echo "   Will rebuild and restart yatmux on any file change"
echo ""

cargo watch -x run