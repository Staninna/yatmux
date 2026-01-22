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

- `font.scale`: clamped to the configured scale range (default `1..=8`)
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

- `family` (string): `"JetBrains Mono"`
  - Currently, yatmux always uses the bundled JetBrains Mono font. System font discovery/selection is not implemented yet, so other values fall back to the bundled font.
- `size` (f32): `14.0` - Base font size (before scaling).
- `scale` (f32): `1.0` - Font scale multiplier (default `1.0..=8.0`). Accepts decimals like `1.5`, `2.25`.
- `weight` (enum): `regular` (currently unused; parsed for future use)
- `slant` (enum): `normal` (currently unused; parsed for future use)

Notes:

- The default allowed font scale range is `1..=8`.
- You can override that clamp range via `[experimental.font_scale_clamp]` (see below).

#### Font Scale Details

**Supported Values:**
- Accepts fractional values in 0.25 increments (e.g., 1.0, 1.25, 1.5, 1.75, 2.0)
- Range: 0.25 to 64.0 (further constrained by `experimental.font_scale_clamp`)
- Default: 1.0

**Why 0.25 Increments?**
Provides fine-grained control while keeping glyph cache size manageable. Each scale value pre-renders glyphs, so finer increments would increase memory without proportional UX benefit.

**Migration from Integer Scaling:**
Previous configs used integers (1-8). These still work. You can now use fractional values:
- Example: `scale = 1.5` for 150% base font size
- Keybindings increment/decrement by 0.25

**Example:**
```toml
[font]
scale = 1.5  # 150% base size

[experimental.font_scale_clamp]
min = 0.5    # Allow smaller zoom
max = 4.0    # Restrict maximum zoom
```

**Font Reload Behavior:**
Changing `font.scale` / `font.size` takes effect via config reload (Ctrl+Shift+R) and pane zoom keybindings. `font.family`/`weight`/`slant` are parsed and saved but currently don’t change rendering (system font selection is not implemented yet).

### `[pane]`

All padding values are pixels.

- `padding` (usize | omitted): omitted
- `padding_left` (usize | omitted): omitted
- `padding_right` (usize | omitted): omitted
- `padding_top` (usize | omitted): omitted
- `padding_bottom` (usize | omitted): omitted
- `min_size` (usize | omitted): omitted
- `inherit_cwd_on_split` (bool): `true`

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

### `[plugins]`

- `enabled` (bool): `true`
- `enable_default_dir` (bool): `true` (loads from `~/.config/yatmux/plugins`)
- `paths` (array of strings): `[]` (extra plugin paths)


Notes:

- Each plugin lives in a directory containing `plugin.sh`.
- Paths can be files (direct `plugin.sh`) or directories. Directories without `plugin.sh` are scanned for subdirectories containing `plugin.sh`.
- Relative paths are resolved relative to `config.toml`.
- `~` and `~/...` expand to the current user home directory.

### `[experimental]`

⚠️ **Experimental Section** - Features here may change in future versions.

#### `[experimental.font_scale_clamp]`

Override the default font scale range (1.0-8.0) to support extreme zoom levels or restrict scaling.

- `min` (f32): `1.0` - Minimum allowed font scale
- `max` (f32): `8.0` - Maximum allowed font scale

**Notes:**
- Affects `[font].scale`, pane zoom, and UI font scales (tab bar, help, toasts)
- Values are safety-clamped to `0.25..=64.0` to prevent rendering issues
- Changing these values clears the glyph cache

**Use Cases:**

1. **Accessibility** - Support larger scales:
   ```toml
   [experimental.font_scale_clamp]
   max = 16.0
   ```

2. **Presentation Mode** - Wide range for demos:
   ```toml
   [experimental.font_scale_clamp]
   min = 0.5
   max = 12.0
   ```

3. **Restricted Environment** - Limit customization:
   ```toml
   [experimental.font_scale_clamp]
   min = 1.0
   max = 2.0
   ```

- `click_move_max_steps` (usize): `512`
- `pane_resize_step` (float): `0.05`
- `focus_move_overlap_weight` (i64): `1000`

### `[ui]`

UI colors are optional. If a color is omitted, yatmux derives it from base colors.

#### `[ui.tab_bar]`

- `gap_px` (usize): `4` - Space between tabs in pixels
- `side_padding_px` (usize): `8` - Padding on left and right edges of tab bar
- `max_width_cells` (usize): `12` - Maximum tab width in character cells
- `max_width_px_extra` (usize): `16` - Additional pixels on top of cell width
- `vertical_padding_px` (usize): `8` - Vertical padding above and below tab text (increases tab bar height)
- `tab_internal_padding_px` (usize): `16` - Horizontal padding on each side of tab text (total = 2× this value)
- `min_height_cells` (usize | omitted): omitted - Optional minimum tab bar height in character cells
- `font_scale` (f32 | omitted): `2.0` - Tab text font scale (1.0-8.0, accepts decimals). Default is 2.0 for better readability. Independent from global `[font].scale`
- `background` (color | omitted): omitted
- `border` (color | omitted): omitted
- `inactive_tab_background` (color | omitted): omitted
- `inactive_text` (color | omitted): omitted

**Example: Custom tab padding**
```toml
[ui.tab_bar]
vertical_padding_px = 10        # More vertical breathing room
tab_internal_padding_px = 20    # More horizontal padding within tabs (40px total)
gap_px = 6                      # Slightly wider gaps between tabs
```

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
- `font_scale` (f32 | omitted): omitted (auto-scale up to the help overlay font scale, 1.0-8.0, accepts decimals)
- `bottom_margin_cells` (usize): `2`
- `background` (color | omitted): omitted
- `text` (color | omitted): omitted
- `border` (color | omitted): omitted

#### `[ui.help]`

- `padding_x_cells` (usize): `2`
- `padding_y_cells` (usize): `1`
- `font_scale` (f32 | omitted): `8.0` - Help overlay font scale (1.0-8.0, accepts decimals)
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

### `[profiles.<name>]`

Profiles allow you to define different sets of keybindings for different workflows. Each profile can override global keybindings and optionally define a custom border color for visual distinction.

**Key Features:**
- Each pane has its own active profile (default is `"default"`)
- Split panes inherit their parent's profile
- New tabs always start with the `"default"` profile
- Switch profiles with Ctrl+Shift+P (cycle) or Ctrl+Alt+1-9 (direct)
- Profiles only scope keybindings (not themes, fonts, or other settings)

**Keybind Resolution Order:**
1. Profile-specific keybinds (active profile's `[profiles.<name>]` section)
2. Global keybinds (main `[keybinds]` section)
3. App-level actions (tab management, help, profile switching) always use global keybinds

**Configuration:**

Each profile is a TOML section `[profiles.<name>]` with optional keybinds and border color:

```toml
[profiles.default]
border_color = 0xBD93F9  # Optional: purple border
"ctrl+shift+c" = "copy"
"ctrl+shift+v" = "paste"

[profiles.vim]
border_color = 0x50FA7B  # Optional: green border for vim mode
"ctrl+h" = "focus_left"
"ctrl+j" = "focus_down"
"ctrl+k" = "focus_up"
"ctrl+l" = "focus_right"

[profiles.emacs]
border_color = 0x8BE9FD  # Optional: cyan border for emacs mode
# ... emacs-style keybinds
```

**Border Colors:**
- `border_color` (optional): Hex color (e.g., `0xRRGGBB`) for the focused pane border
- If omitted, uses the global accent color from `[colors].accent`
- Inactive panes always use the divider color

**Example Usage:**
```toml
# Global keybinds (fallback for all profiles)
[keybinds]
"ctrl+shift+t" = "new_tab"
"ctrl+shift+q" = "close_tab"

# Vim-style navigation profile
[profiles.vim]
border_color = 0x50FA7B
"ctrl+h" = "focus_left"
"ctrl+j" = "focus_down"
"ctrl+k" = "focus_up"
"ctrl+l" = "focus_right"
"ctrl+shift+\\" = "split_vertical"
"ctrl+shift+-" = "split_horizontal"

# Standard profile
[profiles.default]
border_color = 0xBD93F9
"ctrl+shift+c" = "copy"
"ctrl+shift+v" = "paste"
"ctrl+shift+\\" = "split_vertical"
"ctrl+shift+-" = "split_horizontal"
```

**Switching Profiles:**
- Ctrl+Shift+P: Cycle to next profile
- Ctrl+Alt+1-9: Switch to profile by index (sorted alphabetically, "default" first)
- Profile switches are pane-scoped and persist until changed

**Notes:**
- The `"default"` profile always exists (even if not explicitly defined in config)
- Profile names are sorted alphabetically for indexed switching, with "default" always first
- Disabling a keybind in a profile (`"ctrl+c" = "none"`) prevents fallback to global keybinds
- App-level actions (NewTab, CloseTab, NextTab, PrevTab, Tab1-9, ToggleHelp, ReloadConfig, CycleProfile, SwitchToProfile1-9) always use global keybinds first

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
