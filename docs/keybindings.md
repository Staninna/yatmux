# Keybindings

Keybindings are configured under `[keybinds]` in `config.toml`.

## Format

The `[keybinds]` table is a flat map of:

- key: a string like `"ctrl+shift+c"` or `"alt+left"`
- value: an action name in `snake_case`

Example:

```toml
[keybinds]
"ctrl+shift+c" = "copy"
"ctrl+shift+v" = "paste"
```

### Supported modifiers

- `ctrl` (or `control`)
- `shift`
- `alt` (or `meta`)

### Key matching (important)

yatmux tries to match keybindings using the **physical** key where possible.
This is why defaults like `ctrl+shift+-` work reliably (it matches `-`, not `_`).

Practical implications:

- Prefer punctuation keys by their unshifted symbol (e.g. `-`, `=`, `\\`, `/`).
- Use the help overlay (`ctrl+shift+/`) to see the exact key strings yatmux is matching.

### Disabling a default binding

To unbind a default shortcut, set it to `"none"`:

```toml
[keybinds]
"ctrl+shift+-" = "none"  # disable split horizontal
```

Notes:

- Disabled bindings still appear in the `[keybinds]` table but are ignored at runtime.
- On load, yatmux merges in any *new* default bindings you don’t already have.
  Your explicit overrides (including `"none"`) are preserved.

## Available actions

Actions are defined in `src/config/action.rs` and are serialized as `snake_case`.

- `none`
- `copy`, `paste`
- `new_tab`, `close_tab`, `next_tab`, `prev_tab`, `tab1` .. `tab9`
- `split_vertical`, `split_horizontal`, `close_pane`
- `focus_left`, `focus_right`, `focus_up`, `focus_down`
- `resize_left`, `resize_right`, `resize_up`, `resize_down`
- `toggle_help`
- `zoom_in`, `zoom_out`, `zoom_reset`
- `scroll_page_up`, `scroll_page_down`, `scroll_line_up`, `scroll_line_down`
- `scroll_to_top`, `scroll_to_bottom`, `clear_scrollback`, `reset`
- `search_find`, `search_close`, `search_next`, `search_prev`, `search_toggle_case`, `search_toggle_regex`, `search_confirm`
- `copy_last_output`, `jump_to_prev_prompt`, `jump_to_next_prompt`, `toggle_shadow_prompt`
- `reload_config`
- `toggle_test_pattern`

## Default bindings

These are the built-in defaults from `KeybindConfig::default()`.

### General

```toml
"ctrl+shift+c" = "copy"
"ctrl+shift+v" = "paste"
"ctrl+v" = "paste"
"shift+insert" = "paste"
```

### Tabs

```toml
"ctrl+shift+t" = "new_tab"
"ctrl+shift+q" = "close_tab"
"ctrl+tab" = "next_tab"
"ctrl+shift+tab" = "prev_tab"

"alt+1" = "tab1"
"alt+2" = "tab2"
"alt+3" = "tab3"
"alt+4" = "tab4"
"alt+5" = "tab5"
"alt+6" = "tab6"
"alt+7" = "tab7"
"alt+8" = "tab8"
"alt+9" = "tab9"
```

### Panes

```toml
"ctrl+shift+\\" = "split_vertical"
"ctrl+shift+-" = "split_horizontal"

"alt+left" = "focus_left"
"alt+right" = "focus_right"
"alt+up" = "focus_up"
"alt+down" = "focus_down"

"ctrl+shift+left" = "resize_left"
"ctrl+shift+right" = "resize_right"
"ctrl+shift+up" = "resize_up"
"ctrl+shift+down" = "resize_down"

"ctrl+shift+w" = "close_pane"
"ctrl+shift+/" = "toggle_help"
```

### Zoom

```toml
"ctrl+alt+=" = "zoom_in"
"ctrl+alt+-" = "zoom_out"
"ctrl+alt+0" = "zoom_reset"
```

### Scrollback

```toml
"shift+pageup" = "scroll_page_up"
"shift+pagedown" = "scroll_page_down"
"shift+up" = "scroll_line_up"
"shift+down" = "scroll_line_down"
"ctrl+shift+home" = "scroll_to_top"
"ctrl+shift+end" = "scroll_to_bottom"

"ctrl+shift+f" = "search_find"
"ctrl+shift+k" = "clear_scrollback"
```

### Search mode

These are active while search UI is open:

```toml
"escape" = "search_close"
"enter" = "search_confirm"
"ctrl+n" = "search_next"
"ctrl+p" = "search_prev"
"ctrl+c" = "search_toggle_case"
"ctrl+r" = "search_toggle_regex"
"down" = "search_next"
"up" = "search_prev"
```

### Config

```toml
"ctrl+shift+r" = "reload_config"
```

### Shell integration

```toml
"ctrl+shift+o" = "copy_last_output"
"ctrl+shift+pageup" = "jump_to_prev_prompt"
"ctrl+shift+pagedown" = "jump_to_next_prompt"
"ctrl+shift+y" = "toggle_shadow_prompt"
```

### Debug

```toml
"ctrl+shift+g" = "toggle_test_pattern"
```
