//! Scrollback buffer management for the terminal.
//!
//! This module provides scrollback support by combining the terminal model's native scrollback
//! tracking with our own history storage. The model correctly tracks when lines scroll
//! off, but its API only allows viewing `view_rows` lines of history at a time.
//! We maintain our own copy for full history access (e.g., for search).

use std::collections::VecDeque;
use crate::core::color::Color;

use crate::constants::SCROLLBACK_CAPACITY;

use crate::core::grid::RowSnapshot;

/// Manages the scrollback buffer for terminal history.
///
/// Uses the model's scrollback length changes to detect when lines scroll off,
/// then stores those lines in our own history buffer. This gives us:
/// - Reliable detection (the model handles rapid output correctly)
/// - Full history access (for search and arbitrary scrolling)
pub struct ScrollbackBuffer {
    /// Historical lines that have scrolled off the top.
    history: VecDeque<RowSnapshot>,
    /// Maximum number of history lines to keep.
    capacity: usize,
    /// Current scroll offset (0 = live view, >0 = scrolled into history).
    offset: usize,
    /// Number of rows in the terminal view.
    view_rows: usize,
    /// Number of columns in the terminal view.
    view_cols: usize,
    /// Terminal model's scrollback length from the last frame.
    last_scrollback_len: usize,
}

impl Default for ScrollbackBuffer {
    fn default() -> Self {
        ScrollbackBuffer::new()
    }
}

impl ScrollbackBuffer {
    /// Creates a new scrollback buffer with the default capacity.
    pub fn new() -> Self {
        ScrollbackBuffer::with_capacity(SCROLLBACK_CAPACITY)
    }

    /// Creates a new scrollback buffer with the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        ScrollbackBuffer {
            history: VecDeque::new(),
            capacity,
            offset: 0,
            view_rows: 0,
            view_cols: 0,
            last_scrollback_len: 0,
        }
    }

    /// Updates the view dimensions.
    /// Note: We preserve original history row widths on resize.
    /// Display handles truncation/padding in get_display_rows().
    pub fn set_dimensions(&mut self, rows: usize, cols: usize) {
        self.view_rows = rows;
        self.view_cols = cols;
    }

    /// Adjusts a row's width for display by padding with spaces or truncating.
    /// Returns a new RowSnapshot, preserving the original.
    fn adjust_row_for_display(row: &RowSnapshot, cols: usize) -> RowSnapshot {
        let current_len = row.cells.len();

        if current_len == cols {
            row.clone()
        } else if current_len < cols {
            // Pad with spaces
            let mut cells = row.cells.clone();
            cells.reserve(cols - current_len);
            for _ in current_len..cols {
                cells.push((' ', Color::Default, Color::Default));
            }
            let mut tabs = row.tabs.clone();
            tabs.resize(cols, None);
            RowSnapshot::new(cells, tabs)
        } else {
            // Truncate for display (original row is preserved)
            let cells = row.cells[..cols].to_vec();
            let tabs = row.tabs[..cols].to_vec();
            RowSnapshot::new(cells, tabs)
        }
    }

    /// Clears the history buffer and resets the offset.
    pub fn clear(&mut self) {
        self.history.clear();
        self.offset = 0;
        self.last_scrollback_len = 0;
    }

    /// Resets the scrollback tracking without clearing history.
    /// Call this after terminal resize when the model's scrollback may have been reset.
    pub fn reset_scrollback_tracking(&mut self, new_len: usize) {
        self.last_scrollback_len = new_len;
    }

    /// Returns the last known scrollback length from the terminal model.
    pub fn last_scrollback_len(&self) -> usize {
        self.last_scrollback_len
    }

    /// Adds newly captured history rows and updates the scrollback length.
    ///
    /// `new_rows`: Rows captured from the terminal model that just scrolled off (oldest first)
    /// `current_len`: Current scrollback length from the terminal model
    pub fn add_history_rows(&mut self, new_rows: Vec<RowSnapshot>, current_len: usize) {
        let new_lines = new_rows.len();

        // Add new rows to history - we keep their original width
        for row in new_rows {
            self.history.push_back(row);
            if self.history.len() > self.capacity {
                self.history.pop_front();
            }
        }

        // If user is scrolled up, adjust offset to maintain view position
        if self.offset > 0 && new_lines > 0 {
            self.offset = (self.offset + new_lines).min(self.history.len());
        }

        self.last_scrollback_len = current_len;
    }

    /// Scrolls by the given number of lines (positive = up into history, negative = down toward live).
    pub fn scroll_by(&mut self, delta: isize) {
        if self.history.is_empty() {
            self.offset = 0;
            return;
        }

        let max_offset = self.history.len();
        let new_offset = (self.offset as isize + delta).clamp(0, max_offset as isize);
        self.offset = new_offset as usize;
    }

    /// Returns the current scroll offset.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Snaps to the bottom (live view), clearing any scroll offset.
    pub fn snap_to_bottom(&mut self) {
        self.offset = 0;
    }

    /// Returns true if scrolled up (not at live view).
    pub fn is_scrolled_up(&self) -> bool {
        self.offset > 0
    }

    /// Returns the number of lines in our history buffer.
    pub fn len(&self) -> usize {
        self.history.len()
    }

    /// Returns true if the history buffer is empty.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }

    /// Returns the absolute row index of the first visible row.
    /// This is used to convert display row indices to absolute indices for search.
    pub fn view_start(&self) -> usize {
        // When offset=0, we're at live view, so view_start = history.len()
        // When offset>0, we're scrolled back, so view_start = history.len() - offset
        self.history.len().saturating_sub(self.offset)
    }

    /// Returns the rows to display based on current scroll offset.
    ///
    /// The conceptual model is a combined buffer: [history...] + [live_rows...]
    /// - offset=0: show the last view_rows (the live view)
    /// - offset=N: scroll N lines up into history
    ///
    /// Rows are adjusted to current view_cols for display (padded or truncated),
    /// but the original row data in history is preserved.
    pub fn get_display_rows(&self, live_rows: &[RowSnapshot], cols: usize) -> Vec<RowSnapshot> {
        if self.offset == 0 || self.history.is_empty() {
            // Live view - adjust live rows to display width
            return live_rows
                .iter()
                .map(|row| Self::adjust_row_for_display(row, cols))
                .collect();
        }

        // Combined length: history + live
        let total_len = self.history.len() + live_rows.len();

        // The "end" of our view window when not scrolled would be at total_len
        // When scrolled by offset, we view starting from (total_len - view_rows - offset)
        let view_start = total_len
            .saturating_sub(self.view_rows)
            .saturating_sub(self.offset);

        let mut display = Vec::with_capacity(self.view_rows);

        for i in 0..self.view_rows {
            let idx = view_start + i;

            if idx < self.history.len() {
                // This index is in history
                if let Some(row) = self.history.get(idx) {
                    display.push(Self::adjust_row_for_display(row, cols));
                } else {
                    display.push(RowSnapshot::blank(cols));
                }
            } else {
                // This index is in live_rows
                let live_idx = idx - self.history.len();
                if live_idx < live_rows.len() {
                    display.push(Self::adjust_row_for_display(&live_rows[live_idx], cols));
                } else {
                    display.push(RowSnapshot::blank(cols));
                }
            }
        }

        display
    }

    /// Returns all rows: history + live rows combined.
    /// Used for searching the entire scrollback.
    pub fn get_all_rows(&self, live_rows: &[RowSnapshot]) -> Vec<RowSnapshot> {
        let mut all = Vec::with_capacity(self.history.len() + live_rows.len());
        for row in &self.history {
            all.push(row.clone());
        }
        all.extend_from_slice(live_rows);
        all
    }

    /// Scrolls to make a specific absolute row index visible.
    /// Returns the new offset.
    pub fn scroll_to_row(&mut self, row: usize, live_rows_len: usize) -> usize {
        let total_len = self.history.len() + live_rows_len;
        if total_len <= self.view_rows {
            // Everything fits on screen, no scrolling needed
            self.offset = 0;
            return self.offset;
        }

        // Calculate what offset would put this row in the middle of the view
        // The view shows rows from (total_len - view_rows - offset) to (total_len - offset - 1)
        // We want row to be visible, ideally in the middle
        let middle_of_view = self.view_rows / 2;

        // If row is near the end, we need less offset
        // If row is near the start, we need more offset
        let rows_after_row = total_len.saturating_sub(row + 1);
        let ideal_rows_below = self.view_rows.saturating_sub(middle_of_view + 1);

        if rows_after_row < ideal_rows_below {
            // Row is near the end, show live view or close to it
            self.offset =
                rows_after_row.saturating_sub(self.view_rows.saturating_sub(middle_of_view));
        } else {
            // Normal case: offset to put row in middle
            self.offset = rows_after_row.saturating_sub(ideal_rows_below);
        }

        // Clamp to valid range
        let max_offset = self.history.len();
        if self.offset > max_offset {
            self.offset = max_offset;
        }

        self.offset
    }
}
