//! Terminal renderer module.
//!
//! This module provides the main rendering functionality for the terminal,
//! composing together font, color, selection, and scrollback handling.
//!
//! Scrollback uses a hybrid approach:
//! - vt100 tracks how many lines scrolled off (reliably, even with rapid output)
//! - We capture newly scrolled-off lines to our own history buffer
//! - Our history buffer provides full history for search and arbitrary scrollback
//! - Display blends our history with vt100's current live view

mod color;
mod font;
pub mod scrollback;
mod search;
mod selection;
mod url;

pub use color::create_palette;
pub use search::{SearchMatch, SearchState};

use std::sync::{Arc, Mutex};

use anyhow::Result;
use softbuffer::Surface;
use vt100::Color;

use crate::constants::{
    CELL_H, CELL_W, DEFAULT_BG_COLOR, DEFAULT_FG_COLOR, FONT_SCALE, GLYPH_H, GLYPH_W,
    SCROLLBACK_CAPACITY, TAB_STOP_WIDTH,
};

use color::{color_to_u32, lighten_color};
use font::tab_indicator_glyph;
use scrollback::{CellData, RowSnapshot, ScrollbackBuffer};
use selection::SelectionManager;
use url::UrlManager;

/// Search highlight colors.
const SEARCH_MATCH_BG: u32 = 0x4A4A00; // Dark yellow for regular matches
const SEARCH_CURRENT_BG: u32 = 0x806000; // Brighter yellow for current match

/// The main terminal renderer.
pub struct Renderer {
    selection: SelectionManager,
    scrollback: ScrollbackBuffer,
    urls: UrlManager,
    search: SearchState,
    view_rows: usize,
    view_cols: usize,
    /// Cached display rows from last render (for copy operations).
    last_display_rows: Vec<RowSnapshot>,
    /// Shadow copy of live rows at their maximum width.
    /// This preserves content when terminal is resized smaller.
    preserved_live_rows: Vec<RowSnapshot>,
    /// The width at which preserved_live_rows was last updated.
    preserved_width: usize,
}

impl Default for Renderer {
    fn default() -> Self {
        Renderer::new()
    }
}

impl Renderer {
    /// Creates a new renderer with default settings.
    pub fn new() -> Self {
        Renderer {
            selection: SelectionManager::new(),
            scrollback: ScrollbackBuffer::new(),
            urls: UrlManager::new(),
            search: SearchState::new(),
            view_rows: 0,
            view_cols: 0,
            last_display_rows: Vec::new(),
            preserved_live_rows: Vec::new(),
            preserved_width: 0,
        }
    }

    /// Updates the view dimensions.
    fn set_dimensions(&mut self, rows: usize, cols: usize) {
        if self.view_rows != rows || self.view_cols != cols {
            self.view_rows = rows;
            self.view_cols = cols;
            self.scrollback.set_dimensions(rows, cols);
            self.selection.set_dimensions(rows, cols);
            self.urls.set_dimensions(rows);
        }
    }

    /// Draws a single cell to the backbuffer.
    fn draw_cell(
        &self,
        backbuffer: &mut [u32],
        width: usize,
        height: usize,
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
        search_match: Option<bool>, // Some(true) = current match, Some(false) = other match
    ) {
        let fg = color_to_u32(fg_color, DEFAULT_FG_COLOR, palette);
        let bg = color_to_u32(bg_color, DEFAULT_BG_COLOR, palette);

        // Handle cursor inversion
        let (fg, bg) = if invert { (bg, fg) } else { (fg, bg) };

        // URL color: use a blue tint for URLs
        let fg = if is_url { 0x6699FF } else { fg };

        // Determine fill color based on state priority:
        // 1. Search current match (highest)
        // 2. Search other matches
        // 3. Selection
        // 4. Tab indicator
        // 5. Normal background (lowest)
        let fill_color = match search_match {
            Some(true) => SEARCH_CURRENT_BG, // Current search match
            Some(false) => SEARCH_MATCH_BG,  // Other search matches
            None if selected || tab_info.is_some() => lighten_color(bg),
            None => bg,
        };

        let x0 = col * CELL_W;
        let y0 = row * CELL_H;

        // Fill background
        for y in y0..(y0 + CELL_H).min(height) {
            for x in x0..(x0 + CELL_W).min(width) {
                backbuffer[y * width + x] = fill_color;
            }
        }

        // Draw tab indicator if this is the start of a tab
        if let Some((start_col, _)) = tab_info {
            if start_col == col {
                let tab_fg = lighten_color(fg);
                self.draw_glyph(
                    backbuffer,
                    width,
                    height,
                    x0,
                    y0,
                    tab_indicator_glyph(),
                    tab_fg,
                );
            }
            return;
        }

        // Draw character glyph
        let glyph = font::get_glyph(ch);
        self.draw_glyph(backbuffer, width, height, x0, y0, glyph, fg);

        // Draw underline for hovered URLs
        if is_url_hovered {
            let underline_y = y0 + CELL_H - 2;
            if underline_y < height {
                for x in x0..(x0 + CELL_W).min(width) {
                    backbuffer[underline_y * width + x] = fg;
                }
            }
        }
    }

    /// Draws a glyph bitmap at the specified position.
    fn draw_glyph(
        &self,
        backbuffer: &mut [u32],
        width: usize,
        height: usize,
        x0: usize,
        y0: usize,
        glyph: [u8; 8],
        color: u32,
    ) {
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
                        if x < width && y < height {
                            backbuffer[y * width + x] = color;
                        }
                    }
                }
            }
        }
    }

    /// Renders the terminal to the surface.
    pub fn render(
        &mut self,
        surface: &mut Surface<winit::event_loop::OwnedDisplayHandle, winit::window::Window>,
        parser: &Arc<Mutex<vt100::Parser>>,
        palette: &Arc<[u32; 256]>,
    ) -> Result<()> {
        let mut buffer = surface
            .buffer_mut()
            .map_err(|e| anyhow::anyhow!("softbuffer buffer_mut failed: {e:?}"))?;

        let buffer_width = buffer.width().get() as usize;
        let buffer_height = buffer.height().get() as usize;
        buffer.fill(DEFAULT_BG_COLOR);

        let rows = buffer_height / CELL_H;
        let cols = buffer_width / CELL_W;
        self.set_dimensions(rows, cols);

        // Capture live screen data and update our scrollback history
        let (cursor, live_rows, _vt100_scrollback_len) =
            self.capture_live_and_update_history(parser, rows, cols)?;

        // Get display rows based on current scroll offset
        let display_rows = self.scrollback.get_display_rows(&live_rows, cols);

        // Update selection with current scroll state
        let total_rows = self.scrollback.len() + rows;
        self.selection
            .set_scroll_state(self.scrollback.offset(), total_rows);

        // Cache display rows for copy operations
        self.last_display_rows = display_rows.clone();

        // Update search matches for ALL rows (history + live) when search is active
        if self.search.is_active() {
            let all_rows = self.scrollback.get_all_rows(&live_rows);
            self.search.update_matches(&all_rows);
        }

        // Calculate view_start for search highlighting
        let view_start = self.scrollback.view_start();

        // Detect URLs in each row
        for (row_idx, row_data) in display_rows.iter().enumerate().take(rows) {
            let text: String = row_data.cells.iter().map(|(ch, _, _)| ch).collect();
            self.urls.update_row(row_idx, &text);
        }

        // Render each cell
        let show_cursor = self.scrollback.offset() == 0;
        for (row_idx, row_data) in display_rows.iter().enumerate().take(rows) {
            for col in 0..cols {
                let (ch, fg, bg) = row_data.cells.get(col).copied().unwrap_or((
                    ' ',
                    Color::Default,
                    Color::Default,
                ));
                let invert = show_cursor && (row_idx as u16, col as u16) == cursor;
                let tab_info = row_data.tabs.get(col).copied().flatten();
                let selected = self.selection.is_selected(row_idx, col);
                let is_url = self.urls.is_url(row_idx, col);
                let is_url_hovered = self.urls.is_hovered(row_idx, col);

                // Search match uses absolute row index
                let search_match = self.search.is_match(row_idx, col, view_start);

                self.draw_cell(
                    &mut buffer,
                    buffer_width,
                    buffer_height,
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
                    search_match,
                );
            }
        }

        // Draw search bar if search is active
        if self.search.is_active() {
            self.draw_search_bar(&mut buffer, buffer_width, buffer_height);
        }

        buffer
            .present()
            .map_err(|e| anyhow::anyhow!("softbuffer present failed: {e:?}"))?;

        Ok(())
    }

    /// Draws the search bar at the bottom of the screen.
    fn draw_search_bar(&self, buffer: &mut [u32], width: usize, height: usize) {
        let bar_height = CELL_H;
        let bar_y = height.saturating_sub(bar_height);

        // Background color for search bar
        let bar_bg = 0x333333;
        let text_color = 0xFFFFFF;
        let match_info_color = 0xAAAAAA;

        // Fill background
        for y in bar_y..height {
            for x in 0..width {
                buffer[y * width + x] = bar_bg;
            }
        }

        // Draw "Find: " prefix
        let prefix = "Find: ";
        let mut x_pos = CELL_W / 2; // Small left padding
        for ch in prefix.chars() {
            let glyph = font::get_glyph(ch);
            self.draw_glyph(buffer, width, height, x_pos, bar_y, glyph, text_color);
            x_pos += CELL_W;
        }

        // Draw query
        for ch in self.search.query().chars() {
            if x_pos + CELL_W > width - 100 {
                break; // Don't overflow
            }
            let glyph = font::get_glyph(ch);
            self.draw_glyph(buffer, width, height, x_pos, bar_y, glyph, text_color);
            x_pos += CELL_W;
        }

        // Draw cursor
        let cursor_x = x_pos;
        for y in bar_y..(bar_y + CELL_H).min(height) {
            if cursor_x < width {
                buffer[y * width + cursor_x] = text_color;
            }
        }

        // Draw match count on the right side
        let match_count = self.search.match_count();
        let current_idx = self.search.current_match_index();
        let case_indicator = if self.search.is_case_sensitive() {
            "[Aa]"
        } else {
            "[aa]"
        };

        let match_info = if match_count > 0 {
            format!("{}/{} {}", current_idx + 1, match_count, case_indicator)
        } else if !self.search.query().is_empty() {
            format!("0/0 {}", case_indicator)
        } else {
            case_indicator.to_string()
        };

        // Calculate right-aligned position
        let info_width = match_info.len() * CELL_W;
        let info_x = width.saturating_sub(info_width + CELL_W);

        let mut x_pos = info_x;
        for ch in match_info.chars() {
            let glyph = font::get_glyph(ch);
            self.draw_glyph(buffer, width, height, x_pos, bar_y, glyph, match_info_color);
            x_pos += CELL_W;
        }
    }

    /// Captures live screen data and updates our scrollback history.
    ///
    /// This uses vt100's scrollback length as the reliable signal for when lines
    /// scroll off, then captures those lines to our own history buffer.
    fn capture_live_and_update_history(
        &mut self,
        parser: &Arc<Mutex<vt100::Parser>>,
        rows: usize,
        cols: usize,
    ) -> Result<((u16, u16), Vec<RowSnapshot>, usize)> {
        let mut parser = parser
            .lock()
            .map_err(|_| anyhow::anyhow!("parser mutex poisoned"))?;

        // Probe for actual scrollback length by setting a very large offset
        // vt100 will clamp it to the actual scrollback size
        parser.set_scrollback(SCROLLBACK_CAPACITY);
        let vt100_scrollback_len = parser.screen().scrollback();

        // Capture any newly scrolled-off rows before updating history
        let view_rows = rows;
        let new_history_rows = {
            let new_lines =
                vt100_scrollback_len.saturating_sub(self.scrollback.last_vt100_scrollback_len());

            if new_lines > 0 {
                // Capture the newly scrolled-off rows from vt100
                // The safe offset is min(new_lines, view_rows) to avoid panic
                let safe_offset = new_lines.min(view_rows);

                parser.set_scrollback(safe_offset);
                let screen = parser.screen();

                // At offset N, rows 0 through N-1 are historical rows
                // We capture them oldest to newest
                let mut captured = Vec::with_capacity(safe_offset);
                for row in 0..safe_offset {
                    let (row_cells, row_tabs) = Self::capture_row_data_static(&screen, row, cols);
                    captured.push(RowSnapshot::new(row_cells, row_tabs));
                }
                captured
            } else {
                Vec::new()
            }
        };

        // Update our scrollback history with the newly captured rows
        self.scrollback
            .add_history_rows(new_history_rows, vt100_scrollback_len);

        // Now capture the live view
        parser.set_scrollback(0);
        let screen = parser.screen();
        let cursor = screen.cursor_position();

        let mut live_rows = Vec::with_capacity(rows);
        for row in 0..rows {
            let (row_cells, row_tabs) = Self::capture_row_data_static(&screen, row, cols);
            live_rows.push(RowSnapshot::new(row_cells, row_tabs));
        }

        Ok((cursor, live_rows, vt100_scrollback_len))
    }

    /// Captures data for a single row from a screen reference (static version).
    fn capture_row_data_static(
        screen: &vt100::Screen,
        row: usize,
        cols: usize,
    ) -> (Vec<CellData>, Vec<Option<(usize, usize)>>) {
        let mut row_cells = Vec::with_capacity(cols);
        let mut row_tabs = vec![None; cols];

        for col in 0..cols {
            let cell = screen.cell(row as u16, col as u16);
            let contents = cell.map(|c| c.contents()).unwrap_or_default();
            let ch = contents.chars().next().unwrap_or(' ');
            let fg = cell.map(|c| c.fgcolor()).unwrap_or(Color::Default);
            let bg = cell.map(|c| c.bgcolor()).unwrap_or(Color::Default);

            if ch == '\t' {
                let end_col = ((col / TAB_STOP_WIDTH) + 1) * TAB_STOP_WIDTH;
                let end_col = end_col.min(cols);
                for c in col..end_col {
                    row_tabs[c] = Some((col, end_col));
                }
            }

            row_cells.push((ch, fg, bg));
        }

        (row_cells, row_tabs)
    }

    /// Scrolls the scrollback buffer by the given number of lines.
    pub fn scrollback_scroll_by(&mut self, delta_lines: isize) {
        self.scrollback.scroll_by(delta_lines);
    }

    /// Snaps scrollback to the bottom (live view).
    pub fn scrollback_snap_to_bottom(&mut self) {
        self.scrollback.snap_to_bottom();
    }

    /// Returns true if scrolled up in history.
    pub fn is_scrolled_up(&self) -> bool {
        self.scrollback.is_scrolled_up()
    }

    /// Preserves the current live screen content before a resize.
    /// Call this BEFORE resizing the vt100 parser to prevent data loss.
    pub fn preserve_content_before_resize(&mut self, parser: &Arc<Mutex<vt100::Parser>>) {
        let Ok(parser) = parser.lock() else {
            return;
        };

        let screen = parser.screen();
        let (rows, cols) = screen.size();
        let rows = rows as usize;
        let cols = cols as usize;

        // Capture current screen content
        let mut current_rows = Vec::with_capacity(rows);
        let mut current_content_chars = 0usize;
        for row in 0..rows {
            let (row_cells, row_tabs) = Self::capture_row_data_static(&screen, row, cols);
            current_content_chars += row_cells
                .iter()
                .filter(|(ch, _, _)| !ch.is_whitespace())
                .count();
            current_rows.push(RowSnapshot::new(row_cells, row_tabs));
        }

        // Count content in preserved rows
        let preserved_content_chars: usize = self
            .preserved_live_rows
            .iter()
            .map(|row| {
                row.cells
                    .iter()
                    .take(cols)
                    .filter(|(ch, _, _)| !ch.is_whitespace())
                    .count()
            })
            .sum();

        // Update preserved data if we have content and either:
        // 1. Current width >= preserved width, OR
        // 2. Current has more content than preserved
        let should_update = current_content_chars > 0
            && (cols >= self.preserved_width || current_content_chars > preserved_content_chars);

        if should_update {
            // If narrower but more content, merge with old preserved tails
            if cols < self.preserved_width && !self.preserved_live_rows.is_empty() {
                for (i, current_row) in current_rows.iter_mut().enumerate() {
                    if let Some(preserved) = self.preserved_live_rows.get(i) {
                        if preserved.cells.len() > current_row.cells.len() {
                            let original_len = current_row.cells.len();
                            current_row
                                .cells
                                .extend_from_slice(&preserved.cells[original_len..]);
                            if original_len < preserved.tabs.len() {
                                current_row
                                    .tabs
                                    .extend_from_slice(&preserved.tabs[original_len..]);
                            }
                            while current_row.tabs.len() < current_row.cells.len() {
                                current_row.tabs.push(None);
                            }
                        }
                    }
                }
            } else {
                self.preserved_width = cols;
            }

            self.preserved_live_rows = current_rows;
        }
    }

    /// Reflows rows to a new width by wrapping long lines.
    fn reflow_rows(&self, rows: &[RowSnapshot], target_width: usize) -> Vec<RowSnapshot> {
        let mut result = Vec::new();

        for row in rows {
            let content_end = row
                .cells
                .iter()
                .rposition(|(ch, _, _)| !ch.is_whitespace())
                .map(|p| p + 1)
                .unwrap_or(0);

            if content_end == 0 {
                result.push(RowSnapshot::blank(target_width));
                continue;
            }

            // Wrap content to target_width
            let mut pos = 0;
            while pos < content_end {
                let end = (pos + target_width).min(content_end);
                let mut cells = Vec::with_capacity(target_width);
                cells.extend_from_slice(&row.cells[pos..end]);
                while cells.len() < target_width {
                    cells.push((' ', Color::Default, Color::Default));
                }

                let mut tabs = Vec::with_capacity(target_width);
                if pos < row.tabs.len() {
                    let tab_end = end.min(row.tabs.len());
                    tabs.extend_from_slice(&row.tabs[pos..tab_end]);
                }
                while tabs.len() < target_width {
                    tabs.push(None);
                }

                result.push(RowSnapshot::new(cells, tabs));
                pos = end;
            }
        }

        result
    }

    /// Starts a text selection at the given cell position.
    pub fn start_selection(&mut self, row: usize, col: usize) {
        self.selection.start(row, col);
    }

    /// Updates the current selection's end position.
    pub fn update_selection(&mut self, row: usize, col: usize) {
        self.selection.update(row, col);
    }

    /// Converts window coordinates to cell position.
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

    /// Returns the current selection bounds as ((start_row, start_col), (end_row, end_col)).
    pub fn get_selection_bounds(&self) -> Option<((usize, usize), (usize, usize))> {
        self.selection.bounds()
    }

    /// Returns the selected text from the current display.
    pub fn get_selected_text(&self) -> Option<String> {
        // Use visible_bounds to get screen coordinates (not absolute scrollback coords)
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

        // Trim trailing whitespace from each line but keep newlines
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

    /// Returns the number of cached display rows (for debugging).
    pub fn display_rows_len(&self) -> usize {
        self.last_display_rows.len()
    }

    /// Clears the current selection.
    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }

    /// Clears the scrollback buffer.
    pub fn clear_scrollback(&mut self) {
        self.scrollback.clear();
    }

    /// Updates the URL hover state based on cursor position.
    /// Returns true if the hover state changed.
    pub fn update_url_hover(&mut self, row: usize, col: usize) -> bool {
        let was_hovered = self.urls.hovered_url().is_some();
        self.urls.update_hover(row, col);
        let is_hovered = self.urls.hovered_url().is_some();
        was_hovered != is_hovered || is_hovered
    }

    /// Clears the URL hover state.
    pub fn clear_url_hover(&mut self) {
        self.urls.clear_hover();
    }

    /// Returns the URL at the given cell position if any.
    pub fn url_at(&self, row: usize, col: usize) -> Option<String> {
        self.urls.url_at(row, col).map(|span| span.full_url())
    }

    /// Returns true if there's a hovered URL.
    pub fn has_hovered_url(&self) -> bool {
        self.urls.hovered_url().is_some()
    }

    // =========================================================================
    // Search Methods
    // =========================================================================

    /// Returns whether search mode is active.
    pub fn is_search_active(&self) -> bool {
        self.search.is_active()
    }

    /// Activates search mode.
    pub fn activate_search(&mut self) {
        self.search.activate();
    }

    /// Deactivates search mode.
    pub fn deactivate_search(&mut self) {
        self.search.deactivate();
    }

    /// Returns the current search query.
    pub fn search_query(&self) -> &str {
        self.search.query()
    }

    /// Returns the number of search matches.
    pub fn search_match_count(&self) -> usize {
        self.search.match_count()
    }

    /// Returns the current search match index.
    pub fn search_current_index(&self) -> usize {
        self.search.current_match_index()
    }

    /// Appends a character to the search query.
    pub fn search_push_char(&mut self, ch: char) {
        self.search.push_char(ch);
    }

    /// Removes the last character from the search query.
    pub fn search_pop_char(&mut self) {
        self.search.pop_char();
    }

    /// Moves to the next search match.
    pub fn search_next(&mut self) {
        self.search.next_match();
        self.scroll_to_current_match();
    }

    /// Moves to the previous search match.
    pub fn search_prev(&mut self) {
        self.search.prev_match();
        self.scroll_to_current_match();
    }

    /// Scrolls to make the current search match visible.
    fn scroll_to_current_match(&mut self) {
        if let Some(match_row) = self.search.current_match_row() {
            // scroll_to_row expects live_rows_len, not total_rows
            self.scrollback.scroll_to_row(match_row, self.view_rows);
        }
    }

    /// Toggles case sensitivity for search.
    pub fn search_toggle_case(&mut self) {
        self.search.toggle_case_sensitive();
    }

    /// Returns whether search is case-sensitive.
    pub fn is_search_case_sensitive(&self) -> bool {
        self.search.is_case_sensitive()
    }

    /// Returns a reference to the scrollback buffer (for search).
    pub fn scrollback(&self) -> &ScrollbackBuffer {
        &self.scrollback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_default() {
        let _renderer = Renderer::new();
        // Just verify construction works
    }

    #[test]
    fn test_window_to_cell() {
        let mut renderer = Renderer::new();
        renderer.set_dimensions(24, 80);

        // At origin
        assert_eq!(renderer.window_to_cell(0.0, 0.0), Some((0, 0)));

        // In the middle of first cell
        assert_eq!(renderer.window_to_cell(8.0, 8.0), Some((0, 0)));

        // Second cell
        assert_eq!(renderer.window_to_cell(17.0, 17.0), Some((1, 1)));
    }

    #[test]
    fn test_window_to_cell_out_of_bounds() {
        let mut renderer = Renderer::new();
        renderer.set_dimensions(24, 80);

        // Way out of bounds
        assert_eq!(renderer.window_to_cell(10000.0, 10000.0), None);
    }

    #[test]
    fn test_search_scroll_to_match() {
        use crate::renderer::scrollback::RowSnapshot;
        use vt100::Color;

        fn make_row(text: &str) -> RowSnapshot {
            let cells: Vec<_> = text
                .chars()
                .map(|ch| (ch, Color::Default, Color::Default))
                .collect();
            let tabs = vec![None; cells.len()];
            RowSnapshot::new(cells, tabs)
        }

        let mut renderer = Renderer::new();
        renderer.set_dimensions(24, 80); // 24 rows visible

        // Add 100 history rows with "test" on rows 10, 50, and 90
        let history: Vec<_> = (0..100)
            .map(|i| {
                if i == 10 || i == 50 || i == 90 {
                    make_row(&format!("line {} test", i))
                } else {
                    make_row(&format!("line {}", i))
                }
            })
            .collect();
        renderer.scrollback.add_history_rows(history, 100);

        // Simulate live rows (24 rows)
        let live_rows: Vec<_> = (0..24).map(|i| make_row(&format!("live {}", i))).collect();

        // Activate search and search for "test"
        renderer.activate_search();
        renderer.search_push_char('t');
        renderer.search_push_char('e');
        renderer.search_push_char('s');
        renderer.search_push_char('t');

        // Update matches
        let all_rows = renderer.scrollback.get_all_rows(&live_rows);
        renderer.search.update_matches(&all_rows);

        assert_eq!(renderer.search_match_count(), 3);
        assert_eq!(renderer.search.current_match_row(), Some(10));

        // Navigate to next match (should be row 50)
        renderer.search_next();
        assert_eq!(renderer.search.current_match_row(), Some(50));

        // Verify scrollback offset puts row 50 in view
        let view_start = renderer.scrollback.view_start();
        let view_end = view_start + 24;
        assert!(
            view_start <= 50 && 50 < view_end,
            "Row 50 should be visible. view_start={}, view_end={}, offset={}",
            view_start,
            view_end,
            renderer.scrollback.offset()
        );

        // Navigate to next match (should be row 90)
        renderer.search_next();
        assert_eq!(renderer.search.current_match_row(), Some(90));

        let view_start = renderer.scrollback.view_start();
        let view_end = view_start + 24;
        assert!(
            view_start <= 90 && 90 < view_end,
            "Row 90 should be visible. view_start={}, view_end={}, offset={}",
            view_start,
            view_end,
            renderer.scrollback.offset()
        );
    }
}
