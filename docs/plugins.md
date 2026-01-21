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

See the **Available Data** section below for complete details on event payloads and response structures.

## Available Data

Plugins receive rich contextual data through event payloads and environment variables. This section documents all available data structures.

### Event Payload Structure

All events follow this JSON structure:
```json
{
  "event": "event_name",
  "action": "action_name",  // optional
  "source": "user|plugin",  // optional
  "tab_id": 123,            // optional - current tab ID
  "pane_id": 456,           // optional - focused pane ID
  "data": {                 // event-specific data
    // varies by event type
  }
}
```

### Context Fields

Many events include contextual information:
- **`tab_id`** (number) - ID of the active tab when event occurred
- **`pane_id`** (number) - ID of the focused pane within the active tab
- **`source`** (string) - Either `"user"` (triggered by user action) or `"plugin"` (triggered by another plugin)

### Event-Specific Data

#### `plugin_command` Event
Triggered when a plugin keybinding is pressed or another plugin sends a command.

**When triggered by keybinding:**
```json
{
  "event": "plugin_command",
  "source": "user",
  "tab_id": 1,
  "pane_id": 2,
  "data": {
    "plugin": "plugin-name",        // target plugin name
    "command": "my_command",        // command name
    "args": ["arg1", "arg2"],       // optional arguments (may be null)
    "cwd": "/home/user/project"     // current working directory
  }
}
```

**When triggered programmatically:**
```json
{
  "event": "plugin_command",
  "source": "plugin",
  "data": {
    "name": "command_name",         // command name
    "args": {...},                  // optional arguments (may be null)
    "cwd": "/home/user/project"     // current working directory
  }
}
```

**Example: Extract command name and arguments**
```bash
event_json="${YATMUX_PLUGIN_EVENT:-$(cat)}"
name=$(echo "$event_json" | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d["data"]["name"])')
args=$(echo "$event_json" | python3 -c 'import json,sys; d=json.load(sys.stdin); print(" ".join(d["data"].get("args", [])))')
```

#### `prompt_response` Event
Response to `prompt`, `confirm`, or `pick` commands.

```json
{
  "event": "prompt_response",
  "data": {
    "id": "request-123",      // matches your request ID
    "ok": true,               // true if confirmed/submitted, false if canceled
    "value": "user input",    // text entered (prompts) or selected item (pick) or empty (confirms)
    "index": 0,               // (pick only) index of selected item
    "kind": "prompt",         // type: "prompt", "confirm", or "pick"
    "reason": "escape"        // (optional) cancellation reason if ok=false
  }
}
```

**Note:** The core fields are `id`, `ok`, and `value`. Additional fields like `index`, `kind`, and `reason` are also available.

**Example: Handle prompt response**
```bash
id=$(echo "$event_json" | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["id"])')
ok=$(echo "$event_json" | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["ok"])')
value=$(echo "$event_json" | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"].get("value", ""))')

if [ "$ok" = "True" ]; then
  echo "User entered: $value"
fi
```

#### `state_response` Event
Response to `request_state` command. Provides complete application state.

```json
{
  "event": "state_response",
  "data": {
    "id": "state-req-123",    // matches your request ID
    "active_tab": 1,          // currently focused tab ID
    "tabs": [
      {
        "id": 1,
        "title": "main",
        "focused_pane": 2,
        "panes": [1, 2, 3],
        "cwd": "/home/user",  // focused pane's cwd
        "pane_cwds": {        // per-pane working directories
          "1": "/home/user/project1",
          "2": "/home/user/project2",
          "3": "/home/user/project3"
        }
      }
    ]
  }
}
```

**Tab Object Fields:**
| Field | Type | Description |
|-------|------|-------------|
| `id` | number | Unique tab identifier |
| `title` | string | Tab display title |
| `focused_pane` | number | ID of currently focused pane in this tab |
| `panes` | number[] | List of all pane IDs in this tab |
| `cwd` | string | Working directory of focused pane |
| `pane_cwds` | object | Map of pane ID to working directory for each pane |

**Example: Find tabs in a specific directory**
```bash
# Request state
request_id="find-tabs-$(date +%s%N)"
echo "{\"command\":\"request_state\",\"id\":\"$request_id\"}"

# Later, in state_response handler:
event_json="${YATMUX_PLUGIN_EVENT:-$(cat)}"
target_dir="/home/user/project"

# Extract tabs matching directory
matching_tabs=$(echo "$event_json" | python3 - "$target_dir" <<'PY'
import json, sys
target = sys.argv[1]
data = json.load(sys.stdin)
tabs = data["data"]["tabs"]
matches = [t for t in tabs if t.get("cwd", "").startswith(target)]
print(json.dumps(matches))
PY
)
```

#### `clipboard_response` Event
Response to `clipboard_read` command.

```json
{
  "event": "clipboard_response",
  "data": {
    "id": "clipboard-123",    // matches your request ID
    "text": "clipboard text"  // clipboard contents (may be empty)
  }
}
```

**Example: Process clipboard content**
```bash
clipboard_text=$(echo "$event_json" | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"].get("text", ""))')
if [ -n "$clipboard_text" ]; then
  # Process clipboard content
  echo "{\"command\":\"toast\",\"message\":\"Clipboard: $clipboard_text\"}"
fi
```

### Working with State

**Common Patterns:**

**1. Check if a tab exists:**
```bash
tab_exists() {
  local tab_id="$1"
  local state_json="$2"
  echo "$state_json" | python3 - "$tab_id" <<'PY'
import json, sys
tab_id = int(sys.argv[1])
data = json.load(sys.stdin)
tabs = data["data"]["tabs"]
exists = any(t["id"] == tab_id for t in tabs)
print("true" if exists else "false")
PY
}
```

**2. Get tab count:**
```bash
tab_count=$(echo "$state_json" | python3 -c 'import json,sys; print(len(json.load(sys.stdin)["data"]["tabs"]))')
```

**3. Iterate over all panes:**
```bash
echo "$state_json" | python3 <<'PY'
import json, sys
data = json.load(sys.stdin)
for tab in data["data"]["tabs"]:
    for pane_id in tab["panes"]:
        cwd = tab["pane_cwds"].get(str(pane_id), "unknown")
        print(f"Tab {tab['id']}, Pane {pane_id}: {cwd}")
PY
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
