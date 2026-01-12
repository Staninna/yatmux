use vt100::Color;

use super::Renderer;

use super::super::color::color_to_u32;
use super::super::font;
use super::super::help;
use super::super::view::TerminalView;
use super::super::{HelpSection, UiStyle};

impl Renderer {
    pub fn paint_help_overlay(
        &self,
        backbuffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        title: &str,
        sections: &[HelpSection],
        scroll_offset: usize,
        accent_color: u32,
        font_scale: usize,
        shell_integration_detected: bool,
        style: &UiStyle,
    ) -> (usize, usize) {
        help::paint_help_overlay(
            backbuffer,
            buffer_width,
            buffer_height,
            title,
            sections,
            scroll_offset,
            accent_color,
            font_scale,
            shell_integration_detected,
            style.help_bg,
            style.help_text,
            style.help_footer_text,
            style.help_padding_x_cells,
            style.help_padding_y_cells,
        )
    }

    /// Paint a sticky prompt at the bottom of a pane region.
    #[allow(clippy::too_many_arguments)]
    pub fn paint_sticky_prompt(
        &self,
        backbuffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        origin_x: usize,
        origin_y: usize,
        region_w: usize,
        region_h: usize,
        cell_w: usize,
        cell_h: usize,
        font_scale: usize,
        prompt_rows: &[crate::core::grid::RowSnapshot],
        cursor: Option<(usize, usize)>,
        palette: &[u32; 256],
        style: &UiStyle,
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

        // Draw separator line
        let separator_y = sticky_area_y.saturating_sub(1);
        if separator_y >= origin_y && separator_y < origin_y + region_h {
            for x in origin_x..(origin_x + region_w).min(buffer_width) {
                if separator_y < buffer_height {
                    backbuffer[separator_y * buffer_width + x] = style.sticky_prompt_separator;
                }
            }
        }

        // Draw background
        let sticky_bg = style.sticky_prompt_bg;
        for y in sticky_area_y..(origin_y + region_h).min(buffer_height) {
            for x in origin_x..(origin_x + region_w).min(buffer_width) {
                backbuffer[y * buffer_width + x] = sticky_bg;
            }
        }

        // Draw each prompt row
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

                let glyph = font::get_glyph(ch);
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

    pub(super) fn draw_search_bar(
        &self,
        buffer: &mut [u32],
        width: usize,
        height: usize,
        origin_x: usize,
        origin_y: usize,
        region_w: usize,
        region_h: usize,
        cell_w: usize,
        cell_h: usize,
        font_scale: usize,
        view: &TerminalView,
        style: &UiStyle,
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
            let glyph = font::get_glyph(ch);
            self.draw_glyph(
                buffer, width, height, origin_x, origin_y, region_w, region_h, font_scale, x_pos,
                bar_y, glyph, text_color,
            );
            x_pos += cell_w;
        }

        for ch in view.search.query().chars() {
            if x_pos + cell_w > clip_right.saturating_sub(style.search_right_reserved_px) {
                break;
            }
            let glyph = font::get_glyph(ch);
            self.draw_glyph(
                buffer, width, height, origin_x, origin_y, region_w, region_h, font_scale, x_pos,
                bar_y, glyph, text_color,
            );
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
        let case_indicator = if view.search.is_case_sensitive() {
            "Aa"
        } else {
            "aa"
        };
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

        // Use red color for invalid regex
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
            let glyph = font::get_glyph(ch);
            self.draw_glyph(
                buffer, width, height, origin_x, origin_y, region_w, region_h, font_scale, x_pos,
                bar_y, glyph, info_color,
            );
            x_pos += cell_w;
        }
    }
}
