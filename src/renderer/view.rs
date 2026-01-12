//! Terminal view state management.
//!
//! `TerminalView` manages UI state including scrolling, selection, URL detection,
//! and search. It builds `RenderFrame`s that the `Renderer` can paint.

use crate::core::color_codes::ColorCodeManager;
use crate::core::grid::RowSnapshot;
use crate::core::search::SearchState;
use crate::core::selection::SelectionManager;
use crate::core::url::UrlManager;
use crate::terminal::Terminal;

use anyhow::Result;

use super::RenderFrame;

/// Maintains terminal UI state and builds `RenderFrame`s.
///
/// This is intentionally separate from pixel painting, which is handled by `Renderer`.
pub struct TerminalView {
    pub(crate) selection: SelectionManager,
    pub(crate) urls: UrlManager,
    pub(crate) color_codes: ColorCodeManager,
    pub(crate) search: SearchState,
    pub(crate) view_rows: usize,
    pub(crate) view_cols: usize,
    /// Current scroll offset (0 = live view, >0 = scrolled back).
    pub(crate) scroll_offset: usize,
    /// Total number of rows (scrollback + viewport) in last frame.
    pub(crate) last_buffer_len: usize,
    /// Cached display rows from last frame (for copy operations).
    pub(crate) last_display_rows: Vec<RowSnapshot>,
    /// Cached search inputs to avoid re-indexing every frame.
    last_search_query: String,
    last_search_terminal_generation: u64,
    last_search_case_sensitive: bool,
    last_search_regex_mode: bool,
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
            last_search_regex_mode: false,
        }
    }

    pub(crate) fn set_dimensions(&mut self, rows: usize, cols: usize) {
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

    pub(crate) fn build_frame(
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
            let regex_mode = self.search.mode() == crate::core::search::SearchMode::Regex;

            if query != self.last_search_query
                || terminal_generation != self.last_search_terminal_generation
                || case_sensitive != self.last_search_case_sensitive
                || regex_mode != self.last_search_regex_mode
            {
                let all_rows = terminal.rows_in_range(0, buffer_len, cols);
                self.search.update_matches(&all_rows);
                self.last_search_query = query;
                self.last_search_terminal_generation = terminal_generation;
                self.last_search_case_sensitive = case_sensitive;
                self.last_search_regex_mode = regex_mode;
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

    // =========================================================================
    // Scrollback Methods
    // =========================================================================

    pub fn scrollback_scroll_by(&mut self, delta_lines: isize) {
        let max_offset = self.max_scroll_offset();
        let new_offset = (self.scroll_offset as isize + delta_lines).clamp(0, max_offset as isize);
        self.scroll_offset = new_offset as usize;
    }

    pub fn scrollback_scroll_to(&mut self, offset: usize) {
        let max_offset = self.max_scroll_offset();
        self.scroll_offset = offset.min(max_offset);
    }

    pub fn scrollback_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn scrollback_snap_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn is_scrolled_up(&self) -> bool {
        self.scroll_offset > 0
    }

    pub fn clear_scrollback(&mut self) {
        self.scroll_offset = 0;
    }

    // =========================================================================
    // Selection Methods
    // =========================================================================

    pub fn start_selection(&mut self, row: usize, col: usize) {
        self.selection.start(row, col);
    }

    pub fn update_selection(&mut self, row: usize, col: usize) {
        self.selection.update(row, col);
    }

    pub fn window_to_cell(
        &self,
        x: f64,
        y: f64,
        cell_w: usize,
        cell_h: usize,
    ) -> Option<(usize, usize)> {
        if self.view_rows == 0 || self.view_cols == 0 {
            return None;
        }

        let cell_w = cell_w.max(1);
        let cell_h = cell_h.max(1);

        let col = (x as usize) / cell_w;
        let row = (y as usize) / cell_h;

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

    // =========================================================================
    // URL Methods
    // =========================================================================

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

    pub fn search_toggle_regex(&mut self) {
        self.search.toggle_mode();
    }

    pub fn is_search_regex(&self) -> bool {
        self.search.mode() == crate::core::search::SearchMode::Regex
    }

    pub fn is_search_regex_valid(&self) -> bool {
        self.search.is_regex_valid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::grid::RowSnapshot;
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

    #[test]
    fn test_view_default() {
        let _view = TerminalView::new();
    }

    #[test]
    fn test_window_to_cell() {
        let mut view = TerminalView::new();
        view.set_dimensions(24, 80);

        let cell = 8 * 2;
        assert_eq!(view.window_to_cell(0.0, 0.0, cell, cell), Some((0, 0)));
        assert_eq!(view.window_to_cell(8.0, 8.0, cell, cell), Some((0, 0)));
        assert_eq!(view.window_to_cell(17.0, 17.0, cell, cell), Some((1, 1)));
    }

    #[test]
    fn test_window_to_cell_out_of_bounds() {
        let mut view = TerminalView::new();
        view.set_dimensions(24, 80);
        let cell = 8 * 2;
        assert_eq!(view.window_to_cell(10000.0, 10000.0, cell, cell), None);
    }

    #[test]
    fn test_search_scroll_to_match() {
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
