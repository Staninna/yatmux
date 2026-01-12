use yatmux::renderer::{create_palette, create_palette_with_ansi};

#[test]
fn test_palette_size() {
    let palette = create_palette();
    assert_eq!(palette.len(), 256);
}

#[test]
fn test_palette_standard_colors() {
    let palette = create_palette();
    assert_eq!(palette[0], 0x00_00_00_00);
    assert_eq!(palette[15], 0x00_FF_FF_FF);
}

#[test]
fn test_palette_overrides_ansi() {
    let mut ansi = [0u32; 16];
    for (i, c) in ansi.iter_mut().enumerate() {
        *c = 0x00_11_22_33 + i as u32;
    }

    let palette = create_palette_with_ansi(Some(ansi));
    for i in 0..16 {
        assert_eq!(palette[i], 0x00_11_22_33 + i as u32);
    }
}
