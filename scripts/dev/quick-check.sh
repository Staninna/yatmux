#!/bin/bash
# Quick development check script
# Runs format, lint, and tests in sequence

set -e

echo "🔍 Running quick development checks..."

echo "📝 Formatting code..."
cargo fmt

echo "🔍 Running lints..."
cargo clippy -- -D warnings

echo "🧪 Running tests..."
cargo test

echo ""
echo "✅ All checks passed! Code is ready to commit."