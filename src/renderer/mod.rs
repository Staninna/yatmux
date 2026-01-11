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

/// Search highlight colors.
const SEARCH_MATCH_BG: u32 = 0x4A4A00; // Dark yellow for regular matches
const SEARCH_CURRENT_BG: u32 = 0x806000; // Brighter yellow for current match

struct RenderFrame {
    cursor: (u16, u16),
    display_rows: Vec<RowSnapshot>,
    rows: usize,
    cols: usize,
    view_start: usize,
    show_cursor: bool,
}

/// The main terminal renderer.
pub struct Renderer {
    selection: SelectionManager,
    urls: UrlManager,
    search: SearchState,
    view_rows: usize,
    view_cols: usize,
    /// Current scroll offset (0 = live view, >0 = scrolled back).
    scroll_offset: usize,
    /// Total number of rows (scrollback + viewport) in last frame.
    last_buffer_len: usize,
    /// Cached display rows from last render (for copy operations).
    last_display_rows: Vec<RowSnapshot>,
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
            urls: UrlManager::new(),
            search: SearchState::new(),
            view_rows: 0,
            view_cols: 0,
            scroll_offset: 0,
            last_buffer_len: 0,
            last_display_rows: Vec::new(),
        }
    }

    /// Updates the view dimensions.
    fn set_dimensions(&mut self, rows: usize, cols: usize) {
        if self.view_rows != rows || self.view_cols != cols {
            self.view_rows = rows;
            self.view_cols = cols;
            self.selection.set_dimensions(rows, cols);
            self.urls.set_dimensions(rows);

            // Clamp scroll offset to new viewport size.
            self.scroll_offset = self.scroll_offset.min(self.max_scroll_offset());
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
        terminal: &Terminal,
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

        let frame = self.build_frame(terminal, rows, cols)?;
        self.paint_frame(&mut buffer, buffer_width, buffer_height, &frame, palette);

        buffer
            .present()
            .map_err(|e| anyhow::anyhow!("softbuffer present failed: {e:?}"))?;

        Ok(())
    }

    fn build_frame(
        &mut self,
        terminal: &Terminal,
        rows: usize,
        cols: usize,
    ) -> Result<RenderFrame> {
        let (mut all_rows, cursor, cursor_visible) = terminal.all_rows(cols);

        // Maintain scroll position when new output arrives.
        if self.scroll_offset > 0 {
            let new_lines = all_rows.len().saturating_sub(self.last_buffer_len);
            if new_lines > 0 {
                self.scroll_offset = (self.scroll_offset + new_lines).min(self.max_scroll_offset());
            }
        }
        self.last_buffer_len = all_rows.len();

        // Ensure we always have at least `rows` rows to show (pad at top).
        if all_rows.len() < rows {
            let mut padded = Vec::with_capacity(rows);
            for _ in 0..(rows - all_rows.len()) {
                padded.push(RowSnapshot::blank(cols));
            }
            padded.append(&mut all_rows);
            all_rows = padded;
        }

        let buffer_len = all_rows.len();

        // Clamp scroll offset to valid range.
        self.scroll_offset = self
            .scroll_offset
            .min(self.max_scroll_offset_with_len(buffer_len));

        // Visible window start in absolute coordinates.
        let window_start = buffer_len.saturating_sub(rows + self.scroll_offset);

        let display_rows: Vec<RowSnapshot> = all_rows
            .iter()
            .skip(window_start)
            .take(rows)
            .cloned()
            .collect();

        self.selection
            .set_scroll_state(self.scroll_offset, buffer_len);

        // Cache display rows for copy operations.
        self.last_display_rows = display_rows.clone();

        // Update search matches for ALL rows when search is active.
        if self.search.is_active() {
            self.search.update_matches(&all_rows);
        }

        // Detect URLs in each visible row.
        for (row_idx, row_data) in display_rows.iter().enumerate() {
            let text: String = row_data.cells.iter().map(|(ch, _, _)| ch).collect();
            self.urls.update_row(row_idx, &text);
        }

        Ok(RenderFrame {
            cursor,
            display_rows,
            rows,
            cols,
            view_start: window_start,
            show_cursor: self.scroll_offset == 0 && cursor_visible,
        })
    }

    fn paint_frame(
        &self,
        buffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        frame: &RenderFrame,
        palette: &[u32; 256],
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
                let selected = self.selection.is_selected(row_idx, col);
                let is_url = self.urls.is_url(row_idx, col);
                let is_url_hovered = self.urls.is_hovered(row_idx, col);

                let search_match = self.search.is_match(row_idx, col, frame.view_start);

                self.draw_cell(
                    buffer,
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

        if self.search.is_active() {
            self.draw_search_bar(buffer, buffer_width, buffer_height);
        }
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

    /// Scrolls the viewport by the given number of lines.
    pub fn scrollback_scroll_by(&mut self, delta_lines: isize) {
        let max_offset = self.max_scroll_offset();
        let new_offset = (self.scroll_offset as isize + delta_lines).clamp(0, max_offset as isize);
        self.scroll_offset = new_offset as usize;
    }

    /// Snaps scrollback to the bottom (live view).
    pub fn scrollback_snap_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    /// Returns true if scrolled up in history.
    pub fn is_scrolled_up(&self) -> bool {
        self.scroll_offset > 0
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

    /// Clears local scroll state (snaps back to live view).
    pub fn clear_scrollback(&mut self) {
        self.scroll_offset = 0;
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
            self.scroll_to_row(match_row);
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

        let mut renderer = Renderer::new();
        renderer.set_dimensions(24, 80); // 24 rows visible

        // Create a combined buffer: 100 history + 24 live
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

        renderer.last_buffer_len = all_rows.len();

        // Activate search and search for "test"
        renderer.activate_search();
        for ch in "test".chars() {
            renderer.search_push_char(ch);
        }

        renderer.search.update_matches(&all_rows);

        assert_eq!(renderer.search_match_count(), 3);
        assert_eq!(renderer.search.current_match_row(), Some(10));

        // Navigate to next match (should be row 50)
        renderer.search_next();
        assert_eq!(renderer.search.current_match_row(), Some(50));

        let view_start = renderer
            .last_buffer_len
            .saturating_sub(renderer.view_rows + renderer.scroll_offset);
        let view_end = view_start + renderer.view_rows;
        assert!(
            view_start <= 50 && 50 < view_end,
            "Row 50 should be visible. view_start={}, view_end={}, offset={}",
            view_start,
            view_end,
            renderer.scroll_offset
        );

        // Navigate to next match (should be row 90)
        renderer.search_next();
        assert_eq!(renderer.search.current_match_row(), Some(90));

        let view_start = renderer
            .last_buffer_len
            .saturating_sub(renderer.view_rows + renderer.scroll_offset);
        let view_end = view_start + renderer.view_rows;
        assert!(
            view_start <= 90 && 90 < view_end,
            "Row 90 should be visible. view_start={}, view_end={}, offset={}",
            view_start,
            view_end,
            renderer.scroll_offset
        );
    }
}
