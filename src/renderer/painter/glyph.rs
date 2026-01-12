use crate::constants::{GLYPH_H, GLYPH_W};

use super::Renderer;

impl Renderer {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_glyph(
        &self,
        backbuffer: &mut [u32],
        width: usize,
        height: usize,
        origin_x: usize,
        origin_y: usize,
        region_w: usize,
        region_h: usize,
        font_scale: usize,
        x0: usize,
        y0: usize,
        glyph: [u8; 8],
        color: u32,
    ) {
        let clip_right = (origin_x + region_w).min(width);
        let clip_bottom = (origin_y + region_h).min(height);

        let font_scale = font_scale.clamp(1, 8);

        for gy in 0..GLYPH_H {
            let bits = glyph[gy];
            for gx in 0..GLYPH_W {
                let on = (bits >> gx) & 1 == 1;
                if !on {
                    continue;
                }

                for sy in 0..font_scale {
                    for sx in 0..font_scale {
                        let x = x0 + gx * font_scale + sx;
                        let y = y0 + gy * font_scale + sy;
                        if x < clip_right && y < clip_bottom {
                            backbuffer[y * width + x] = color;
                        }
                    }
                }
            }
        }
    }
}
