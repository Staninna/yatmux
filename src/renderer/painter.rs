//! Pixel painting for terminal rendering.
//!
//! `Renderer` is responsible for painting `RenderFrame`s to pixel buffers.
//! It is intentionally stateless - all interactive state lives in `TerminalView`.

use std::sync::Arc;

use anyhow::Result;
use softbuffer::Surface;
use vt100::Color;

use crate::constants::{DEFAULT_BG_COLOR, DEFAULT_FG_COLOR, GLYPH_H, GLYPH_W};
use crate::terminal::Terminal;

use super::color::{color_to_u32, lighten_color};
use super::font::{self, tab_indicator_glyph};
use super::help;
use super::view::TerminalView;
use super::{HelpSection, RenderFrame, SEARCH_CURRENT_BG, SEARCH_MATCH_BG};

/// Compute a high-contrast foreground for a given background.
fn contrast_color(bg: u32) -> u32 {
    fn srgb_channel_to_linear(v: f32) -> f32 {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    }

    fn relative_luminance(rgb: u32) -> f32 {
        let r = ((rgb >> 16) & 0xFF) as f32 / 255.0;
        let g = ((rgb >> 8) & 0xFF) as f32 / 255.0;
        let b = (rgb & 0xFF) as f32 / 255.0;

        let r = srgb_channel_to_linear(r);
        let g = srgb_channel_to_linear(g);
        let b = srgb_channel_to_linear(b);

        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    // Use a WCAG-ish cutoff; doesn't need to be perfect.
    if relative_luminance(bg) < 0.5 {
        0xFFFFFF
    } else {
        0x000000
    }
}

/// Pixel-paints a `RenderFrame` to the window surface.
///
/// All interactive state lives in `TerminalView`; this type is intentionally
/// stateless.
pub struct Renderer;

impl Default for Renderer {
    fn default() -> Self {
        Renderer::new()
    }
}

impl Renderer {
    pub fn new() -> Self {
        Renderer
    }

    /// Paint a terminal into a region of an existing backbuffer.
    #[allow(clippy::too_many_arguments)]
    pub fn paint_terminal_region(
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
        terminal: &Terminal,
        palette: &Arc<[u32; 256]>,
        view: &mut TerminalView,
    ) -> Result<()> {
        let cell_w = cell_w.max(1);
        let cell_h = cell_h.max(1);
        let font_scale = font_scale.clamp(1, 8);

        if region_w < cell_w || region_h < cell_h {
            return Ok(());
        }

        let rows = region_h / cell_h;
        let cols = region_w / cell_w;
        view.set_dimensions(rows, cols);

        let frame = view.build_frame(terminal, rows, cols)?;
        self.paint_frame(
            backbuffer,
            buffer_width,
            buffer_height,
            origin_x,
            origin_y,
            region_w,
            region_h,
            cell_w,
            cell_h,
            font_scale,
            &frame,
            palette,
            view,
        );

        Ok(())
    }

    pub fn render(
        &self,
        surface: &mut Surface<winit::event_loop::OwnedDisplayHandle, winit::window::Window>,
        terminal: &Terminal,
        palette: &Arc<[u32; 256]>,
        view: &mut TerminalView,
        font_scale: usize,
    ) -> Result<()> {
        let mut buffer = surface
            .buffer_mut()
            .map_err(|e| anyhow::anyhow!("softbuffer buffer_mut failed: {e:?}"))?;

        let buffer_width = buffer.width().get() as usize;
        let buffer_height = buffer.height().get() as usize;
        buffer.fill(DEFAULT_BG_COLOR);

        let font_scale = font_scale.clamp(1, 8);
        let cell_w = 8 * font_scale;
        let cell_h = 8 * font_scale;

        self.paint_terminal_region(
            &mut buffer,
            buffer_width,
            buffer_height,
            0,
            0,
            buffer_width,
            buffer_height,
            cell_w,
            cell_h,
            font_scale,
            terminal,
            palette,
            view,
        )?;

        buffer
            .present()
            .map_err(|e| anyhow::anyhow!("softbuffer present failed: {e:?}"))?;

        Ok(())
    }

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
                    backbuffer[separator_y * buffer_width + x] = 0x444444;
                }
            }
        }

        // Draw background
        let sticky_bg = 0x1A1A1A;
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

                let fg = color_to_u32(fg_color, DEFAULT_FG_COLOR, palette);
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

    fn paint_frame(
        &self,
        buffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        origin_x: usize,
        origin_y: usize,
        region_w: usize,
        region_h: usize,
        cell_w: usize,
        cell_h: usize,
        font_scale: usize,
        frame: &RenderFrame,
        palette: &[u32; 256],
        view: &TerminalView,
    ) {
        for (row_idx, row_data) in frame.display_rows.iter().enumerate().take(frame.rows) {
            for col in 0..frame.cols {
                let (ch, fg, bg) = row_data.cells.get(col).copied().unwrap_or((
                    ' ',
                    Color::Default,
                    Color::Default,
                ));
                let invert = frame.show_cursor && (row_idx as u16, col as u16) == frame.cursor;
                let tab_info = row_data.tabs.get(col).copied().flatten();
                let selected = view.selection.is_selected(row_idx, col);
                let is_url = view.urls.is_url(row_idx, col);
                let is_url_hovered = view.urls.is_hovered(row_idx, col);
                let hex_bg = view.color_codes.color_at(row_idx, col);

                let search_match = view.search.is_match(row_idx, col, frame.view_start);

                self.draw_cell(
                    buffer,
                    buffer_width,
                    buffer_height,
                    origin_x,
                    origin_y,
                    region_w,
                    region_h,
                    cell_w,
                    cell_h,
                    font_scale,
                    row_idx,
                    col,
                    ch,
                    invert,
                    fg,
                    bg,
                    palette,
                    tab_info,
                    selected,
                    is_url,
                    is_url_hovered,
                    hex_bg,
                    search_match,
                );
            }
        }

        if view.search.is_active() {
            self.draw_search_bar(
                buffer,
                buffer_width,
                buffer_height,
                origin_x,
                origin_y,
                region_w,
                region_h,
                cell_w,
                cell_h,
                font_scale,
                view,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_cell(
        &self,
        backbuffer: &mut [u32],
        width: usize,
        height: usize,
        origin_x: usize,
        origin_y: usize,
        region_w: usize,
        region_h: usize,
        cell_w: usize,
        cell_h: usize,
        font_scale: usize,
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
    ) {
        let fg = color_to_u32(fg_color, DEFAULT_FG_COLOR, palette);
        let bg = color_to_u32(bg_color, DEFAULT_BG_COLOR, palette);

        let mut fill_color = match search_match {
            Some(true) => SEARCH_CURRENT_BG,
            Some(false) => SEARCH_MATCH_BG,
            None if selected || tab_info.is_some() => lighten_color(bg),
            None => bg,
        };

        // If this cell is part of a hex color literal, render it like a "swatch"
        // by using the hex value as the background color, but only when not
        // overridden by selection/search highlighting.
        let mut fg = fg;
        if matches!(search_match, None) && !selected && tab_info.is_none() {
            if let Some(hex) = hex_bg {
                fill_color = hex;
                fg = contrast_color(hex);
            }
        }

        // Handle cursor inversion after picking final fg/bg.
        let (fg, fill_color) = if invert {
            (fill_color, fg)
        } else {
            (fg, fill_color)
        };

        let fg = if is_url { 0x6699FF } else { fg };

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
                    x0,
                    y0,
                    font_scale,
                    tab_indicator_glyph(),
                    tab_fg,
                );
            }
            return;
        }

        let glyph = font::get_glyph(ch);
        self.draw_glyph(
            backbuffer, width, height, origin_x, origin_y, region_w, region_h, font_scale, x0, y0,
            glyph, fg,
        );

        if is_url_hovered {
            let underline_y = y0 + cell_h - 2;
            if underline_y < clip_bottom {
                for x in x0..(x0 + cell_w).min(clip_right) {
                    backbuffer[underline_y * width + x] = fg;
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_glyph(
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

    fn draw_search_bar(
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
    ) {
        let bar_height = cell_h;
        let bar_y = origin_y + region_h.saturating_sub(bar_height);
        let clip_right = (origin_x + region_w).min(width);
        let clip_bottom = (origin_y + region_h).min(height);

        let bar_bg = 0x333333;
        let text_color = 0xFFFFFF;
        let match_info_color = 0xAAAAAA;

        for y in bar_y.min(clip_bottom)..clip_bottom {
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
            if x_pos + cell_w > clip_right.saturating_sub(100) {
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
        for y in bar_y.min(clip_bottom)..(bar_y + cell_h).min(clip_bottom) {
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
            0xFF6666
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

    /// Paints a small toast notification at the bottom-center of the screen.
    pub fn paint_toast(
        &self,
        buffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        message: &str,
        font_scale: usize,
    ) {
        let scale = font_scale.clamp(1, 8);
        let cell_w = GLYPH_W * scale;
        let cell_h = GLYPH_H * scale;

        let text_len = message.chars().count();
        let padding_x = cell_w;
        let padding_y = cell_h / 2;

        let toast_width = text_len * cell_w + padding_x * 2;
        let toast_height = cell_h + padding_y * 2;

        // Position: bottom-center, slightly above the bottom edge
        let toast_x = (buffer_width.saturating_sub(toast_width)) / 2;
        let toast_y = buffer_height.saturating_sub(toast_height + cell_h * 2);

        // Semi-transparent dark background
        let bg_color = 0x2A2A2A;
        let fg_color = 0xCCCCCC;
        let border_color = 0x444444;

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
            let glyph = font::get_glyph(ch);
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

    /// Paints the shadow prompt at the bottom of the screen.
    /// Shows a prompt indicator (e.g. "$") and the buffered input with cursor.
    pub fn paint_shadow_prompt(
        &self,
        buffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        input: &str,
        cursor_pos: usize,
        font_scale: usize,
    ) {
        let scale = font_scale.clamp(1, 8);
        let cell_w = GLYPH_W * scale;
        let cell_h = GLYPH_H * scale;

        // Prompt indicator
        let prompt_indicator = "$ ";
        let prompt_len = prompt_indicator.len();

        // Calculate visible portion of input (handle long input)
        let max_visible_chars = (buffer_width / cell_w).saturating_sub(prompt_len + 2);
        let input_chars: Vec<char> = input.chars().collect();

        // Calculate cursor position in chars
        let cursor_char_pos = input[..cursor_pos.min(input.len())].chars().count();

        // Calculate visible window around cursor
        let (visible_start, visible_input): (usize, String) =
            if input_chars.len() <= max_visible_chars {
                (0, input.to_string())
            } else {
                // Keep cursor visible, preferably in the middle
                let half_visible = max_visible_chars / 2;
                let start = if cursor_char_pos < half_visible {
                    0
                } else if cursor_char_pos > input_chars.len().saturating_sub(half_visible) {
                    input_chars.len().saturating_sub(max_visible_chars)
                } else {
                    cursor_char_pos.saturating_sub(half_visible)
                };
                let end = (start + max_visible_chars).min(input_chars.len());
                (start, input_chars[start..end].iter().collect())
            };

        let visible_cursor_pos = cursor_char_pos.saturating_sub(visible_start);

        let text_len = prompt_len + visible_input.chars().count() + 1; // +1 for cursor
        let padding_x = cell_w / 2;
        let padding_y = cell_h / 4;

        let prompt_width = (text_len * cell_w + padding_x * 2).min(buffer_width);
        let prompt_height = cell_h + padding_y * 2;

        // Position: bottom-left, at the bottom edge
        let prompt_x = padding_x;
        let prompt_y = buffer_height.saturating_sub(prompt_height + cell_h / 2);

        // Semi-transparent dark background with slight blue tint
        let bg_color = 0x1A2030;
        let fg_color = 0xAABBCC;
        let cursor_color = 0x88AAFF;
        let prompt_color = 0x66AA66;
        let border_color = 0x334455;

        // Draw background
        for y in prompt_y..prompt_y + prompt_height {
            if y >= buffer_height {
                continue;
            }
            for x in prompt_x..prompt_x + prompt_width {
                if x >= buffer_width {
                    continue;
                }
                buffer[y * buffer_width + x] = bg_color;
            }
        }

        // Draw border
        for x in prompt_x..prompt_x + prompt_width {
            if x < buffer_width {
                if prompt_y < buffer_height {
                    buffer[prompt_y * buffer_width + x] = border_color;
                }
                if prompt_y + prompt_height - 1 < buffer_height {
                    buffer[(prompt_y + prompt_height - 1) * buffer_width + x] = border_color;
                }
            }
        }
        for y in prompt_y..prompt_y + prompt_height {
            if y < buffer_height {
                if prompt_x < buffer_width {
                    buffer[y * buffer_width + prompt_x] = border_color;
                }
                if prompt_x + prompt_width - 1 < buffer_width {
                    buffer[y * buffer_width + prompt_x + prompt_width - 1] = border_color;
                }
            }
        }

        // Draw prompt indicator
        let text_x = prompt_x + padding_x;
        let text_y = prompt_y + padding_y;

        for (i, ch) in prompt_indicator.chars().enumerate() {
            let x = text_x + i * cell_w;
            if x + cell_w > buffer_width {
                break;
            }
            let glyph = font::get_glyph(ch);
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
                prompt_color,
            );
        }

        // Draw input text
        let input_start_x = text_x + prompt_len * cell_w;
        for (i, ch) in visible_input.chars().enumerate() {
            let x = input_start_x + i * cell_w;
            if x + cell_w > buffer_width {
                break;
            }
            let glyph = font::get_glyph(ch);
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

        // Draw cursor
        let cursor_x = input_start_x + visible_cursor_pos * cell_w;
        if cursor_x + cell_w <= buffer_width {
            // Draw a block cursor
            for y in text_y..text_y + cell_h {
                if y >= buffer_height {
                    continue;
                }
                for x in cursor_x..cursor_x + cell_w {
                    if x >= buffer_width {
                        continue;
                    }
                    buffer[y * buffer_width + x] = cursor_color;
                }
            }

            // Draw the character at cursor position in contrasting color (if any)
            if visible_cursor_pos < visible_input.chars().count() {
                let cursor_char = visible_input.chars().nth(visible_cursor_pos).unwrap_or(' ');
                let glyph = font::get_glyph(cursor_char);
                self.draw_glyph(
                    buffer,
                    buffer_width,
                    buffer_height,
                    0,
                    0,
                    buffer_width,
                    buffer_height,
                    font_scale,
                    cursor_x,
                    text_y,
                    glyph,
                    bg_color,
                );
            }
        }
    }
}
