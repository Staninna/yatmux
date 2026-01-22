use super::super::layout::{HelpLine, HELP_KEY_GAP_CELLS, HELP_LINE_INDENT_CELLS};
use crate::config::FontConfig;
use crate::renderer::font;
use crate::renderer::UiStyle;
use crate::renderer::Renderer;

impl Renderer {
    pub(super) fn draw_help_scrollbar(
        &self,
        backbuffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        origin_x: usize,
        origin_y: usize,
        box_w: usize,
        box_h: usize,
        scroll: usize,
        scroll_total: usize,
        scroll_visible: usize,
        style: &UiStyle,
    ) {
        let bar_width = 2usize;
        let bar_x = origin_x + box_w.saturating_sub(bar_width);
        let bar_y = origin_y;
        let bar_h = box_h;

        let max_scroll = scroll_total.saturating_sub(scroll_visible);
        let ratio = if scroll_total > 0 {
            scroll_visible as f32 / scroll_total as f32
        } else {
            1.0
        };
        let thumb_h = (bar_h as f32 * ratio).round() as usize;
        let thumb_h = thumb_h.max(8).min(bar_h);

        let thumb_y = if max_scroll == 0 {
            bar_y
        } else {
            bar_y + (scroll as f32 / max_scroll as f32 * (bar_h - thumb_h) as f32).round() as usize
        };

        let bar_bg = style.help_bg;
        let bar_fg = style.help_footer_text;

        for y in bar_y..(bar_y + bar_h).min(buffer_height) {
            let row = y * buffer_width;
            for x in bar_x..(bar_x + bar_width).min(buffer_width) {
                let color = if y >= thumb_y && y < thumb_y + thumb_h {
                    bar_fg
                } else {
                    bar_bg
                };
                backbuffer[row + x] = color;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_help_line(
        &mut self,
        backbuffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        x0: usize,
        x1: usize,
        y: usize,
        line: &HelpLine,
        text_color: u32,
        key_color: u32,
        font_config: &FontConfig,
        cell_w: usize,
        font_scale: f32,
        key_col_width: usize,
    ) {
        match line {
            HelpLine::Header(text) => {
                self.draw_help_text_at(
                    backbuffer,
                    buffer_width,
                    buffer_height,
                    x0,
                    x1,
                    y,
                    text,
                    text_color,
                    font_config,
                    cell_w,
                    font_scale,
                );
            }
            HelpLine::Spacer => {}
            HelpLine::Item { key, action } => {
                let key_text = format!("{:<width$}", key, width = key_col_width);
                let key_x0 = x0.saturating_add(HELP_LINE_INDENT_CELLS * cell_w);
                let action_x0 = key_x0
                    .saturating_add(key_col_width * cell_w)
                    .saturating_add(HELP_KEY_GAP_CELLS * cell_w);

                self.draw_help_text_at(
                    backbuffer,
                    buffer_width,
                    buffer_height,
                    key_x0,
                    x1,
                    y,
                    &key_text,
                    key_color,
                    font_config,
                    cell_w,
                    font_scale,
                );
                self.draw_help_text_at(
                    backbuffer,
                    buffer_width,
                    buffer_height,
                    action_x0,
                    x1,
                    y,
                    action,
                    text_color,
                    font_config,
                    cell_w,
                    font_scale,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_help_text_at(
        &mut self,
        backbuffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        x0: usize,
        x1: usize,
        y: usize,
        text: &str,
        text_color: u32,
        font_config: &FontConfig,
        cell_w: usize,
        font_scale: f32,
    ) {
        let mut x = x0;
        for ch in text.chars() {
            if x + cell_w > x1 {
                break;
            }
            if let Ok(Some(tt_glyph)) = self.font_renderer.get_glyph(ch, font_config) {
                let baseline_offset = self.font_renderer.baseline_offset(font_config);
                let glyph_y = y
                    .saturating_add(baseline_offset as usize)
                    .saturating_sub(tt_glyph.bearing_y as usize);
                self.draw_native_glyph(
                    backbuffer,
                    buffer_width,
                    buffer_height,
                    x,
                    glyph_y,
                    &tt_glyph.pixels,
                    tt_glyph.width,
                    tt_glyph.height,
                    text_color,
                );
            } else {
                let glyph = font::get_bitmap_glyph(ch);
                self.draw_glyph_help(
                    backbuffer,
                    buffer_width,
                    buffer_height,
                    x0,
                    0,
                    x1.saturating_sub(x0),
                    buffer_height,
                    font_scale,
                    x,
                    y,
                    glyph,
                    text_color,
                );
            }
            x += cell_w;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_glyph_help(
        &self,
        backbuffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        origin_x: usize,
        origin_y: usize,
        box_w: usize,
        box_h: usize,
        font_scale: f32,
        x0: usize,
        y0: usize,
        glyph: [u8; 8],
        color: u32,
    ) {
        let clip_right = (origin_x + box_w).min(buffer_width);
        let clip_bottom = (origin_y + box_h).min(buffer_height);

        let font_scale = self.font_renderer.quantize_scale(font_scale);

        for gy in 0..8 {
            let bits = glyph[gy];
            for gx in 0..8 {
                let on = (bits >> gx) & 1 == 1;
                if !on {
                    continue;
                }

                for sy in 0..font_scale {
                    for sx in 0..font_scale {
                        let x = x0 + gx * font_scale + sx;
                        let y = y0 + gy * font_scale + sy;
                        if x < clip_right && y < clip_bottom {
                            backbuffer[y * buffer_width + x] = color;
                        }
                    }
                }
            }
        }
    }
}
