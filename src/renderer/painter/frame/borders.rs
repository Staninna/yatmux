use crate::core::color::Color;

use crate::renderer::font::{self, tab_indicator_glyph};
use crate::renderer::color::{color_to_u32, lighten_color};
use crate::renderer::UiStyle;
use super::super::primitives::contrast_color;
use super::Renderer;
use crate::config::FontConfig;

impl Renderer {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_cell(
        &mut self,
        backbuffer: &mut [u32],
        width: usize,
        height: usize,
        origin_x: usize,
        origin_y: usize,
        region_w: usize,
        region_h: usize,
        cell_w: usize,
        cell_h: usize,
        font_scale: f32,
        row: usize,
        col: usize,
        ch: char,
        invert: bool,
        fg_color: Color,
        bg_color: Color,
        palette: &[u32; 256],
        tab_info: Option<(usize, usize)>,
        selected: bool,
        is_url: bool,
        is_url_hovered: bool,
        hex_bg: Option<u32>,
        search_match: Option<bool>,
        style: &UiStyle,
        font_config: &FontConfig,
    ) {
        let fg = color_to_u32(fg_color, style.base_fg, palette);
        let bg = color_to_u32(bg_color, style.base_bg, palette);

        let mut fill_color = match search_match {
            Some(true) => style.search_current_bg,
            Some(false) => style.search_match_bg,
            None if selected || tab_info.is_some() => lighten_color(bg),
            None => bg,
        };

        let mut fg = fg;
        if matches!(search_match, None) && !selected && tab_info.is_none() {
            if let Some(hex) = hex_bg {
                fill_color = hex;
                fg = contrast_color(hex);
            }
        }

        let (fg, fill_color) = if invert { (fill_color, fg) } else { (fg, fill_color) };

        let fg = if is_url { style.url_fg } else { fg };

        let x0 = origin_x + col * cell_w;
        let y0 = origin_y + row * cell_h;

        let clip_right = (origin_x + region_w).min(width);
        let clip_bottom = (origin_y + region_h).min(height);

        if x0 >= clip_right || y0 >= clip_bottom {
            return;
        }

        for y in y0..(y0 + cell_h).min(clip_bottom) {
            for x in x0..(x0 + cell_w).min(clip_right) {
                backbuffer[y * width + x] = fill_color;
            }
        }

        if let Some((start_col, _)) = tab_info {
            if start_col == col {
                let tab_fg = lighten_color(fg);
                self.draw_glyph(
                    backbuffer,
                    width,
                    height,
                    origin_x,
                    origin_y,
                    region_w,
                    region_h,
                    font_scale,
                    x0,
                    y0,
                    tab_indicator_glyph(),
                    tab_fg,
                );
            }
            return;
        }

        match self.font_renderer.get_glyph(ch, font_config) {
            Ok(Some(tt_glyph)) if tt_glyph.height > 0 && tt_glyph.width > 0 => {
                let baseline = self.font_renderer.baseline_offset(font_config) as usize;
                let glyph_top = y0 + baseline.saturating_sub(tt_glyph.bearing_y as usize);
                self.draw_native_glyph(
                    backbuffer,
                    width,
                    height,
                    x0,
                    glyph_top,
                    &tt_glyph.pixels,
                    tt_glyph.width,
                    tt_glyph.height,
                    fg,
                );
            }
            _ => {
                let glyph = font::get_bitmap_glyph(ch);
                self.draw_glyph(
                    backbuffer,
                    width,
                    height,
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

        if is_url_hovered {
            self.draw_url_hover_underline(
                backbuffer,
                width,
                clip_right,
                clip_bottom,
                x0,
                y0,
                cell_w,
                cell_h,
                fg,
            );
        }
    }
}
