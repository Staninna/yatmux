//! Font handling for the terminal renderer.
//!
//! This module provides font style management and glyph lookup
//! using the font8x8 bitmap font library.

use font8x8::UnicodeFonts;

/// Available font styles for rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStyle {
    Basic,
    BoxDrawing,
    Block,
    Greek,
    Hiragana,
    Latin,
    Misc,
    Sga,
}

impl Default for FontStyle {
    fn default() -> Self {
        FontStyle::Basic
    }
}

impl FontStyle {
    /// Returns an array of all available font styles.
    pub const ALL: [FontStyle; 8] = [
        FontStyle::Basic,
        FontStyle::BoxDrawing,
        FontStyle::Block,
        FontStyle::Greek,
        FontStyle::Hiragana,
        FontStyle::Latin,
        FontStyle::Misc,
        FontStyle::Sga,
    ];

    /// Gets the glyph bitmap for a character in this font style.
    /// Falls back to other fonts if the character is not found.
    pub fn get_glyph(&self, ch: char) -> [u8; 8] {
        let glyph = match self {
            FontStyle::Basic => font8x8::BASIC_FONTS.get(ch).unwrap_or([0; 8]),
            FontStyle::BoxDrawing => font8x8::BOX_FONTS.get(ch).unwrap_or([0; 8]),
            FontStyle::Block => font8x8::BLOCK_FONTS.get(ch).unwrap_or([0; 8]),
            FontStyle::Greek => font8x8::GREEK_FONTS.get(ch).unwrap_or([0; 8]),
            FontStyle::Hiragana => font8x8::HIRAGANA_FONTS.get(ch).unwrap_or([0; 8]),
            FontStyle::Latin => font8x8::LATIN_FONTS.get(ch).unwrap_or([0; 8]),
            FontStyle::Misc => font8x8::MISC_FONTS.get(ch).unwrap_or([0; 8]),
            FontStyle::Sga => font8x8::SGA_FONTS.get(ch).unwrap_or([0; 8]),
        };

        if glyph != [0; 8] {
            return glyph;
        }

        get_fallback_glyph(ch)
    }

    /// Cycles to the next font style.
    pub fn next(self) -> FontStyle {
        let idx = Self::ALL.iter().position(|&s| s == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }
}

/// Attempts to find a glyph for the given character across all font families.
fn get_fallback_glyph(ch: char) -> [u8; 8] {
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
    fn test_font_style_default() {
        assert_eq!(FontStyle::default(), FontStyle::Basic);
    }

    #[test]
    fn test_get_glyph_basic() {
        let style = FontStyle::Basic;
        let glyph = style.get_glyph('A');
        assert_ne!(glyph, [0; 8]);
    }

    #[test]
    fn test_get_glyph_empty() {
        let style = FontStyle::Basic;
        let glyph = style.get_glyph('\u{0000}');
        assert_eq!(glyph, [0; 8]);
    }

    #[test]
    fn test_box_drawing_glyph() {
        let style = FontStyle::BoxDrawing;
        let glyph = style.get_glyph('─');
        assert_ne!(glyph, [0; 8]);
    }

    #[test]
    fn test_fallback_ascii_in_box_drawing() {
        let style = FontStyle::BoxDrawing;
        let glyph = style.get_glyph('A');
        assert_ne!(glyph, [0; 8], "ASCII 'A' should render via fallback");
    }

    #[test]
    fn test_fallback_ascii_in_greek() {
        let style = FontStyle::Greek;
        let glyph = style.get_glyph('z');
        assert_ne!(glyph, [0; 8], "ASCII 'z' should render via fallback");
    }

    #[test]
    fn test_get_fallback_glyph() {
        assert_ne!(get_fallback_glyph('A'), [0; 8]);
        assert_ne!(get_fallback_glyph('─'), [0; 8]);
        assert_ne!(get_fallback_glyph('α'), [0; 8]);
    }

    #[test]
    fn test_font_style_next() {
        let style = FontStyle::Basic;
        assert_eq!(style.next(), FontStyle::BoxDrawing);

        let style = FontStyle::Sga;
        assert_eq!(style.next(), FontStyle::Basic);
    }

    #[test]
    fn test_tab_indicator_glyph() {
        let glyph = tab_indicator_glyph();
        assert_ne!(glyph, [0; 8]);
    }
}
