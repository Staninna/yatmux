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

Shows picker to select worktree to remove. Runs `git worktree remove` and closes tab.

⚠️ **Warning:** This is destructive! Uncommitted changes may be lost.

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
