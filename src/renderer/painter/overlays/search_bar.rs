use crate::config::FontConfig;

use crate::renderer::font;
use crate::renderer::view::TerminalView;
use crate::renderer::UiStyle;
use crate::renderer::Renderer;

impl Renderer {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw_search_bar(
        &mut self,
        buffer: &mut [u32],
        width: usize,
        height: usize,
        origin_x: usize,
        origin_y: usize,
        region_w: usize,
        region_h: usize,
        cell_w: usize,
        cell_h: usize,
        font_scale: f32,
        view: &TerminalView,
        style: &UiStyle,
        font_config: &FontConfig,
    ) {
        let bar_height = cell_h;
        let bar_y = origin_y;
        let clip_right = (origin_x + region_w).min(width);
        let clip_bottom = (origin_y + region_h).min(height);
        let bar_bottom = (bar_y + bar_height).min(clip_bottom);

        let bar_bg = style.search_bar_bg;
        let text_color = style.search_bar_text;
        let match_info_color = style.search_bar_hint_text;

        for y in bar_y..bar_bottom {
            if y >= clip_bottom {
                break;
            }
            let row = y * width;
            for x in origin_x.min(width)..clip_right {
                buffer[row + x] = bar_bg;
            }
        }

        let prefix = "Find: ";
        let mut x_pos = origin_x + cell_w / 2;
        for ch in prefix.chars() {
            if x_pos + cell_w > clip_right {
                break;
            }
            if let Ok(Some(tt_glyph)) = self.font_renderer.get_glyph(ch, font_config) {
                let baseline_offset = self.font_renderer.baseline_offset(font_config);
                let fixed_bearing = self.font_renderer.max_bearing_y(font_config);
                let glyph_y = bar_y
                    .saturating_add(baseline_offset as usize)
                    .saturating_sub(fixed_bearing as usize);
                self.draw_rasterized_glyph(
                    buffer,
                    width,
                    height,
                    origin_x,
                    origin_y,
                    region_w,
                    region_h,
                    x_pos,
                    glyph_y,
                    &tt_glyph.pixels,
                    tt_glyph.width,
                    tt_glyph.height,
                    fixed_bearing,
                    text_color,
                );
            } else {
                let glyph = font::get_bitmap_glyph(ch);
                self.draw_glyph(
                    buffer,
                    width,
                    height,
                    origin_x,
                    origin_y,
                    region_w,
                    region_h,
                    font_scale,
                    x_pos,
                    bar_y,
                    glyph,
                    text_color,
                );
            }
            x_pos += cell_w;
        }

        for ch in view.search.query().chars() {
            if x_pos + cell_w > clip_right.saturating_sub(style.search_right_reserved_px) {
                break;
            }
            if let Ok(Some(tt_glyph)) = self.font_renderer.get_glyph(ch, font_config) {
                let baseline_offset = self.font_renderer.baseline_offset(font_config);
                let fixed_bearing = self.font_renderer.max_bearing_y(font_config) as usize;
                let glyph_y = bar_y
                    .saturating_add(baseline_offset as usize)
                    .saturating_sub(fixed_bearing);
                self.draw_native_glyph(
                    buffer,
                    width,
                    height,
                    x_pos,
                    glyph_y,
                    &tt_glyph.pixels,
                    tt_glyph.width,
                    tt_glyph.height,
                    text_color,
                );
            } else {
                let glyph = font::get_bitmap_glyph(ch);
                self.draw_glyph(
                    buffer,
                    width,
                    height,
                    origin_x,
                    origin_y,
                    region_w,
                    region_h,
                    font_scale,
                    x_pos,
                    bar_y,
                    glyph,
                    text_color,
                );
            }
            x_pos += cell_w;
        }

        let cursor_x = x_pos;
        for y in bar_y..bar_bottom {
            if cursor_x < clip_right {
                buffer[y * width + cursor_x] = text_color;
            }
        }

        let match_count = view.search.match_count();
        let current_idx = view.search.current_match_index();
        let case_indicator = if view.search.is_case_sensitive() { "Aa" } else { "aa" };
        let regex_indicator = if view.is_search_regex() { ".*" } else { "" };

        let match_info = if !view.is_search_regex_valid() {
            format!("[{}{}] invalid regex", case_indicator, regex_indicator)
        } else if match_count > 0 {
            format!(
                "{}/{} [{}{}]",
                current_idx + 1,
                match_count,
                case_indicator,
                regex_indicator
            )
        } else if !view.search.query().is_empty() {
            format!("0/0 [{}{}]", case_indicator, regex_indicator)
        } else {
            format!("[{}{}]", case_indicator, regex_indicator)
        };

        let info_color = if !view.is_search_regex_valid() {
            style.search_invalid_regex_text
        } else {
            match_info_color
        };

        let info_width = match_info.len() * cell_w;
        let info_x = clip_right.saturating_sub(info_width + cell_w);

        let mut x_pos = info_x;
        for ch in match_info.chars() {
            if x_pos + cell_w > clip_right {
                break;
            }
            if let Ok(Some(tt_glyph)) = self.font_renderer.get_glyph(ch, font_config) {
                let baseline_offset = self.font_renderer.baseline_offset(font_config);
                let fixed_bearing = self.font_renderer.max_bearing_y(font_config) as usize;
                let glyph_y = bar_y
                    .saturating_add(baseline_offset as usize)
                    .saturating_sub(fixed_bearing);
                self.draw_native_glyph(
                    buffer,
                    width,
                    height,
                    x_pos,
                    glyph_y,
                    &tt_glyph.pixels,
                    tt_glyph.width,
                    tt_glyph.height,
                    info_color,
                );
            } else {
                let glyph = font::get_bitmap_glyph(ch);
                self.draw_glyph(
                    buffer,
                    width,
                    height,
                    origin_x,
                    origin_y,
                    region_w,
                    region_h,
                    font_scale,
                    x_pos,
                    bar_y,
                    glyph,
                    info_color,
                );
            }
            x_pos += cell_w;
        }
    }
}
