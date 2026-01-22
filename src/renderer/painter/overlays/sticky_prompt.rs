use crate::config::FontConfig;
use crate::core::color::Color;

use crate::renderer::color::color_to_u32;
use crate::renderer::font;
use crate::renderer::UiStyle;
use crate::renderer::Renderer;

impl Renderer {
    /// Paint a sticky prompt at the bottom of a pane region.
    #[allow(clippy::too_many_arguments)]
    pub fn paint_sticky_prompt(
        &mut self,
        backbuffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        origin_x: usize,
        origin_y: usize,
        region_w: usize,
        region_h: usize,
        cell_w: usize,
        cell_h: usize,
        font_scale: f32,
        prompt_rows: &[crate::core::grid::RowSnapshot],
        cursor: Option<(usize, usize)>,
        palette: &[u32; 256],
        style: &UiStyle,
        font_config: &FontConfig,
    ) {
        if prompt_rows.is_empty() {
            return;
        }

        let num_prompt_rows = prompt_rows.len();
        let padding_top = cell_h / 2;
        let padding_bottom = cell_h / 2;
        let prompt_height = num_prompt_rows * cell_h + padding_top + padding_bottom;

        if prompt_height > region_h / 2 {
            return;
        }

        let sticky_area_y = origin_y + region_h - prompt_height;
        let prompt_y = sticky_area_y + padding_top;

        let separator_y = sticky_area_y.saturating_sub(1);
        if separator_y >= origin_y && separator_y < origin_y + region_h {
            for x in origin_x..(origin_x + region_w).min(buffer_width) {
                if separator_y < buffer_height {
                    backbuffer[separator_y * buffer_width + x] = style.sticky_prompt_separator;
                }
            }
        }

        let sticky_bg = style.sticky_prompt_bg;
        for y in sticky_area_y..(origin_y + region_h).min(buffer_height) {
            for x in origin_x..(origin_x + region_w).min(buffer_width) {
                backbuffer[y * buffer_width + x] = sticky_bg;
            }
        }

        for (row_idx, row_data) in prompt_rows.iter().enumerate() {
            let cols = row_data.cells.len();
            for col in 0..cols {
                let (ch, fg_color, bg_color) = row_data.cells.get(col).copied().unwrap_or((
                    ' ',
                    Color::Default,
                    Color::Default,
                ));

                let is_cursor = cursor
                    .map(|(r, c)| r == row_idx && c == col)
                    .unwrap_or(false);

                let fg = color_to_u32(fg_color, style.base_fg, palette);
                let bg = color_to_u32(bg_color, sticky_bg, palette);

                let fill_color = if matches!(bg_color, Color::Default) {
                    sticky_bg
                } else {
                    bg
                };

                let (fg, fill_color) = if is_cursor {
                    (fill_color, fg)
                } else {
                    (fg, fill_color)
                };

                let x0 = origin_x + col * cell_w;
                let y0 = prompt_y + row_idx * cell_h;

                let clip_right = (origin_x + region_w).min(buffer_width);
                let clip_bottom = (origin_y + region_h).min(buffer_height);

                if x0 >= clip_right || y0 >= clip_bottom {
                    continue;
                }

                for y in y0..(y0 + cell_h).min(clip_bottom) {
                    for x in x0..(x0 + cell_w).min(clip_right) {
                        backbuffer[y * buffer_width + x] = fill_color;
                    }
                }

                if let Ok(Some(tt_glyph)) = self.font_renderer.get_glyph(ch, font_config) {
                    let baseline_offset = self.font_renderer.baseline_offset(font_config);
                    let glyph_y = y0
                        .saturating_add(baseline_offset as usize)
                        .saturating_sub(tt_glyph.bearing_y as usize);
                    self.draw_native_glyph(
                        backbuffer,
                        buffer_width,
                        buffer_height,
                        x0,
                        glyph_y,
                        &tt_glyph.pixels,
                        tt_glyph.width,
                        tt_glyph.height,
                        fg,
                    );
                } else {
                    let glyph = font::get_bitmap_glyph(ch);
                    self.draw_glyph(
                        backbuffer,
                        buffer_width,
                        buffer_height,
                        origin_x,
                        origin_y,
                        region_w,
                        region_h,
                        font_scale,
                        x0,
                        y0,
                        glyph,
                        fg,
                    );
                }
            }
        }
    }
}
