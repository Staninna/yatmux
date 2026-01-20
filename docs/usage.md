# Usage

This page covers day-to-day usage that isn’t purely “edit the config”.

## Help overlay

- Toggle help: `ctrl+shift+/`
- When the help overlay is open:
  - Scroll with mouse wheel, `up/down`, `pageup/pagedown`, `home/end`.
- The help overlay uses two columns when there is enough horizontal space.
- The help overlay auto-scales to the largest readable size; override with `[ui.help].font_scale`.
- If shell integration isn’t detected, the help footer points you at `scripts/shell/yatmux.bash`.

## Rendering

yatmux uses a built-in 8x8 bitmap font (`font8x8`) and scales it up.
There is no font family selection at the moment; `[font].scale` controls the base size.

## Tabs

- New tab: `ctrl+shift+t`
- Close tab: `ctrl+shift+q`
- Next/prev tab: `ctrl+tab` / `ctrl+shift+tab`
- Jump directly: `alt+1` … `alt+9`

Tab bar notes:

- The tab bar only renders when there are **2+ tabs**.
- You can switch tabs by clicking on the tab bar.

## Panes (splits)

- Split vertical: `ctrl+shift+\\`
- Split horizontal: `ctrl+shift+-`
- Close pane: `ctrl+shift+w`
- Focus movement: `alt+arrow keys`
- Resize split: `ctrl+shift+arrow keys`

Split constraints:

- Splits are rejected if they’d create a pane smaller than `[pane].min_size` (default 100px).

## Zoom (per pane)

Zoom changes the focused pane’s font scale only.

- Zoom in/out/reset: `ctrl+alt+=`, `ctrl+alt+-`, `ctrl+alt+0`

Notes:

- Pane zoom is clamped to `1..=8`.
- The tab bar uses the global `[font].scale`, not the pane’s zoom.

## Mouse

### Selection vs mouse-aware terminal apps

yatmux switches behavior depending on whether the running app has enabled mouse reporting.

- If the terminal app **grabs the mouse** (common in TUIs): clicks/moves/scroll wheel are forwarded into the terminal.
  - Note: yatmux still uses right click for its context menu (so right click may not reach the TUI).
- Otherwise:
  - Left click focuses a pane.
  - Left click + drag selects text.
  - Selection/copy is based on the currently rendered viewport (it won’t span arbitrary scrollback that isn’t on screen).

### Scrolling

- Mouse wheel scrolls the pane under the cursor (or the focused pane as a fallback).
- `terminal.scroll_speed` only affects “line wheel” events (not pixel-precise trackpad scroll).
- If the help overlay is open, the wheel scrolls the help overlay instead of the terminal.

### Context menu

- Right click opens a context menu with actions like Copy/Paste/Search, scroll controls, and shell-integration actions when available.
- Right click on a tab opens a tab context menu (new/close, move left/right/start/end, next/previous, close others/right).
- Middle click pastes.

## Copy / paste

- Copy requires a selection (no copy-on-select): `ctrl+shift+c`.
- Paste: `ctrl+v`, `ctrl+shift+v`, `shift+insert`, or middle click.

## Scrollback behavior

- When you scroll up, new output does **not** snap you back to the bottom; yatmux keeps your current view stable as lines are appended.
- Jump to top/bottom: `ctrl+shift+home` / `ctrl+shift+end`.
- Clear scrollback: `ctrl+shift+k` (keeps viewport content).
- “Reset terminal” clears scrollback and selection (it does not restart your shell).
- When you press `enter` to run a command, yatmux snaps scrollback back to the bottom.

## Search

- Enter search: `ctrl+shift+f`
- While search is open:
  - Type to search
  - Next/prev match: `ctrl+n` / `ctrl+p` (also `down` / `up`)
  - Toggle case: `ctrl+c`
  - Toggle regex: `ctrl+r`
  - Close search: `escape`
  - `enter` advances to the next match if matches exist

Performance note:

- Search indexes the full scrollback+viewport buffer when the query (or terminal generation) changes.
- In regex mode, invalid patterns stop matching until the regex is valid again.

## Plugins (bash)

Each plugin is a folder containing a `plugin.sh` script.

Default plugin directory:

- `~/.config/yatmux/plugins`

Add extra plugin paths via `[plugins].paths` in `config.toml`.
Plugins run with `bash` from your PATH.

### Debug plugin (log hooks)

Example plugin:

- `examples/plugins/debug/plugin.sh`

To enable:

```toml
[plugins]
paths = ["./examples/plugins/debug"]
```

It logs every event to `~/.cache/yatmux/plugin-debug.log` (or `$XDG_CACHE_HOME/yatmux/plugin-debug.log`).

### Worktree plugin (git)

Example plugin:

- `examples/plugins/worktree/plugin.sh`

To enable:

```toml
[plugins]
paths = ["./examples/plugins/worktree"]
```

Suggested keybinds:

```toml
[keybinds]
"ctrl+shift+g" = { plugin = "worktree", command = "new" }
"ctrl+shift+s" = { plugin = "worktree", command = "switch" }
"ctrl+shift+x" = { plugin = "worktree", command = "close" }
"ctrl+shift+y" = { plugin = "worktree", command = "sync" }
```

Notes:

- Uses `git worktree` and creates worktrees under `.worktrees/` in the repo by default.
- `sync` opens missing worktrees in new tabs and updates tab titles.
- `close` removes the worktree and closes the matching tab when possible.

### Event input

yatmux invokes each `plugin.sh` on events and writes a JSON payload to stdin.

Environment variables:

- `YATMUX_PLUGIN_EVENT`: the full JSON event payload
- `YATMUX_PLUGIN_NAME`: plugin directory name
- `YATMUX_PLUGIN_ROOT`: plugin directory path
- `YATMUX_CONFIG_PATH`: resolved config path (if available)

### Event subscriptions

Plugins only receive `startup`/`shutdown` events by default. To receive other events, the plugin must subscribe:

```json
{"command":"subscribe","events":["action","tab_changed","pane_focus_changed"]}
```

Use `["all"]` to receive all events.

Example event payload:

```json
{"event":"action","action":"new_tab","source":"user","tab_id":1,"pane_id":1,"data":{"cwd":"/home/stan/code"}}
```

Common event names:

- `startup`, `shutdown`
- `config_reload`
- `action` (includes `action` + `source`)
- `plugin_command` (includes command data from keybinds or plugins)
- `tab_created`, `tab_closed`, `tab_changed`
- `pane_split`, `pane_closed`, `pane_focus_changed`
- `prompt_response`
- `state_response`
- `clipboard_response`

Notes:

- `data.cwd` is populated when shell integration provides OSC 7 cwd data.
- `plugin_command` events from keybinds include `data.plugin` and `data.command`.
- `prompt_response` events include `data.id`, `data.ok`, and optional `data.value`/`data.index`.

### Commands output

Print JSON commands (one per line or a JSON array) on stdout:

```json
{"command":"toast","message":"hi"}
{"command":"action","action":"split_vertical"}
{"command":"config_patch","toml":"[font]\nscale = 1.25","persist":false}
{"command":"register_keybind","key":"ctrl+shift+w","action":{"plugin":"worktree","command":"new","args":{"branch":"feat-x"}}}
{"command":"prompt","id":"branch","title":"New worktree","message":"Branch name?","default":"feat-x"}
{"command":"pick","id":"pick-branch","title":"Checkout","items":["main","feat-x"]}
```

Supported commands:

- `action`: execute a built-in action by name (snake_case)
- `toast`: show a toast message
- `set_tab_title`: set the current tab title (or include `tab_id`)
- `set_window_title`: set the window title
- `new_tab`: create a new tab at `cwd` (optional `title`)
- `set_tab_cwd`: change cwd for all panes in a tab (`cwd`, optional `tab_id`)
- `set_pane_cwd`: change cwd for one pane (`cwd`, optional `tab_id`/`pane_id`)
- `prompt`: open a text prompt (`id`, `title`, optional `message`, optional `default`)
- `confirm`: open a confirm dialog (`id`, `title`, optional `message`, optional labels)
- `pick`: open a quick-pick list (`id`, `title`, optional `message`, `items`, optional `selected`)
- `request_state`: emit a `state_response` event (`id`)
- `clipboard_read`: emit a `clipboard_response` event (`id`)
- `clipboard_write`: write text to clipboard (`text`)
- `send_text`: send raw text to a pane (`text`, optional `tab_id`/`pane_id`)
- `subscribe`: choose which events this plugin receives (`events`, or `["all"]`)
- `focus_tab`: focus a tab by id (`tab_id`)
- `close_tab`: close a tab by id (`tab_id`)
- `config_patch`: merge TOML into the live config (`persist` writes `config.toml`)
- `reload_config`: reload `config.toml` from disk
- `plugin_command`: emit a `plugin_command` event (`name`, optional `args`)
- `register_keybind`: register a keybind at runtime (`key`, `action`, optional `persist` to save `config.toml`). `action` accepts the same formats as `[keybinds]`.

### Response payloads

Example prompt response:

```json
{"event":"prompt_response","data":{"id":"branch","ok":true,"value":"feat-x","kind":"input"}}
```

Example state response:

```json
{"event":"state_response","data":{"id":"snapshot-1","active_tab":2,"tabs":[{"id":2,"title":"feat-x","focused_pane":1,"panes":[1,2],"cwd":"/home/stan/code/feat-x"}]}}
```

## URLs and hyperlinks

- yatmux detects URLs in visible text (`http(s)://…` and `www.…`) and also supports OSC 8 hyperlinks.
  - If a URL doesn’t include a scheme (e.g. `www.example.com`), yatmux opens it as `https://…`.
  - OSC 8 hyperlinks take priority over regex detection.
- Hovering a URL changes the cursor; left click opens it.
- Right click shows “Open URL” when a URL is under the cursor.

## Hex color “swatches”

When output contains `#RGB` or `#RRGGBB`, yatmux renders that span with its color as a background swatch (unless overridden by selection/search highlighting).

## Shell integration (high level)

- Shell integration is optional but powers features like:
  - Copy last output / jump between prompts
  - Sticky prompt while scrolled up
  - “Click to move cursor” within the current input zone
  - Shadow prompt type-ahead flushing on prompt return

See `docs/shell-integration.md`.
