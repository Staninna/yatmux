# Configuration

yatmux is configured via a single TOML file plus optional imports and an optional theme.

## Config file location

yatmux resolves the config path using `dirs::config_dir()`.

Typical locations:

- Linux: `$XDG_CONFIG_HOME/yatmux/config.toml` (usually `~/.config/yatmux/config.toml`)
- macOS: `~/Library/Application Support/yatmux/config.toml`
- Windows: `%APPDATA%\\yatmux\\config.toml`

If the platform has no config directory (rare), yatmux falls back to built-in defaults and does **not** write a template.

## First-run behavior (template generation)

On startup, yatmux calls `Config::load()`.

- If `config.toml` exists: read and parse it.
- If `config.toml` does not exist: create parent directories and write a **commented template** config, then load using that template so the default theme is applied immediately.
- If the file exists but can’t be read / is invalid TOML / can’t deserialize: print a warning and use `Config::default()`.

## Merge & precedence rules

yatmux performs a simple, deterministic merge before deserializing into the strongly-typed `Config` struct.

### 1) Optional imports

If `[theme].imports` is present, each entry is loaded and merged **in order**.

- Relative paths are resolved relative to the directory containing `config.toml`.
- Absolute paths are allowed.
- `~` and `~/...` expand to the current user home directory.
- Missing/unreadable/invalid import files are ignored with a warning.

### 2) Main config wins

The main `config.toml` is merged on top of imports. In general:

- Tables are merged recursively.
- Non-tables (numbers/strings/bools/arrays) replace earlier values.

### 3) Theme overrides UI + colors

If theme loading is enabled and a theme TOML is found/valid, yatmux merges only the theme’s:

- `[ui]` (and nested sub-tables)
- `[colors]`

on top of everything else.

This is why the template states:

- UI color precedence: `theme > [ui] > [colors]`

## Value normalization / safety clamps

After loading, yatmux runs `config.apply_defaults()` to keep behavior sane even with unusual values.

Clamps and fallbacks:

- `font.scale`: clamped to `1..=8`
- `terminal.rows`, `terminal.cols`, `terminal.tab_width`, `terminal.scrollback_lines`: minimum `1`
- `terminal.scroll_speed`: if not finite or `<= 0`, reset to default (`3.0`)
- `ui.toast.duration_ms`: maximum `60_000`
- `ui.search.right_reserved_px`: maximum `2_000`
- `ui.tab_bar.gap_px`: maximum `128`
- `ui.tab_bar.side_padding_px`: maximum `256`
- `ui.tab_bar.max_width_cells`: clamped to `4..=200`
- `ui.tab_bar.max_width_px_extra`: maximum `512`
- `interaction.click_move_max_steps`: clamped to `1..=10_000`
- `interaction.pane_resize_step`: if not finite or `<= 0`, reset to default (`0.05`), then clamped to `0.005..=0.5`
- `interaction.focus_move_overlap_weight`: clamped to `1..=1_000_000`
- `keybinds`: missing default bindings are added (your overrides remain)

## Color formats

Most color fields accept either:

- Strings: `"#RGB"`, `"#RRGGBB"`, or `"0xRRGGBB"`
- Integers: `0xRRGGBB`

When yatmux serializes config, it writes colors as `"#RRGGBB"` strings.

`[colors].palette` must be either omitted or contain **exactly 16 colors**.

## Full schema & defaults

This section lists every supported key and its default value.

### `[theme]`

- `name` (string | omitted): `"dracula"`
  - Set to `""` (empty) or `"off"` / `"disabled"` / `"none"` to disable theme loading.
- `imports` (array of strings): `[]`

### `[window]`

- `title` (string): `"yatmux"`

### `[terminal]`

- `rows` (u16): `24`
- `cols` (u16): `80`
- `scrollback_lines` (usize): `4096`
- `scroll_speed` (float): `3.0`
- `tab_width` (usize): `8`

Notes:

- `scrollback_lines` and `scroll_speed` are active.
- `rows`, `cols`, and `tab_width` are currently not wired into the runtime (yatmux sizes the PTY from the actual window/pane size). They’re kept for future/compatibility and appear in the template.

### `[font]`

- `scale` (usize): `2` (clamped to `1..=8`)

### `[pane]`

All padding values are pixels.

- `padding` (usize | omitted): omitted
- `padding_left` (usize | omitted): omitted
- `padding_right` (usize | omitted): omitted
- `padding_top` (usize | omitted): omitted
- `padding_bottom` (usize | omitted): omitted
- `min_size` (usize | omitted): omitted

Effective defaults:

- If a per-side value is set, it wins.
- Else if `padding` is set, it is used.
- Else: default padding is `8` px.
- `min_size` defaults to `100` px.

### `[colors]`

- `background`: `"#101010"` (internally `0x00101010`)
- `foreground`: `"#D0D0D0"` (internally `0x00D0D0D0`)
- `accent`: `"#66AAFF"`
- `palette` (16-item array | omitted): omitted

### `[shell_integration]`

These flags control whether yatmux *uses* OSC information when present.
They don’t emit OSC sequences by themselves.

- `cwd_from_osc7` (bool): `true`
- `semantic_zones_from_osc133` (bool): `true`
- `title_from_osc` (bool): `true`
- `tab_title_source` (enum): `"cwd"` (`"none" | "cwd" | "title"`)
- `window_title_follows_active_tab` (bool): `true`
- `sticky_prompt` (bool): `true`
- `shadow_prompt` (enum): `"on_typing"` (`"off" | "always" | "on_typing"`)
- `shadow_prompt_enabled_by_default` (bool): `false`
- `debug_log` (bool): `false`

### `[interaction]`

- `click_move_max_steps` (usize): `512`
- `pane_resize_step` (float): `0.05`
- `focus_move_overlap_weight` (i64): `1000`

### `[ui]`

UI colors are optional. If a color is omitted, yatmux derives it from base colors.

#### `[ui.tab_bar]`

- `gap_px` (usize): `4`
- `side_padding_px` (usize): `8`
- `max_width_cells` (usize): `12`
- `max_width_px_extra` (usize): `16`
- `background` (color | omitted): omitted
- `border` (color | omitted): omitted
- `inactive_tab_background` (color | omitted): omitted
- `inactive_text` (color | omitted): omitted

#### `[ui.search]`

- `right_reserved_px` (usize): `100`
- `match_bg` (color | omitted): omitted
- `current_match_bg` (color | omitted): omitted
- `bar_bg` (color | omitted): omitted
- `bar_text` (color | omitted): omitted
- `bar_hint_text` (color | omitted): omitted
- `invalid_regex_text` (color | omitted): omitted

#### `[ui.toast]`

- `duration_ms` (u64): `1500`
- `bottom_margin_cells` (usize): `2`
- `background` (color | omitted): omitted
- `text` (color | omitted): omitted
- `border` (color | omitted): omitted

#### `[ui.help]`

- `padding_x_cells` (usize): `2`
- `padding_y_cells` (usize): `1`
- `font_scale` (usize | omitted): omitted (preferred help overlay scale, clamped to `1..=8`)
- `background` (color | omitted): omitted
- `text` (color | omitted): omitted
- `footer_text` (color | omitted): omitted

#### `[ui.sticky_prompt]`

- `background` (color | omitted): omitted
- `separator` (color | omitted): omitted

#### `[ui.context_menu]`

- `background` (color | omitted): omitted
- `hover_background` (color | omitted): omitted
- `text` (color | omitted): omitted
- `border` (color | omitted): omitted

#### `[ui.shadow_prompt]`

- `background` (color | omitted): omitted
- `text` (color | omitted): omitted
- `cursor` (color | omitted): omitted
- `prompt_indicator` (color | omitted): omitted
- `border` (color | omitted): omitted

#### `[ui.dividers]`

- `color` (color | omitted): omitted

### `[keybinds]`

See `docs/keybindings.md`.

## Minimal examples

### Change title + disable theme

```toml
[window]
title = "work"

[theme]
name = "off"
```

### Customize palette

```toml
[colors]
background = "#101010"
foreground = "#d0d0d0"
accent = "#66aaff"

palette = [
  "#000000", "#FF5555", "#50FA7B", "#F1FA8C",
  "#BD93F9", "#FF79C6", "#8BE9FD", "#F8F8F2",
  "#6272A4", "#FF6E6E", "#69FF94", "#FFFFA5",
  "#D6ACFF", "#FF92DF", "#A4FFFF", "#FFFFFF",
]
```
