# Worktree Plugin

Manage git worktrees directly from yatmux tabs. Open multiple branches simultaneously with dedicated tabs for each worktree.

## Features

- 🌿 Create worktrees with automatic tab creation
- 🔄 Switch between worktrees (reuses existing tabs)
- 🗑️  Close worktrees and remove directories
- 🔄 Sync tabs with all existing worktrees

## Requirements

- Git with worktree support (Git 2.5+)
- Python 3.x
- Bash

## Commands

### Create New Worktree
```toml
[[keybind]]
key = "ctrl+shift+w"
action = { type = "plugin", name = "worktree", args = { command = "new" } }
```

Creates a new worktree:
1. Prompts for branch name
2. Creates worktree in `.worktrees/<branch>`
3. Opens new tab with branch as title

**With arguments:**
```toml
# Specify branch directly
action = { type = "plugin", name = "worktree", args = { command = "new", branch = "feature/foo" } }

# Custom path
action = { type = "plugin", name = "worktree", args = { command = "new", path = "/custom/path" } }

# Custom base directory for all worktrees
action = { type = "plugin", name = "worktree", args = { command = "new", base_dir = "~/worktrees" } }
```

### Switch to Worktree
```toml
[[keybind]]
key = "alt+w"
action = { type = "plugin", name = "worktree", args = { command = "switch" } }
```

Shows picker with all worktrees. If tab already exists for worktree, focuses it. Otherwise creates new tab.

### Close Worktree
```toml
[[keybind]]
key = "ctrl+shift+d"
action = { type = "plugin", name = "worktree", args = { command = "close" } }
```

Shows picker to select worktree to remove. Runs `git worktree remove` and closes any panes in that worktree.
Tabs close automatically if they become empty.

⚠️ **Warning:** This is destructive! Uncommitted changes may be lost.

By default, there is an extra confirmation **after** you pick a worktree (safer).
You can disable the extra confirmation by adding this to your config:

```toml
[plugins.worktree]
close_confirm_after_pick = false
```

Note: this setting is read by the plugin itself (not validated by yatmux).

If a worktree is open in multiple panes, the plugin will close **all matching panes**.
The tab is only closed if it ends up empty after those panes are removed.

### Sync Tabs with Worktrees
```toml
[[keybind]]
key = "ctrl+shift+s"
action = { type = "plugin", name = "worktree", args = { command = "sync" } }
```

Creates tabs for all worktrees that don't have tabs yet. Updates tab titles to match branch names.

**Close orphaned tabs:**
```toml
action = { type = "plugin", name = "worktree", args = { command = "sync", close_orphans = true } }
```

## Usage Example

```bash
# In your main repo
cd ~/projects/myapp

# Create worktrees for different features
Ctrl+Shift+W → "feature/auth"
Ctrl+Shift+W → "feature/ui"
Ctrl+Shift+W → "bugfix/crash"

# Switch between them
Alt+W → select worktree → tab focuses or opens

# When done with a worktree
Ctrl+Shift+D → select worktree → removed
```

## Configuration

### Worktree Location

Default worktree location: `<repo>/.worktrees/<branch-name>`

To customize, use `base_dir` argument:
```toml
[[keybind]]
key = "ctrl+shift+w"
action = { type = "plugin", name = "worktree", args = { command = "new", base_dir = "~/worktrees" } }
```

### State Cleanup

The plugin automatically cleans up old state files after 1 day by default. To configure this:

```bash
# Set cleanup age in days (in your shell config or yatmux config)
export WORKTREE_STATE_CLEANUP_DAYS=7  # Clean up after 7 days

# Disable automatic cleanup
export WORKTREE_STATE_CLEANUP_DAYS=0
```

Alternatively, edit the plugin script directly and change the `STATE_CLEANUP_DAYS` variable at the top.

## How It Works

1. **State Management:** Uses `.state/` directory to track multi-step workflows
2. **Tab Matching:** Compares tab CWDs with worktree paths
3. **Branch Detection:** Parses `git worktree list --porcelain` output
4. **Smart Reuse:** Avoids creating duplicate tabs for same worktree

## Behavior Contract

This plugin follows a strict event → command flow so it can run safely inside yatmux:

### Startup
- `startup` → `subscribe` to `plugin_command`, `prompt_response`, `state_response`
- If python3 is missing → `toast` and exit

### `new`
- If `branch` is missing → `prompt`
  - `prompt_response(ok=true)` → create worktree → `new_tab`
  - `prompt_response(ok=false)` → emit nothing
- If `branch` is provided → create worktree → `new_tab`
- If `path`/`base_dir` is provided → create worktree at that path → `new_tab`

### `switch`
- If no args → `request_state` → `pick`
  - `prompt_response(ok=true,index)` → `request_state` → `focus_tab` or `new_tab`
- If `branch` or `path` is provided → `request_state` → `focus_tab` or `new_tab`

### `close`
- Always `confirm` first
  - `prompt_response(ok=false)` → emit nothing
  - `prompt_response(ok=true)` → `request_state` → `pick`
    - `prompt_response(ok=true,index)` → `request_state` → `close_tab` (if tab matches)

### `sync`
- `request_state` → emits `new_tab` for missing worktrees
- If `close_orphans=true` → also emits `close_tab` for tabs not backed by worktrees

### Errors
- If not in a git repo → `toast` and exit

## End-to-End Script

Run the full e2e exercise:
```bash
scripts/dev/worktree-plugin-e2e.sh
```

Verbose step-by-step output:
```bash
VERBOSE=1 scripts/dev/worktree-plugin-e2e.sh
```

## Troubleshooting

**"python3 required" toast on startup:**
- Install Python 3: `apt install python3` or `brew install python3`

**Worktree not showing in list:**
- Run `git worktree list` to verify worktree exists
- Check you're in a git repository

**Tab not focusing on switch:**
- Tab CWD must exactly match worktree path
- Use `sync` command to fix tab titles

## Advanced Usage

### Bulk Setup
```bash
# Create worktrees for all branches
for branch in $(git branch | sed 's/\*//' | xargs); do
  yatmux plugin-command worktree new --branch "$branch"
done

# Sync all to tabs
Ctrl+Shift+S
```

### Integration with Scripts
```bash
#!/bin/bash
# Create worktree via plugin
cat <<EOF | yatmux-cli plugin-event
{
  "event": "plugin_command",
  "data": {
    "plugin": "worktree",
    "command": "new",
    "args": {"branch": "feature/new-feature"}
  }
}
EOF
```
