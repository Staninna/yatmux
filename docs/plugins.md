# Plugin Development Guide

## Overview
Plugins extend yatmux with custom functionality using bash scripts.

## Quick Start

### 1. Create Plugin Directory
```bash
mkdir -p ~/.config/yatmux/plugins/my-plugin
cd ~/.config/yatmux/plugins/my-plugin
```

### 2. Create plugin.sh
```bash
#!/usr/bin/env bash
set -euo pipefail

event_json="${YATMUX_PLUGIN_EVENT:-$(cat)}"
event="$(echo "$event_json" | python3 -c 'import json,sys; print(json.load(sys.stdin)["event"])')"

if [ "$event" = "startup" ]; then
  echo '{"command":"toast","message":"Hello from my plugin!"}'
fi
```

### 3. Enable Plugin
Plugins in `~/.config/yatmux/plugins/` are loaded automatically.

## Plugin Commands Reference

### UI Commands
- `toast` - Show notification message
- `prompt` - Ask for text input
- `confirm` - Show yes/no dialog
- `pick` - Show selection list

### Tab/Pane Commands
- `new_tab` - Create new tab
- `focus_tab` - Switch to tab
- `close_tab` - Close tab
- `set_tab_title` - Rename tab
- `set_tab_cwd` / `set_pane_cwd` - Change directory
- `send_text` - Send keys to terminal

### State Commands
- `request_state` - Get app state (tabs, panes)
- `clipboard_read` / `clipboard_write` - Access clipboard

### Pane Commands
- `close_pane` - Close a pane by `tab_id` and `pane_id`

### Meta Commands
- `subscribe` - Listen to events
- `config_patch` - Modify config
- `reload_config` - Reload config
- `register_keybind` - Add keybinding

## Events

Plugins receive events as JSON:
- `startup` - Plugin loaded (always received)
- `shutdown` - App closing (always received)
- `plugin_command` - User triggered plugin
- `prompt_response` - User answered prompt
- `state_response` - App sent state
- `clipboard_response` - Clipboard contents

**Subscribe to events:**
```bash
echo '{"command":"subscribe","events":["plugin_command","tab_changed"]}'
```

`state_response` payload now includes per-pane cwd mappings:
```json
{
  "tabs": [
    {
      "id": 1,
      "panes": [1, 2],
      "pane_cwds": {
        "1": "/path/one",
        "2": "/path/two"
      }
    }
  ]
}
```

## Environment Variables

- `YATMUX_PLUGIN_EVENT` - Event JSON payload
- `YATMUX_PLUGIN_NAME` - Your plugin name
- `YATMUX_PLUGIN_ROOT` - Plugin directory path
- `YATMUX_CONFIG_PATH` - Config file path

## Advanced Patterns

### State Management
Use filesystem for multi-step workflows:
```bash
state_dir="$YATMUX_PLUGIN_ROOT/.state"
mkdir -p "$state_dir"

# Save state
echo '{"step":"awaiting_input"}' > "$state_dir/request-123.json"

# Load state later
state=$(cat "$state_dir/request-123.json")

# Cleanup
rm "$state_dir/request-123.json"
```

### JSON Helpers
```bash
json_get() {
  python3 - "$1" "$2" <<'PY'
import json,sys
path=sys.argv[1]
data=json.loads(sys.argv[2])
# Navigate path...
PY
}

emit_command() {
  # Use json_escape for safety
  printf '{"command":"%s","field":"%s"}\n' "$1" "$(json_escape "$2")"
}
```

## Security Best Practices

⚠️ **Plugins have full system access**
- Only install plugins from trusted sources
- Review plugin code before enabling
- Plugins run with your user permissions
- No sandboxing (by design for flexibility)

**Safe practices:**
- Validate all user input
- Use `--` in shell commands to prevent option injection
- Escape shell quotes in user-provided strings
- Don't pass secrets via environment variables

## Examples

See `examples/plugins/` for real-world examples:
- `hello-toast` - Simple toast notification
- `worktree` - Git worktree management (advanced)
- `tab-summary` - Display tab information
- `send-snippet` - Code snippet insertion

## Behavior Contracts

For multi-step plugins, document the expected event → command flow so it stays testable and predictable.

Example: worktree plugin (simplified)
- `startup` → `subscribe`
- `plugin_command:new` → `prompt` (if branch missing) → `prompt_response` → `new_tab`
- `plugin_command:switch` → `request_state` → `pick` → `prompt_response` → `request_state` → `focus_tab`/`new_tab`
- `plugin_command:close` → `confirm` → `prompt_response` → `request_state` → `pick` → `prompt_response` → `request_state` → `close_tab`
- `plugin_command:sync` → `request_state` → `new_tab` (+ `close_tab` if `close_orphans=true`)

## Troubleshooting

**Plugin not loading:**
- Check plugin.sh is executable: `chmod +x plugin.sh`
- Check shebang line: `#!/usr/bin/env bash`
- Check stderr output: plugins log errors there

**Events not received:**
- Verify subscription in startup event
- Check event name spelling (lowercase)

**JSON parsing errors:**
- Validate JSON with `python3 -m json.tool`
- Check for unescaped quotes in strings
