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
        font_scale: f32,
        x0: usize,
        y0: usize,
        glyph: [u8; 8],
        color: u32,
    ) {
        let clip_right = (origin_x + region_w).min(width);
        let clip_bottom = (origin_y + region_h).min(height);

        let font_scale = self.font_renderer.quantize_scale(font_scale);

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

    #[allow(dead_code)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_rasterized_glyph(
        &self,
        backbuffer: &mut [u32],
        width: usize,
        height: usize,
        origin_x: usize,
        origin_y: usize,
        region_w: usize,
        region_h: usize,
        x0: usize,
        y0: usize,
        glyph_data: &[u8],
        glyph_width: usize,
        glyph_height: usize,
        _bearing_y: i32,
        color: u32,
    ) {
        let clip_right = (origin_x + region_w).min(width);
        let clip_bottom = (origin_y + region_h).min(height);

        let (r, g, b) = (
            ((color >> 16) & 0xFF) as u8,
            ((color >> 8) & 0xFF) as u8,
            (color & 0xFF) as u8,
        );

        for gy in 0..glyph_height {
            let y = y0 + gy;
            if y >= clip_bottom {
                break;
            }

            for gx in 0..glyph_width {
                let x = x0 + gx;
                if x >= clip_right {
                    break;
                }

                let alpha = glyph_data[gy * glyph_width + gx];

                if alpha > 0 {
                    let buffer_idx = y * width + x;
                    let existing = backbuffer[buffer_idx];

                    if alpha == 255 {
                        backbuffer[buffer_idx] = color;
                    } else {
                        let (er, eg, eb) = (
                            ((existing >> 16) & 0xFF) as u8,
                            ((existing >> 8) & 0xFF) as u8,
                            (existing & 0xFF) as u8,
                        );

                        let t = alpha as f32 / 255.0;
                        let nr = (er as f32 + (r as f32 - er as f32) * t) as u8;
                        let ng = (eg as f32 + (g as f32 - eg as f32) * t) as u8;
                        let nb = (eb as f32 + (b as f32 - eb as f32) * t) as u8;

                        backbuffer[buffer_idx] =
                            ((nr as u32) << 16) | ((ng as u32) << 8) | (nb as u32);
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_scaled_glyph(
        &self,
        backbuffer: &mut [u32],
        width: usize,
        height: usize,
        x0: usize,
        y0: usize,
        target_w: usize,
        target_h: usize,
        glyph_data: &[u8],
        glyph_width: usize,
        glyph_height: usize,
        color: u32,
    ) {
        if glyph_width == 0 || glyph_height == 0 {
            return;
        }

        // Simple nearest-neighbor scaling
        for ty in 0..target_h {
            let src_y = (ty * glyph_height / target_h).min(glyph_height - 1);
            let y = y0 + ty;
            if y >= height {
                break;
            }

            for tx in 0..target_w {
                let src_x = (tx * glyph_width / target_w).min(glyph_width - 1);
                let x = x0 + tx;
                if x >= width {
                    break;
                }

                let alpha = glyph_data[src_y * glyph_width + src_x];

                if alpha > 128 {
                    backbuffer[y * width + x] = color;
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_native_glyph(
        &self,
        backbuffer: &mut [u32],
        width: usize,
        height: usize,
        x0: usize,
        y0: usize,
        glyph_data: &[u8],
        glyph_width: usize,
        glyph_height: usize,
        color: u32,
    ) {
        if glyph_width == 0 || glyph_height == 0 {
            return;
        }

        let (r, g, b) = (
            ((color >> 16) & 0xFF) as u8,
            ((color >> 8) & 0xFF) as u8,
            (color & 0xFF) as u8,
        );

        for gy in 0..glyph_height {
            let y = y0 + gy;
            if y >= height {
                break;
            }

            for gx in 0..glyph_width {
                let x = x0 + gx;
                if x >= width {
                    break;
                }

                let alpha = glyph_data[gy * glyph_width + gx];

                if alpha > 0 {
                    let buffer_idx = y * width + x;
                    let existing = backbuffer[buffer_idx];

                    if alpha == 255 {
                        backbuffer[buffer_idx] = color;
                    } else {
                        let (er, eg, eb) = (
                            ((existing >> 16) & 0xFF) as u8,
                            ((existing >> 8) & 0xFF) as u8,
                            (existing & 0xFF) as u8,
                        );

                        let t = alpha as f32 / 255.0;
                        let nr = (er as f32 + (r as f32 - er as f32) * t) as u8;
                        let ng = (eg as f32 + (g as f32 - eg as f32) * t) as u8;
                        let nb = (eb as f32 + (b as f32 - eb as f32) * t) as u8;

                        backbuffer[buffer_idx] =
                            ((nr as u32) << 16) | ((ng as u32) << 8) | (nb as u32);
                    }
                }
            }
        }
    }
}
