use crate::app::App;

impl App {
    /// Helper to blend two colors with given alpha (0-255)
    pub(super) fn blend_colors(fg: u32, bg: u32, alpha: u8) -> u32 {
        let fg_r = ((fg >> 16) & 0xFF) as u8;
        let fg_g = ((fg >> 8) & 0xFF) as u8;
        let fg_b = (fg & 0xFF) as u8;

        let bg_r = ((bg >> 16) & 0xFF) as u8;
        let bg_g = ((bg >> 8) & 0xFF) as u8;
        let bg_b = (bg & 0xFF) as u8;

        let t = alpha as f32 / 255.0;
        let r = (bg_r as f32 + (fg_r as f32 - bg_r as f32) * t) as u8;
        let g = (bg_g as f32 + (fg_g as f32 - bg_g as f32) * t) as u8;
        let b = (bg_b as f32 + (fg_b as f32 - bg_b as f32) * t) as u8;

        ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
    }
}
