/// Compute a high-contrast foreground for a given background.
pub(super) fn contrast_color(bg: u32) -> u32 {
    fn srgb_channel_to_linear(v: f32) -> f32 {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    }

    fn relative_luminance(rgb: u32) -> f32 {
        let r = ((rgb >> 16) & 0xFF) as f32 / 255.0;
        let g = ((rgb >> 8) & 0xFF) as f32 / 255.0;
        let b = (rgb & 0xFF) as f32 / 255.0;

        let r = srgb_channel_to_linear(r);
        let g = srgb_channel_to_linear(g);
        let b = srgb_channel_to_linear(b);

        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    // Use a WCAG-ish cutoff; doesn't need to be perfect.
    if relative_luminance(bg) < 0.5 {
        0xFFFFFF
    } else {
        0x000000
    }
}
