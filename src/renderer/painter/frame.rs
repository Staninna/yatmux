use std::sync::Arc;

use anyhow::Result;
use softbuffer::Surface;
use vt100::Color;

use crate::config::FontConfig;
use crate::terminal::Terminal;

use super::Renderer;
use super::primitives::contrast_color;

use super::super::color::{color_to_u32, lighten_color};
use super::super::font::{self, tab_indicator_glyph};
use super::super::view::TerminalView;
use super::super::{RenderFrame, UiStyle};

impl Renderer {
    /// Paint a terminal into a region of an existing backbuffer.
    #[allow(clippy::too_many_arguments)]
    pub fn paint_terminal_region(
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
        terminal: &Terminal,
        palette: &Arc<[u32; 256]>,
        view: &mut TerminalView,
        style: &UiStyle,
        font_config: &FontConfig,
    ) -> Result<()> {
        let cell_w = cell_w.max(1);
        let cell_h = cell_h.max(1);
        let font_scale = self.font_renderer.clamp_scale(font_scale);

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
            style,
            font_config,
        );

        Ok(())
    }

    pub fn render(
        &mut self,
        surface: &mut Surface<winit::event_loop::OwnedDisplayHandle, winit::window::Window>,
        terminal: &Terminal,
        palette: &Arc<[u32; 256]>,
        view: &mut TerminalView,
        font_scale: f32,
        style: &UiStyle,
        font_config: &FontConfig,
    ) -> Result<()> {
        let mut buffer = surface
            .buffer_mut()
            .map_err(|e| anyhow::anyhow!("softbuffer buffer_mut failed: {e:?}"))?;

        let buffer_width = buffer.width().get() as usize;
        let buffer_height = buffer.height().get() as usize;
        buffer.fill(style.base_bg);

        let font_scale = self.font_renderer.clamp_scale(font_scale);
        let (cell_w, cell_h) = self.font_renderer.cell_size(font_config);

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
            style,
            font_config,
        )?;

        buffer
            .present()
            .map_err(|e| anyhow::anyhow!("softbuffer present failed: {e:?}"))?;

        Ok(())
    }

    fn paint_frame(
        &mut self,
        buffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        origin_x: usize,
        origin_y: usize,
        region_w: usize,
        region_h: usize,
        cell_w: usize,
        cell_h: usize,
        font_scale: f32,
        frame: &RenderFrame,
        palette: &[u32; 256],
        view: &TerminalView,
        style: &UiStyle,
        font_config: &FontConfig,
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
                    style,
                    font_config,
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
                style,
                font_config,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_cell(
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

        let (fg, fill_color) = if invert {
            (fill_color, fg)
        } else {
            (fg, fill_color)
        };

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
                    backbuffer, width, height, origin_x, origin_y, region_w, region_h, font_scale,
                    x0, y0, glyph, fg,
                );
            }
        }

        if is_url_hovered {
            let underline_y = y0 + cell_h - 2;
            if underline_y < clip_bottom {
                for x in x0..(x0 + cell_w).min(clip_right) {
                    backbuffer[underline_y * width + x] = fg;
                }
            }
        }
    }
}
