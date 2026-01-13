# Themes

yatmux supports theme-driven color configuration via TOML theme files.

## Theme selection

In `config.toml`:

```toml
[theme]
name = "dracula"  # built-in by default
```

To disable theme loading:

```toml
[theme]
name = "off"      # also accepts: "disabled", "none", or "" (empty)
```

## Built-in themes

Built-in themes are compiled into the binary from the repo’s `themes/` directory.

Current built-ins in this repo:

- `dracula` (default)
- `gruvbox-dark`
- `light`

## Custom themes

If `name` isn’t a built-in theme, yatmux looks for a file at:

- Linux (typical): `~/.config/yatmux/themes/<name>.toml`
- macOS: `~/Library/Application Support/yatmux/themes/<name>.toml`
- Windows: `%APPDATA%\\yatmux\\themes\\<name>.toml`

Example:

```toml
[theme]
name = "my-theme"
```

Create `%CONFIG_DIR%/yatmux/themes/my-theme.toml` with theme contents.

## Theme file format

Theme files are TOML and are currently used only for `[colors]` and `[ui]`.

A minimal theme:

```toml
[colors]
background = "#282A36"
foreground = "#F8F8F2"
accent = "#BD93F9"
```

Optional ANSI palette:

```toml
[colors]
palette = [
  "#21222C", "#FF5555", "#50FA7B", "#F1FA8C",
  "#BD93F9", "#FF79C6", "#8BE9FD", "#F8F8F2",
  "#6272A4", "#FF6E6E", "#69FF94", "#FFFFA5",
  "#D6ACFF", "#FF92DF", "#A4FFFF", "#FFFFFF",
]
```

Optional UI overrides (any of the `ui.*` optional colors):

```toml
[ui.dividers]
color = "#333333"

[ui.search]
match_bg = "#3B2E5A"
current_match_bg = "#4D3B7A"
```

## Precedence with config

yatmux merges settings in this order:

1. `[theme].imports` (if any)
2. main `config.toml`
3. theme overrides (only `[ui]` and `[colors]`)

So:

- Theme colors override your `[ui]` and `[colors]` values.
- If you want full manual control, disable the theme (`name = "off"`).
