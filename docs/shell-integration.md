# Shell integration (OSC 7 / OSC 133)

yatmux can consume shell-emitted OSC sequences to provide better titles, navigation, and “sticky prompt” UI.

This has two parts:

1. Your shell must **emit** the OSC markers.
2. yatmux must be configured to **use** them.

## What you get

With shell integration working:

- Tab titles can reflect the current working directory (OSC 7) or shell title (OSC 0/1/2).
  - With `tab_title_source = "cwd"`, yatmux displays the basename of the OSC 7 `file://...` path.
- Window title can follow the active tab (format: `<tab title> — <window.title>`).
- Scrollback features can use semantic prompt zones (OSC 133):
  - Jump to previous/next prompt
  - Copy last command output
  - Sticky prompt (show prompt while scrolled up)
  - Click-to-move cursor (limited; only within the active input zone)
- Shadow prompt can provide type-ahead while a command is running.

## Configuration knobs

In `config.toml` under `[shell_integration]`:

- `cwd_from_osc7` (default `true`): read current directory from OSC 7.
- `semantic_zones_from_osc133` (default `true`): enable semantic zones features.
- `title_from_osc` (default `true`): read title from OSC 0/1/2.
- `tab_title_source` (default `"cwd"`): `"none" | "cwd" | "title"`.
- `window_title_follows_active_tab` (default `true`)
- `sticky_prompt` (default `true`)
- `shadow_prompt` (default `"on_typing"`):
  - `"off"`: never show shadow prompt
  - `"always"`: show during command execution
  - `"on_typing"`: show only after you start typing while a command is running
- `shadow_prompt_enabled_by_default` (default `false`): initial per-pane toggle.
- `debug_log` (default `false`): prints shell integration status changes; may be noisy.

## Bash setup

This repo ships a bash integration script at `scripts/shell/yatmux.bash`.

Add to your `~/.bashrc` (or similar):

```bash
# Only enable for interactive shells
[[ $- == *i* ]] || return

# Option A: always enable
source /path/to/yatmux/scripts/shell/yatmux.bash

# Option B: gate by TERM_PROGRAM (yatmux sets TERM_PROGRAM=yatmux)
# [[ ${TERM_PROGRAM-} == yatmux ]] && source /path/to/yatmux/scripts/shell/yatmux.bash
```

### Environment variables set by yatmux

yatmux sets these in the spawned shell process:

- `TERM=xterm-256color`
- `TERM_PROGRAM=yatmux`
- `TERM_PROGRAM_VERSION=<yatmux version>`
- `YATMUX=1`

What it emits:

- OSC 7 (cwd): `ESC ] 7 ; file://<host><path> BEL`
- OSC 133 markers:
  - `133;A` prompt start
  - `133;B` prompt end / input start
  - `133;C` input end / output start
  - `133;D;<status>` command end

## Notes / gotchas

- Without OSC markers, yatmux still works; shell integration features just stay inactive.
- Shadow prompt is disabled automatically for alt-screen apps (vim/less/htop), because it would interfere.
- Shadow prompt relies on OSC 133 markers to detect prompt return; without OSC 133, buffered type-ahead may never flush automatically.
- Sticky prompt also relies on semantic prompt zones; without OSC 133 markers, it won’t have prompt boundaries to display.
- `command_running` state is a heuristic: it flips to “running” when you press Enter and flips back when OSC 133 prompt markers are seen. Without OSC 133, it can get stuck “running”, which affects sticky prompt + shadow prompt.
- Click-to-move cursor currently only works on the cursor row and only inside the current input zone, and it’s capped by `interaction.click_move_max_steps`.
