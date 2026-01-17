#!/bin/bash
# Documentation generation and validation script
# Builds documentation and checks for coverage issues

set -e

echo "📚 Building documentation..."

# Generate documentation
cargo doc --no-deps

echo "✅ Documentation generated successfully"
echo "   View with: cargo doc --open"

# Check for undocumented items
echo ""
echo "🔍 Checking for undocumented items..."
DOC_WARNINGS=$(cargo doc --document-private-items --no-deps 2>&1 | grep -i warning || true)

if [ -n "$DOC_WARNINGS" ]; then
    echo "⚠️  Documentation warnings found:"
    echo "$DOC_WARNINGS"
else
    echo "✅ No documentation warnings found"
fi

echo ""
echo "📋 Documentation files:"
echo "   - docs/README.md (User documentation)"
echo "   - docs/usage.md (Usage guide)"
echo "   - docs/config.md (Configuration)"
echo "   - docs/keybindings.md (Key bindings)"
echo "   - docs/themes.md (Theme system)"
echo "   - docs/shell-integration.md (Shell integration)"

echo ""
echo "💡 Tips for maintaining documentation:"
echo "   - Update docs/ whenever you change UI/UX"
echo "   - Update docs/config.md for config changes"
echo "   - Update docs/keybindings.md for keybinding changes"
echo "   - Update docs/themes.md for theme system changes"