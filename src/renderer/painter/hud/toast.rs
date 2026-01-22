use crate::config::FontConfig;

use crate::renderer::font;
use crate::renderer::UiStyle;
use crate::renderer::Renderer;

impl Renderer {
    /// Paints a small toast notification at the bottom-center of the screen.
    pub fn paint_toast(
        &mut self,
        buffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        message: &str,
        style: &UiStyle,
        font_config: &FontConfig,
    ) {
        let text_len = message.chars().count().max(1);
        let (font_scale, cell_w, cell_h) =
            if let Some(override_scale) = style.toast_font_scale_override {
                let scale = self.font_renderer.clamp_scale(override_scale);
                let mut probe_font = font_config.clone();
                probe_font.scale = scale;
                let (cw, ch) = self.font_renderer.cell_size(&probe_font);
                (scale, cw.max(1), ch.max(1))
            } else {
                self.choose_toast_scale(
                    text_len,
                    style.toast_font_scale_max,
                    style.toast_bottom_margin_cells,
                    buffer_width,
                    buffer_height,
                    font_config,
                )
            };

        let mut toast_font_config = font_config.clone();
        toast_font_config.scale = font_scale;

        let padding_x = cell_w;
        let padding_y = cell_h / 2;

        let toast_width = text_len * cell_w + padding_x * 2;
        let toast_height = cell_h + padding_y * 2;

        // Position: bottom-center, slightly above the bottom edge
        let toast_x = (buffer_width.saturating_sub(toast_width)) / 2;
        let toast_y =
            buffer_height.saturating_sub(toast_height + cell_h * style.toast_bottom_margin_cells);

        let bg_color = style.toast_bg;
        let fg_color = style.toast_text;
        let border_color = style.toast_border;

        // Draw background
        for y in toast_y..toast_y + toast_height {
            if y >= buffer_height {
                continue;
            }
            for x in toast_x..toast_x + toast_width {
                if x >= buffer_width {
                    continue;
                }
                buffer[y * buffer_width + x] = bg_color;
            }
        }

        // Draw border (single pixel)
        for x in toast_x..toast_x + toast_width {
            if x < buffer_width {
                if toast_y > 0 && toast_y - 1 < buffer_height {
                    buffer[(toast_y) * buffer_width + x] = border_color;
                }
                if toast_y + toast_height - 1 < buffer_height {
                    buffer[(toast_y + toast_height - 1) * buffer_width + x] = border_color;
                }
            }
        }
        for y in toast_y..toast_y + toast_height {
            if y < buffer_height {
                if toast_x < buffer_width {
                    buffer[y * buffer_width + toast_x] = border_color;
                }
                if toast_x + toast_width - 1 < buffer_width {
                    buffer[y * buffer_width + toast_x + toast_width - 1] = border_color;
                }
            }
        }

        // Draw text
        let text_x = toast_x + padding_x;
        let text_y = toast_y + padding_y;

        for (i, ch) in message.chars().enumerate() {
            let x = text_x + i * cell_w;
            if x + cell_w > buffer_width {
                break;
            }
            if let Ok(Some(tt_glyph)) = self.font_renderer.get_glyph(ch, &toast_font_config) {
                let baseline_offset = self.font_renderer.baseline_offset(&toast_font_config);
                let glyph_y = text_y
                    .saturating_add(baseline_offset as usize)
                    .saturating_sub(tt_glyph.bearing_y as usize);
                self.draw_native_glyph(
                    buffer,
                    buffer_width,
                    buffer_height,
                    x,
                    glyph_y,
                    &tt_glyph.pixels,
                    tt_glyph.width,
                    tt_glyph.height,
                    fg_color,
                );
            } else {
                let glyph = font::get_bitmap_glyph(ch);
                self.draw_glyph(
                    buffer,
                    buffer_width,
                    buffer_height,
                    0,
                    0,
                    buffer_width,
                    buffer_height,
                    font_scale,
                    x,
                    text_y,
                    glyph,
                    fg_color,
                );
            }
        }
    }

    fn choose_toast_scale(
        &mut self,
        message_len: usize,
        max_scale: f32,
        bottom_margin_cells: usize,
        buffer_width: usize,
        buffer_height: usize,
        font_config: &FontConfig,
    ) -> (f32, usize, usize) {
        let max_scale = max_scale.max(1.0);
        let max_allowed_width = buffer_width.saturating_sub(buffer_width / 4); // keep toast narrower

        // Try scales from max down to 1.0 in 0.25 steps
        let mut scale = max_scale;
        while scale >= 1.0 {
            let mut probe_font = font_config.clone();
            probe_font.scale = scale;
            let (cell_w, cell_h) = self.font_renderer.cell_size(&probe_font);
            if cell_w == 0 || cell_h == 0 {
                continue;
            }

            let padding_x = cell_w;
            let padding_y = cell_h / 2;
            let toast_width = message_len * cell_w + padding_x * 2;
            let toast_height = cell_h + padding_y * 2;
            let margin_height = cell_h * bottom_margin_cells;

            if toast_width <= max_allowed_width && toast_height + margin_height <= buffer_height {
                return (scale, cell_w, cell_h);
            }

            scale -= 0.25;
        }

        let mut probe_font = font_config.clone();
        probe_font.scale = 1.0;
        let (cell_w, cell_h) = self.font_renderer.cell_size(&probe_font);
        (1.0, cell_w.max(1), cell_h.max(1))
    }
}
