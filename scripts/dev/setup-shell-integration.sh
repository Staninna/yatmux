#!/bin/bash
# Shell integration setup script
# Installs and configures shell integration for yatmux

set -e

SHELL_NAME=$(basename "$SHELL")
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SHELL_INTEGRATION_FILE="$SCRIPT_DIR/../shell/yatmux.bash"

echo "🐚 Setting up shell integration for $SHELL_NAME..."

if [ ! -f "$SHELL_INTEGRATION_FILE" ]; then
    echo "❌ Shell integration script not found: $SHELL_INTEGRATION_FILE"
    exit 1
fi

case "$SHELL_NAME" in
    "bash")
        CONFIG_FILE="$HOME/.bashrc"
        ;;
    *)
echo "⚠️  Unsupported shell: $SHELL_NAME"
        echo "   Only bash is supported at this time"
        echo "   Manual integration required for other shells"
        exit 1
        ;;
esac

# Check if already configured
if grep -q "yatmux.bash" "$CONFIG_FILE" 2>/dev/null; then
    echo "✅ Shell integration already configured in $CONFIG_FILE"
else
    echo "# yatmux shell integration" >> "$CONFIG_FILE"
    echo "source \"$SHELL_INTEGRATION_FILE\"" >> "$CONFIG_FILE"
    echo "✅ Added shell integration to $CONFIG_FILE"
fi

echo ""
echo "📋 Shell integration features:"
echo "   - OSC 7 URL tracking"
echo "   - OSC 133 command tracking"
echo "   - Enhanced terminal state detection"

echo ""
echo "💡 To apply changes:"
echo "   Restart your shell or run: source $CONFIG_FILE"
echo ""
echo "📚 For more details, see: docs/shell-integration.md"