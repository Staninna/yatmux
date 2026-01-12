//! Font handling for the terminal renderer.
//!
//! This module provides glyph lookup using the font8x8 bitmap font library.

use font8x8::UnicodeFonts;

/// Gets the glyph bitmap for a character, falling back across font families.
pub fn get_glyph(ch: char) -> [u8; 8] {
    font8x8::BASIC_FONTS
        .get(ch)
        .or_else(|| font8x8::BOX_FONTS.get(ch))
        .or_else(|| font8x8::BLOCK_FONTS.get(ch))
        .or_else(|| font8x8::GREEK_FONTS.get(ch))
        .or_else(|| font8x8::HIRAGANA_FONTS.get(ch))
        .or_else(|| font8x8::LATIN_FONTS.get(ch))
        .or_else(|| font8x8::MISC_FONTS.get(ch))
        .or_else(|| font8x8::SGA_FONTS.get(ch))
        .unwrap_or([0; 8])
}

/// Gets the glyph for the tab indicator character ('>').
pub fn tab_indicator_glyph() -> [u8; 8] {
    font8x8::BASIC_FONTS.get('>').unwrap_or([0; 8])
}
