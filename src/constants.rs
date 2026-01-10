//! Configuration for the terminal emulator.
//!
//! This module provides compile-time constants and configuration values.

// Font rendering
pub const FONT_SCALE: usize = 2;
pub const GLYPH_W: usize = 8;
pub const GLYPH_H: usize = 8;
pub const CELL_W: usize = GLYPH_W * FONT_SCALE;
pub const CELL_H: usize = GLYPH_H * FONT_SCALE;

// Terminal dimensions
pub const DEFAULT_ROWS: u16 = 24;
pub const DEFAULT_COLS: u16 = 80;

// Colors
pub const DEFAULT_BG_COLOR: u32 = 0x00_10_10_10;
pub const DEFAULT_FG_COLOR: u32 = 0x00_D0_D0_D0;

// Control bytes
pub const ESCAPE_BYTE: u8 = 0x1b;
pub const BACKSPACE_BYTE: u8 = 0x7f;
pub const NULL_BYTE: u8 = 0x00;

// Buffer sizes
pub const READ_BUFFER_SIZE: usize = 8192;
pub const SCROLLBACK_CAPACITY: usize = 4096;

// Tab handling
pub const TAB_STOP_WIDTH: usize = 8;

// Scroll settings
pub const SCROLL_SPEED_MULTIPLIER: f32 = 3.0;

#[cfg(test)]
mod tests {
    use super::*;

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
}
