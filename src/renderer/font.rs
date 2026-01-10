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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_glyph_basic() {
        let glyph = get_glyph('A');
        assert_ne!(glyph, [0; 8]);
    }

    #[test]
    fn test_get_glyph_empty() {
        let glyph = get_glyph('\u{0000}');
        assert_eq!(glyph, [0; 8]);
    }

    #[test]
    fn test_get_glyph_box_drawing() {
        let glyph = get_glyph('─');
        assert_ne!(glyph, [0; 8]);
    }

    #[test]
    fn test_get_glyph_greek() {
        let glyph = get_glyph('α');
        assert_ne!(glyph, [0; 8]);
    }

    #[test]
    fn test_tab_indicator_glyph() {
        let glyph = tab_indicator_glyph();
        assert_ne!(glyph, [0; 8]);
    }
}
