//! Terminal rendering and terminal view state.
//!
//! There are two concepts here:
//! - `TerminalView`: UI state + frame building (scrolling, selection, URLs, search).
//! - `Renderer`: a pixel painter for a `RenderFrame`.
//!
//! Scrollback lives in the terminal model (`wezterm-term`). The view asks the
//! terminal for a snapshot of the *visible* rows. Only when search needs to
//! (re)index matches do we build a full snapshot of the scrollback.

mod color;
mod font;
pub mod scrollback;
mod search;
mod selection;
mod url;

pub use color::create_palette;
pub use search::{SearchMatch, SearchState};

use std::sync::Arc;

use anyhow::Result;
use softbuffer::Surface;
use vt100::Color;

use crate::constants::{
    CELL_H, CELL_W, DEFAULT_BG_COLOR, DEFAULT_FG_COLOR, FONT_SCALE, GLYPH_H, GLYPH_W,
};
use crate::terminal::Terminal;

use color::{color_to_u32, lighten_color};
use font::tab_indicator_glyph;
use scrollback::RowSnapshot;
use selection::SelectionManager;
use url::UrlManager;

use crate::core::color_codes::ColorCodeManager;

/// Search highlight colors.
const SEARCH_MATCH_BG: u32 = 0x4A4A00; // Dark yellow for regular matches
const SEARCH_CURRENT_BG: u32 = 0x806000; // Brighter yellow for current match

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

fn contrast_color(bg: u32) -> u32 {
    // Use a WCAG-ish cutoff; doesn't need to be perfect.
    if relative_luminance(bg) < 0.5 {
        0xFFFFFF
    } else {
        0x000000
    }
}

struct RenderFrame {
    cursor: (u16, u16),
    display_rows: Vec<RowSnapshot>,
    rows: usize,
    cols: usize,
    view_start: usize,
    show_cursor: bool,
}

/// Maintains terminal UI state and builds `RenderFrame`s.
///
/// This is intentionally separate from pixel painting, which is handled by `Renderer`.
pub struct TerminalView {
    selection: SelectionManager,
    urls: UrlManager,
    color_codes: ColorCodeManager,
    search: SearchState,
    view_rows: usize,
    view_cols: usize,
    /// Current scroll offset (0 = live view, >0 = scrolled back).
    scroll_offset: usize,
    /// Total number of rows (scrollback + viewport) in last frame.
    last_buffer_len: usize,
    /// Cached display rows from last frame (for copy operations).
    last_display_rows: Vec<RowSnapshot>,
    /// Cached search inputs to avoid re-indexing every frame.
    last_search_query: String,
    last_search_terminal_generation: u64,
    last_search_case_sensitive: bool,
}

impl Default for TerminalView {
    fn default() -> Self {
        TerminalView::new()
    }
}

impl TerminalView {
    pub fn new() -> Self {
        TerminalView {
            selection: SelectionManager::new(),
            urls: UrlManager::new(),
            color_codes: ColorCodeManager::new(),
            search: SearchState::new(),
            view_rows: 0,
            view_cols: 0,
            scroll_offset: 0,
            last_buffer_len: 0,
            last_display_rows: Vec::new(),
            last_search_query: String::new(),
            last_search_terminal_generation: 0,
            last_search_case_sensitive: false,
        }
    }

    fn set_dimensions(&mut self, rows: usize, cols: usize) {
        if self.view_rows != rows || self.view_cols != cols {
            self.view_rows = rows;
            self.view_cols = cols;
            self.selection.set_dimensions(rows, cols);
            self.urls.set_dimensions(rows);
            self.color_codes.set_dimensions(rows);

            // Clamp scroll offset to new viewport size.
            self.scroll_offset = self.scroll_offset.min(self.max_scroll_offset());
        }
    }

    fn build_frame(
        &mut self,
        terminal: &Terminal,
        rows: usize,
        cols: usize,
    ) -> Result<RenderFrame> {
        let buffer_len = terminal.buffer_len();
        let terminal_generation = terminal.generation();

        // Maintain scroll position when new output arrives.
        if self.scroll_offset > 0 {
            let new_lines = buffer_len.saturating_sub(self.last_buffer_len);
            if new_lines > 0 {
                self.scroll_offset = (self.scroll_offset + new_lines).min(self.max_scroll_offset());
            }
        }
        self.last_buffer_len = buffer_len;

        // Clamp scroll offset to valid range.
        self.scroll_offset = self
            .scroll_offset
            .min(self.max_scroll_offset_with_len(buffer_len));

        // Visible window start in absolute coordinates.
        let window_start = buffer_len.saturating_sub(rows + self.scroll_offset);

        let mut display_rows = terminal.rows_in_range(window_start, rows, cols);
        if display_rows.len() < rows {
            let mut padded = Vec::with_capacity(rows);
            for _ in 0..(rows - display_rows.len()) {
                padded.push(RowSnapshot::blank(cols));
            }
            padded.extend(display_rows);
            display_rows = padded;
        }

        self.selection
            .set_scroll_state(self.scroll_offset, buffer_len);

        // Cache display rows for copy operations.
        self.last_display_rows = display_rows.clone();

        // Update search matches when search is active and inputs changed.
        if self.search.is_active() {
            let query = self.search.query().to_string();
            let case_sensitive = self.search.is_case_sensitive();

            if query != self.last_search_query
                || terminal_generation != self.last_search_terminal_generation
                || case_sensitive != self.last_search_case_sensitive
            {
                let all_rows = terminal.rows_in_range(0, buffer_len, cols);
                self.search.update_matches(&all_rows);
                self.last_search_query = query;
                self.last_search_terminal_generation = terminal_generation;
                self.last_search_case_sensitive = case_sensitive;
            }
        }

        // Detect URLs and hex color codes in each visible row.
        for (row_idx, row_data) in display_rows.iter().enumerate() {
            let text: String = row_data.cells.iter().map(|(ch, _, _)| ch).collect();
            self.urls.update_row(row_idx, &text);
            self.color_codes.update_row(row_idx, &text);
        }

        let (cursor, cursor_visible) = terminal.cursor();

        Ok(RenderFrame {
            cursor,
            display_rows,
            rows,
            cols,
            view_start: window_start,
            show_cursor: self.scroll_offset == 0 && cursor_visible,
        })
    }

    fn max_scroll_offset(&self) -> usize {
        self.max_scroll_offset_with_len(self.last_buffer_len)
    }

    fn max_scroll_offset_with_len(&self, buffer_len: usize) -> usize {
        buffer_len.saturating_sub(self.view_rows)
    }

    fn scroll_to_row(&mut self, row: usize) {
        let buffer_len = self.last_buffer_len;
        if self.view_rows == 0 {
            self.scroll_offset = 0;
            return;
        }

        let window_start = buffer_len.saturating_sub(self.view_rows + self.scroll_offset);
        let window_end = window_start + self.view_rows;

        if row < window_start {
            let desired_start = row;
            self.scroll_offset = buffer_len.saturating_sub(self.view_rows + desired_start);
        } else if row >= window_end {
            let desired_start = row + 1 - self.view_rows;
            self.scroll_offset = buffer_len.saturating_sub(self.view_rows + desired_start);
        }

        self.scroll_offset = self.scroll_offset.min(self.max_scroll_offset());
    }

    pub fn scrollback_scroll_by(&mut self, delta_lines: isize) {
        let max_offset = self.max_scroll_offset();
        let new_offset = (self.scroll_offset as isize + delta_lines).clamp(0, max_offset as isize);
        self.scroll_offset = new_offset as usize;
    }

    pub fn scrollback_snap_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn is_scrolled_up(&self) -> bool {
        self.scroll_offset > 0
    }

    pub fn start_selection(&mut self, row: usize, col: usize) {
        self.selection.start(row, col);
    }

    pub fn update_selection(&mut self, row: usize, col: usize) {
        self.selection.update(row, col);
    }

    pub fn window_to_cell(&self, x: f64, y: f64) -> Option<(usize, usize)> {
        if self.view_rows == 0 || self.view_cols == 0 {
            return None;
        }

        let col = (x as usize) / CELL_W;
        let row = (y as usize) / CELL_H;

        if row >= self.view_rows || col >= self.view_cols {
            return None;
        }

        Some((row, col))
    }

    pub fn get_selection_bounds(&self) -> Option<((usize, usize), (usize, usize))> {
        self.selection.bounds()
    }

    pub fn get_selected_text(&self) -> Option<String> {
        let ((start_row, start_col), (end_row, end_col)) = self.selection.visible_bounds()?;

        if self.last_display_rows.is_empty() {
            return None;
        }

        let mut text = String::new();

        for row in start_row..=end_row {
            if row >= self.last_display_rows.len() {
                break;
            }

            let row_data = &self.last_display_rows[row];
            let row_start = if row == start_row { start_col } else { 0 };
            let row_end = if row == end_row {
                (end_col + 1).min(row_data.cells.len())
            } else {
                row_data.cells.len()
            };

            for col in row_start..row_end {
                if let Some((ch, _, _)) = row_data.cells.get(col) {
                    text.push(*ch);
                }
            }

            if row != end_row {
                text.push('\n');
            }
        }

        let trimmed: String = text
            .lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n");

        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }

    pub fn display_rows_len(&self) -> usize {
        self.last_display_rows.len()
    }

    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }

    pub fn clear_scrollback(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn update_url_hover(&mut self, row: usize, col: usize) -> bool {
        let was_hovered = self.urls.hovered_url().is_some();
        self.urls.update_hover(row, col);
        let is_hovered = self.urls.hovered_url().is_some();
        was_hovered != is_hovered || is_hovered
    }

    pub fn clear_url_hover(&mut self) {
        self.urls.clear_hover();
    }

    pub fn url_at(&self, row: usize, col: usize) -> Option<String> {
        self.urls.url_at(row, col).map(|span| span.full_url())
    }

    pub fn has_hovered_url(&self) -> bool {
        self.urls.hovered_url().is_some()
    }

    // =========================================================================
    // Search Methods
    // =========================================================================

    pub fn is_search_active(&self) -> bool {
        self.search.is_active()
    }

    pub fn activate_search(&mut self) {
        self.search.activate();
    }

    pub fn deactivate_search(&mut self) {
        self.search.deactivate();
    }

    pub fn search_query(&self) -> &str {
        self.search.query()
    }

    pub fn search_match_count(&self) -> usize {
        self.search.match_count()
    }

    pub fn search_current_index(&self) -> usize {
        self.search.current_match_index()
    }

    pub fn search_push_char(&mut self, ch: char) {
        self.search.push_char(ch);
    }

    pub fn search_pop_char(&mut self) {
        self.search.pop_char();
    }

    pub fn search_next(&mut self) {
        self.search.next_match();
        self.scroll_to_current_match();
    }

    pub fn search_prev(&mut self) {
        self.search.prev_match();
        self.scroll_to_current_match();
    }

    fn scroll_to_current_match(&mut self) {
        if let Some(match_row) = self.search.current_match_row() {
            self.scroll_to_row(match_row);
        }
    }

    pub fn search_toggle_case(&mut self) {
        self.search.toggle_case_sensitive();
    }

    pub fn is_search_case_sensitive(&self) -> bool {
        self.search.is_case_sensitive()
    }
}

/// A categorized list of key bindings for the help overlay.
#[derive(Clone, Debug)]
pub struct HelpSection {
    pub title: String,
    pub bindings: Vec<(String, String)>,
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
    pub fn paint_terminal_region(
        &self,
        backbuffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        origin_x: usize,
        origin_y: usize,
        region_w: usize,
        region_h: usize,
        terminal: &Terminal,
        palette: &Arc<[u32; 256]>,
        view: &mut TerminalView,
    ) -> Result<()> {
        if region_w < CELL_W || region_h < CELL_H {
            return Ok(());
        }

        let rows = region_h / CELL_H;
        let cols = region_w / CELL_W;
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
    ) -> Result<()> {
        let mut buffer = surface
            .buffer_mut()
            .map_err(|e| anyhow::anyhow!("softbuffer buffer_mut failed: {e:?}"))?;

        let buffer_width = buffer.width().get() as usize;
        let buffer_height = buffer.height().get() as usize;
        buffer.fill(DEFAULT_BG_COLOR);

        self.paint_terminal_region(
            &mut buffer,
            buffer_width,
            buffer_height,
            0,
            0,
            buffer_width,
            buffer_height,
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
    ) -> (usize, usize) {
        if buffer_width < CELL_W * 10 || buffer_height < CELL_H * 5 {
            return (0, 0);
        }

        let padding_cells_x = 2usize;
        let padding_cells_y = 1usize;

        let fixed_lines = vec![title.to_string(), String::new()];
        let mut content_lines: Vec<String> = Vec::new();

        let mut max_line_len = title.len();
        for (section_idx, section) in sections.iter().enumerate() {
            if section_idx > 0 {
                content_lines.push(String::new());
            }

            max_line_len = max_line_len.max(section.title.len());
            content_lines.push(section.title.clone());

            // Render like: "  ctrl+shift+/  Toggle help"
            for (key, action) in &section.bindings {
                let line = format!("  {:<16} {}", key, action);
                max_line_len = max_line_len.max(line.len());
                content_lines.push(line);
            }
        }

        let box_cols = (max_line_len + padding_cells_x * 2).min(buffer_width / CELL_W);
        let total_rows = fixed_lines.len() + content_lines.len() + padding_cells_y * 2;
        let box_rows = total_rows.min(buffer_height / CELL_H);

        let box_w = box_cols * CELL_W;
        let box_h = box_rows * CELL_H;
        let origin_x = buffer_width.saturating_sub(box_w) / 2;
        let origin_y = buffer_height.saturating_sub(box_h) / 2;

        // Background
        let bg = 0x1A1A1A;
        let border = accent_color;

        for y in origin_y..(origin_y + box_h).min(buffer_height) {
            let row = y * buffer_width;
            for x in origin_x..(origin_x + box_w).min(buffer_width) {
                backbuffer[row + x] = bg;
            }
        }

        // Border
        for x in origin_x..(origin_x + box_w).min(buffer_width) {
            backbuffer[origin_y * buffer_width + x] = border;
            let yb = (origin_y + box_h - 1).min(buffer_height - 1);
            backbuffer[yb * buffer_width + x] = border;
        }
        for y in origin_y..(origin_y + box_h).min(buffer_height) {
            backbuffer[y * buffer_width + origin_x] = border;
            let xb = (origin_x + box_w - 1).min(buffer_width - 1);
            backbuffer[y * buffer_width + xb] = border;
        }

        let content_rows = box_rows
            .saturating_sub(padding_cells_y * 2)
            .saturating_sub(fixed_lines.len());
        let max_scroll = content_lines.len().saturating_sub(content_rows);
        let scroll = scroll_offset.min(max_scroll);

        // Scrollbar (only when there is overflow)
        if max_scroll > 0 && content_rows > 0 {
            let track_y0 = origin_y + padding_cells_y * CELL_H + fixed_lines.len() * CELL_H;
            let track_y1 = origin_y + box_h - padding_cells_y * CELL_H;
            let track_y0 = track_y0.min(buffer_height.saturating_sub(1));
            let track_y1 = track_y1.min(buffer_height);

            if track_y1 > track_y0 {
                let track_h = track_y1 - track_y0;
                let total = content_lines.len().max(1);
                let visible = content_rows.min(total);

                let mut thumb_h = (track_h * visible) / total;
                thumb_h = thumb_h.max(CELL_H).min(track_h);

                let travel = track_h.saturating_sub(thumb_h);
                let thumb_y0 = if max_scroll == 0 {
                    track_y0
                } else {
                    track_y0 + (travel * scroll) / max_scroll
                };
                let thumb_y1 = (thumb_y0 + thumb_h).min(track_y1);

                // Draw inside the right padding area.
                let bar_w = 2usize;
                let bar_x1 = (origin_x + box_w).min(buffer_width).saturating_sub(2);
                let bar_x0 = bar_x1.saturating_sub(bar_w);

                let track_color = 0x333333;
                let thumb_color = accent_color;

                for y in track_y0..track_y1 {
                    let row = y * buffer_width;
                    for x in bar_x0..bar_x1 {
                        backbuffer[row + x] = track_color;
                    }
                }

                for y in thumb_y0..thumb_y1 {
                    let row = y * buffer_width;
                    for x in bar_x0..bar_x1 {
                        backbuffer[row + x] = thumb_color;
                    }
                }
            }
        }

        // Text
        let text_color = 0xFFFFFF;
        let mut y = origin_y + padding_cells_y * CELL_H;

        for (idx, line) in fixed_lines.iter().enumerate() {
            if idx + padding_cells_y >= box_rows {
                break;
            }
            let mut x = origin_x + padding_cells_x * CELL_W;
            for ch in line.chars() {
                if x + CELL_W > origin_x + box_w - padding_cells_x * CELL_W {
                    break;
                }
                let glyph = font::get_glyph(ch);
                self.draw_glyph(
                    backbuffer,
                    buffer_width,
                    buffer_height,
                    origin_x,
                    origin_y,
                    box_w,
                    box_h,
                    x,
                    y,
                    glyph,
                    text_color,
                );
                x += CELL_W;
            }
            y += CELL_H;
        }

        for (idx, line) in content_lines
            .iter()
            .skip(scroll)
            .take(content_rows)
            .enumerate()
        {
            let overall_idx = idx + fixed_lines.len();
            if overall_idx + padding_cells_y >= box_rows {
                break;
            }
            let mut x = origin_x + padding_cells_x * CELL_W;
            for ch in line.chars() {
                if x + CELL_W > origin_x + box_w - padding_cells_x * CELL_W {
                    break;
                }
                let glyph = font::get_glyph(ch);
                self.draw_glyph(
                    backbuffer,
                    buffer_width,
                    buffer_height,
                    origin_x,
                    origin_y,
                    box_w,
                    box_h,
                    x,
                    y,
                    glyph,
                    text_color,
                );
                x += CELL_W;
            }
            y += CELL_H;
        }

        (scroll, max_scroll)
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
                view,
            );
        }
    }

    fn draw_cell(
        &self,
        backbuffer: &mut [u32],
        width: usize,
        height: usize,
        origin_x: usize,
        origin_y: usize,
        region_w: usize,
        region_h: usize,
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

        let x0 = origin_x + col * CELL_W;
        let y0 = origin_y + row * CELL_H;

        let clip_right = (origin_x + region_w).min(width);
        let clip_bottom = (origin_y + region_h).min(height);

        if x0 >= clip_right || y0 >= clip_bottom {
            return;
        }

        for y in y0..(y0 + CELL_H).min(clip_bottom) {
            for x in x0..(x0 + CELL_W).min(clip_right) {
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
                    tab_indicator_glyph(),
                    tab_fg,
                );
            }
            return;
        }

        let glyph = font::get_glyph(ch);
        self.draw_glyph(
            backbuffer, width, height, origin_x, origin_y, region_w, region_h, x0, y0, glyph, fg,
        );

        if is_url_hovered {
            let underline_y = y0 + CELL_H - 2;
            if underline_y < clip_bottom {
                for x in x0..(x0 + CELL_W).min(clip_right) {
                    backbuffer[underline_y * width + x] = fg;
                }
            }
        }
    }

    fn draw_glyph(
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
        glyph: [u8; 8],
        color: u32,
    ) {
        let clip_right = (origin_x + region_w).min(width);
        let clip_bottom = (origin_y + region_h).min(height);

        for gy in 0..GLYPH_H {
            let bits = glyph[gy];
            for gx in 0..GLYPH_W {
                let on = (bits >> gx) & 1 == 1;
                if !on {
                    continue;
                }

                for sy in 0..FONT_SCALE {
                    for sx in 0..FONT_SCALE {
                        let x = x0 + gx * FONT_SCALE + sx;
                        let y = y0 + gy * FONT_SCALE + sy;
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
        view: &TerminalView,
    ) {
        let bar_height = CELL_H;
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
        let mut x_pos = origin_x + CELL_W / 2;
        for ch in prefix.chars() {
            if x_pos + CELL_W > clip_right {
                break;
            }
            let glyph = font::get_glyph(ch);
            self.draw_glyph(
                buffer, width, height, origin_x, origin_y, region_w, region_h, x_pos, bar_y, glyph,
                text_color,
            );
            x_pos += CELL_W;
        }

        for ch in view.search.query().chars() {
            if x_pos + CELL_W > clip_right.saturating_sub(100) {
                break;
            }
            let glyph = font::get_glyph(ch);
            self.draw_glyph(
                buffer, width, height, origin_x, origin_y, region_w, region_h, x_pos, bar_y, glyph,
                text_color,
            );
            x_pos += CELL_W;
        }

        let cursor_x = x_pos;
        for y in bar_y.min(clip_bottom)..(bar_y + CELL_H).min(clip_bottom) {
            if cursor_x < clip_right {
                buffer[y * width + cursor_x] = text_color;
            }
        }

        let match_count = view.search.match_count();
        let current_idx = view.search.current_match_index();
        let case_indicator = if view.search.is_case_sensitive() {
            "[Aa]"
        } else {
            "[aa]"
        };

        let match_info = if match_count > 0 {
            format!("{}/{} {}", current_idx + 1, match_count, case_indicator)
        } else if !view.search.query().is_empty() {
            format!("0/0 {}", case_indicator)
        } else {
            case_indicator.to_string()
        };

        let info_width = match_info.len() * CELL_W;
        let info_x = clip_right.saturating_sub(info_width + CELL_W);

        let mut x_pos = info_x;
        for ch in match_info.chars() {
            if x_pos + CELL_W > clip_right {
                break;
            }
            let glyph = font::get_glyph(ch);
            self.draw_glyph(
                buffer,
                width,
                height,
                origin_x,
                origin_y,
                region_w,
                region_h,
                x_pos,
                bar_y,
                glyph,
                match_info_color,
            );
            x_pos += CELL_W;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_default() {
        let _view = TerminalView::new();
    }

    #[test]
    fn test_window_to_cell() {
        let mut view = TerminalView::new();
        view.set_dimensions(24, 80);

        assert_eq!(view.window_to_cell(0.0, 0.0), Some((0, 0)));
        assert_eq!(view.window_to_cell(8.0, 8.0), Some((0, 0)));
        assert_eq!(view.window_to_cell(17.0, 17.0), Some((1, 1)));
    }

    #[test]
    fn test_window_to_cell_out_of_bounds() {
        let mut view = TerminalView::new();
        view.set_dimensions(24, 80);
        assert_eq!(view.window_to_cell(10000.0, 10000.0), None);
    }

    #[test]
    fn test_search_scroll_to_match() {
        use crate::renderer::scrollback::RowSnapshot;
        use vt100::Color;

        fn make_row(text: &str, cols: usize) -> RowSnapshot {
            let mut cells: Vec<_> = text
                .chars()
                .map(|ch| (ch, Color::Default, Color::Default))
                .collect();
            while cells.len() < cols {
                cells.push((' ', Color::Default, Color::Default));
            }
            let tabs = vec![None; cols];
            RowSnapshot::new(cells, tabs)
        }

        let mut view = TerminalView::new();
        view.set_dimensions(24, 80);

        let mut all_rows: Vec<_> = (0..100)
            .map(|i| {
                if i == 10 || i == 50 || i == 90 {
                    make_row(&format!("line {} test", i), 80)
                } else {
                    make_row(&format!("line {}", i), 80)
                }
            })
            .collect();
        all_rows.extend((0..24).map(|i| make_row(&format!("live {}", i), 80)));

        view.last_buffer_len = all_rows.len();

        view.activate_search();
        for ch in "test".chars() {
            view.search_push_char(ch);
        }

        view.search.update_matches(&all_rows);

        assert_eq!(view.search_match_count(), 3);
        assert_eq!(view.search.current_match_row(), Some(10));

        view.search_next();
        assert_eq!(view.search.current_match_row(), Some(50));

        let view_start = view
            .last_buffer_len
            .saturating_sub(view.view_rows + view.scroll_offset);
        let view_end = view_start + view.view_rows;
        assert!(view_start <= 50 && 50 < view_end);

        view.search_next();
        assert_eq!(view.search.current_match_row(), Some(90));

        let view_start = view
            .last_buffer_len
            .saturating_sub(view.view_rows + view.scroll_offset);
        let view_end = view_start + view.view_rows;
        assert!(view_start <= 90 && 90 < view_end);
    }
}
