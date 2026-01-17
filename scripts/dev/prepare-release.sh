#!/bin/bash
# Release preparation script
# Performs checks and prepares for a new release

set -e

echo "🚀 Preparing release..."

# Check if working directory is clean
if [ -n "$(git status --porcelain)" ]; then
    echo "❌ Working directory is not clean. Commit or stash changes first."
    exit 1
fi

echo "✅ Working directory is clean"

# Run full test suite
echo "🧪 Running tests..."
cargo test --all-features

# Run linting
echo "🔍 Running lints..."
cargo clippy -- -D warnings

# Check formatting
echo "📝 Checking formatting..."
cargo fmt -- --check

# Build release version
echo "🔨 Building release..."
cargo build --release

# Check binary size and basic functionality
BINARY_PATH="target/release/yatmux"
if [ -f "$BINARY_PATH" ]; then
    echo "✅ Release binary built successfully"
    echo "   Size: $(du -h "$BINARY_PATH" | cut -f1)"
    
    # Test basic functionality
    echo "🧪 Testing basic functionality..."
    "$BINARY_PATH" --version || echo "⚠️  Version check failed"
    "$BINARY_PATH" --help || echo "⚠️  Help check failed"
else
    echo "❌ Release binary not found"
    exit 1
fi

echo ""
echo "✅ Release preparation complete!"
echo "   Binary is ready at: $BINARY_PATH"
echo ""
echo "📋 Next steps:"
echo "   1. Test the binary manually"
echo "   2. Update version in Cargo.toml if needed"
echo "   3. Create a git tag: git tag v$(cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name == "yatmux") | .version')"
echo "   4. Push to repository"