//! Color handling utilities for the terminal renderer.
//!
//! This module provides:
//! - A 256-color palette following xterm standards
//! - RGB color manipulation helpers
//! - Conversion between vt100::Color and u32

use vt100::Color;

use crate::constants::{DEFAULT_BG_COLOR, DEFAULT_FG_COLOR};

/// Extracts RGB components from a packed u32 color.
#[inline]
pub fn u32_to_rgb(color: u32) -> (u8, u8, u8) {
    let r = ((color >> 16) & 0xff) as u8;
    let g = ((color >> 8) & 0xff) as u8;
    let b = (color & 0xff) as u8;
    (r, g, b)
}

/// Packs RGB components into a u32 color.
#[inline]
pub fn rgb_to_u32(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

/// Lightens a color by adding 0x20 to each RGB component (capped at 0xFF).
pub fn lighten_color(color: u32) -> u32 {
    let (r, g, b) = u32_to_rgb(color);
    let r = r.saturating_add(0x20);
    let g = g.saturating_add(0x20);
    let b = b.saturating_add(0x20);
    rgb_to_u32(r, g, b)
}

/// Converts a vt100::Color to a u32 color value.
///
/// # Arguments
/// * `color` - The vt100 color to convert
/// * `default` - The default color to use if `color` is `Color::Default`
/// * `palette` - The 256-color palette for indexed colors
pub fn color_to_u32(color: Color, default: u32, palette: &[u32; 256]) -> u32 {
    match color {
        Color::Default => default,
        Color::Idx(n) => palette[n as usize],
        Color::Rgb(r, g, b) => rgb_to_u32(r, g, b),
    }
}

/// Resolves foreground color from a vt100::Color using the standard palette.
#[allow(dead_code)]
pub fn resolve_fg(color: Color, palette: &[u32; 256]) -> u32 {
    color_to_u32(color, DEFAULT_FG_COLOR, palette)
}

/// Resolves background color from a vt100::Color using the standard palette.
#[allow(dead_code)]
pub fn resolve_bg(color: Color, palette: &[u32; 256]) -> u32 {
    color_to_u32(color, DEFAULT_BG_COLOR, palette)
}

/// Creates a standard 256-color xterm palette.
///
/// Colors 0-15: Standard ANSI colors
/// Colors 16-231: 6x6x6 color cube
/// Colors 232-255: Grayscale ramp
pub fn create_palette() -> [u32; 256] {
    let mut palette = [0u32; 256];

    // Standard ANSI colors (0-15)
    palette[0] = 0x00_00_00_00; // Black
    palette[1] = 0x00_80_00_00; // Red
    palette[2] = 0x00_00_80_00; // Green
    palette[3] = 0x00_80_80_00; // Yellow
    palette[4] = 0x00_00_00_80; // Blue
    palette[5] = 0x00_80_00_80; // Magenta
    palette[6] = 0x00_00_FF_FF; // Cyan
    palette[7] = 0x00_C0_C0_C0; // White
    palette[8] = 0x00_80_80_80; // Bright Black
    palette[9] = 0x00_FF_00_00; // Bright Red
    palette[10] = 0x00_00_FF_00; // Bright Green
    palette[11] = 0x00_FF_FF_00; // Bright Yellow
    palette[12] = 0x00_00_00_FF; // Bright Blue
    palette[13] = 0x00_FF_00_FF; // Bright Magenta
    palette[14] = 0x00_00_FF_FF; // Bright Cyan
    palette[15] = 0x00_FF_FF_FF; // Bright White

    // Extended colors (16-23): Low intensity ramp
    for i in 16..24 {
        let gray = 8 + (i - 16) * 10;
        palette[i] = rgb_to_u32(gray as u8, gray as u8, gray as u8);
    }

    // 6x6x6 color cube (24-231)
    for i in 24..232 {
        let idx = i - 24;
        let r = 40 + (idx / 36) * 40;
        let g = 40 + ((idx / 6) % 6) * 40;
        let b = 40 + (idx % 6) * 40;
        palette[i] = rgb_to_u32(r as u8, g as u8, b as u8);
    }

    // Grayscale ramp (232-255)
    for i in 232..256 {
        let gray = 8 + (i - 232) * 10;
        palette[i] = rgb_to_u32(gray as u8, gray as u8, gray as u8);
    }

    palette
}

/// Creates a standard 256-color palette, optionally overriding ANSI 0-15.
pub fn create_palette_with_ansi(ansi: Option<[u32; 16]>) -> [u32; 256] {
    let mut palette = create_palette();
    if let Some(ansi) = ansi {
        for (i, c) in ansi.iter().enumerate() {
            palette[i] = *c;
        }
    }
    palette
}
