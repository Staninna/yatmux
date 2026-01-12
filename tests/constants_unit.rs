use yatmux::constants::*;

#[test]
fn test_cell_dimensions() {
    assert_eq!(CELL_W, 16);
    assert_eq!(CELL_H, 16);
    assert_eq!(GLYPH_W * FONT_SCALE, CELL_W);
    assert_eq!(GLYPH_H * FONT_SCALE, CELL_H);
}

#[test]
fn test_default_terminal_size() {
    assert_eq!(DEFAULT_ROWS, 24);
    assert_eq!(DEFAULT_COLS, 80);
}

#[test]
fn test_color_formats() {
    assert_eq!(DEFAULT_BG_COLOR & 0xFF, 0x10);
    assert_eq!(DEFAULT_FG_COLOR & 0xFF, 0xD0);
}

#[test]
fn test_byte_constants() {
    assert_eq!(ESCAPE_BYTE, 27);
    assert_eq!(BACKSPACE_BYTE, 127);
    assert_eq!(NULL_BYTE, 0);
}

#[test]
fn test_read_buffer_size() {
    assert!(READ_BUFFER_SIZE > 0);
    assert!(READ_BUFFER_SIZE.is_power_of_two());
}
