use yatmux::renderer::font::{get_glyph, tab_indicator_glyph};

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
