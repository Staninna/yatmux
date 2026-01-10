pub const FONT_SCALE: usize = 2;
pub const GLYPH_W: usize = 8;
pub const GLYPH_H: usize = 8;
pub const CELL_W: usize = GLYPH_W * FONT_SCALE;
pub const CELL_H: usize = GLYPH_H * FONT_SCALE;

pub const DEFAULT_ROWS: u16 = 24;
pub const DEFAULT_COLS: u16 = 80;

pub const DEFAULT_BG_COLOR: u32 = 0x00_10_10_10;
pub const DEFAULT_FG_COLOR: u32 = 0x00_D0_D0_D0;

pub const ESCAPE_BYTE: u8 = 0x1b;
pub const BACKSPACE_BYTE: u8 = 0x7f;
pub const NULL_BYTE: u8 = 0x00;

pub const READ_BUFFER_SIZE: usize = 8192;
