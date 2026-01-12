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
