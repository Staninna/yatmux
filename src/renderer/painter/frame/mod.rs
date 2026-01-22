use std::sync::Arc;

use anyhow::Result;
use softbuffer::Surface;

use crate::config::FontConfig;
use crate::terminal::Terminal;

use super::Renderer;
use super::super::view::TerminalView;
use super::super::{RenderFrame, UiStyle};

mod borders;
mod focus;

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
                    crate::core::color::Color::Default,
                    crate::core::color::Color::Default,
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
}
