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
