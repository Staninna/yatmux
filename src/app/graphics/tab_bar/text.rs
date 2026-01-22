use crate::app::App;

impl App {
    /// Helper to draw a glyph with proper alpha blending
    pub(super) fn draw_glyph_with_alpha(
        buffer: &mut [u32],
        buffer_width: usize,
        max_height: usize,
        x0: usize,
        y0: usize,
        glyph_data: &[u8],
        glyph_width: usize,
        glyph_height: usize,
        color: u32,
    ) {
        let (r, g, b) = (
            ((color >> 16) & 0xFF) as u8,
            ((color >> 8) & 0xFF) as u8,
            (color & 0xFF) as u8,
        );

        for gy in 0..glyph_height {
            let y = y0 + gy;
            if y >= max_height {
                break;
            }

            for gx in 0..glyph_width {
                let x = x0 + gx;
                if x >= buffer_width {
                    break;
                }

                let alpha = glyph_data[gy * glyph_width + gx];
                if alpha > 0 {
                    let buffer_idx = y * buffer_width + x;

                    if alpha == 255 {
                        buffer[buffer_idx] = color;
                    } else {
                        // Proper alpha blending
                        let existing = buffer[buffer_idx];
                        let (er, eg, eb) = (
                            ((existing >> 16) & 0xFF) as u8,
                            ((existing >> 8) & 0xFF) as u8,
                            (existing & 0xFF) as u8,
                        );

                        let t = alpha as f32 / 255.0;
                        let nr = (er as f32 + (r as f32 - er as f32) * t) as u8;
                        let ng = (eg as f32 + (g as f32 - eg as f32) * t) as u8;
                        let nb = (eb as f32 + (b as f32 - eb as f32) * t) as u8;

                        buffer[buffer_idx] =
                            ((nr as u32) << 16) | ((ng as u32) << 8) | (nb as u32);
                    }
                }
            }
        }
    }
}
