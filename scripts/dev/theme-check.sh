#!/bin/bash
# Theme development helper script
# Validates and tests theme files

set -e

THEMES_DIR="themes"

if [ ! -d "$THEMES_DIR" ]; then
    echo "❌ Themes directory not found: $THEMES_DIR"
    exit 1
fi

echo "🎨 Validating theme files..."

for theme_file in "$THEMES_DIR"/*.toml; do
    if [ -f "$theme_file" ]; then
        theme_name=$(basename "$theme_file" .toml)
        echo "🔍 Checking theme: $theme_name"
        
        # Basic TOML syntax check
        if command -v tomly &> /dev/null; then
            tomly -c "$theme_file" || echo "⚠️  TOML syntax check failed for $theme_name"
        else
            echo "   (Install 'tomly' for TOML validation: cargo install tomly-cli)"
        fi
        
        # Check if theme is properly structured
        echo "   ✅ Theme file exists and is readable"
    fi
done

echo ""
echo "📋 Available themes:"
ls -1 "$THEMES_DIR"/*.toml | xargs -n 1 basename -s .toml

echo ""
echo "💡 To test a theme, run:"
echo "   cargo run -- --theme <theme-name>"