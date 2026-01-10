# TODO List - Terminal Emulator Improvements

## Easy (1-2 hours each)

### 1. Extract magic numbers into constants ✅
- Extract 80x24 default terminal size into constants
- Extract 0x7f, 0x1b and other magic byte values
- Extract hardcoded colors like 0x00_10_10_10

### 2. Compute color_palette() once at startup ✅
- Move palette computation to main() instead of each render
- Store as Arc<[u32; 256]> or similar

### 3. Replace unwrap() with proper error handling ✅
- Replace `parser.lock().ok().unwrap()` with proper error handling
- Make Pty operations fail gracefully

## Medium (3-6 hours each)

### 4. Split main.rs into modules
- Create src/parser.rs for terminal parsing
- Create src/renderer.rs for rendering logic
- Create src/pty.rs for PTY handling
- Create src/keys.rs for keyboard handling
- Create src/config.rs for configuration

### 5. Add unit/integration tests
- Test key_to_pty_bytes function
- Test color palette generation
- Test cell rendering logic

### 6. Add more font options
- Support fontBOX_DRAWING8x8::_FONTS
- Support additional Unicode fonts
- Add font selection CLI argument

## Hard (1-2 days each)

### 7. Tab support ✅
- Render `\t` as a highlighted range up to the next tab stop
- Draw a visual arrow at the leading tab cell
- Keep tab spacing aligned to 8-column stops

### 8. Selection with mouse
- Detect mouse drag events
- Calculate selection bounds
- Draw selection highlight
- Copy to clipboard on release

### 9. Scrollback buffer
- Store previous screen contents
- Add scroll wheel support
- Configurable scrollback size

### 10. Clipboard copy/paste ✅
- Integrate with clipboard via `arboard`
- Paste text on `Ctrl+V` or `Shift+Insert`
- Keep sending terminal input untouched when clipboard is empty or fails

### 11. Clickable URLs
- Regex pattern to detect URLs
- Underline/color URLs
- Handle clicks on URLs

### 12. Window transparency
- Enable transparent surface
- Blur background
- Wallpaper behind terminal

### 13. Config file
- Create ~/.config/term/config.toml
- Load font, colors, transparency settings
- Hot reload support

### 14. Custom key bindings
- JSON/YAML config for bindings
- Support for macros
- Vi mode keybindings

### 15. True color terminal support
- Detect 24-bit color support
- Pass true color to terminal
- Dithering fallback if needed

### 16. Search in terminal
- Search bar UI
- Regex search
- Highlight matches
- Next/previous match

### 17. Batch pixel operations / SIMD
- Use SIMD for glyph rendering
- Parallelize row rendering
- Optimize buffer fills
