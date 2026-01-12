//! Terminal view state management.
//!
//! `TerminalView` manages UI state including scrolling, selection, URL detection,
//! and search. It builds `RenderFrame`s that the `Renderer` can paint.

mod frame;
mod scroll;
mod search;
mod selection;
mod urls;

use crate::core::color_codes::ColorCodeManager;
use crate::core::grid::RowSnapshot;
use crate::core::search::SearchState;
use crate::core::selection::SelectionManager;
use crate::core::url::UrlManager;

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

    pub fn set_dimensions(&mut self, rows: usize, cols: usize) {
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

    fn max_scroll_offset(&self) -> usize {
        self.max_scroll_offset_with_len(self.last_buffer_len)
    }

    fn max_scroll_offset_with_len(&self, buffer_len: usize) -> usize {
        buffer_len.saturating_sub(self.view_rows)
    }

    fn scroll_to_row(&mut self, row: usize) {
        let buffer_len = self.last_buffer_len;
        if self.view_rows == 0 || buffer_len == 0 {
            self.scroll_offset = 0;
            return;
        }

        let max_start = buffer_len.saturating_sub(self.view_rows);
        let desired_start = row.saturating_sub(self.view_rows / 2);
        let window_start = desired_start.min(max_start);

        self.scroll_offset = buffer_len.saturating_sub(self.view_rows + window_start);
        self.scroll_offset = self.scroll_offset.min(self.max_scroll_offset());
    }
}
