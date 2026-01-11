//! Scrollback buffer management for the terminal.
//!
//! This module provides scrollback support by combining vt100's native scrollback
//! tracking with our own history storage. vt100 correctly tracks when lines scroll
//! off, but its API only allows viewing `view_rows` lines of history at a time.
//! We maintain our own copy for full history access (e.g., for search).

use std::collections::VecDeque;
use vt100::Color;

use crate::constants::SCROLLBACK_CAPACITY;

use crate::core::grid::RowSnapshot;

/// Manages the scrollback buffer for terminal history.
///
/// Uses vt100's scrollback length changes to detect when lines scroll off,
/// then stores those lines in our own history buffer. This gives us:
/// - Reliable detection (vt100 handles rapid output correctly)
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
    /// vt100's scrollback length from the last frame.
    last_vt100_scrollback_len: usize,
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
            last_vt100_scrollback_len: 0,
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
        self.last_vt100_scrollback_len = 0;
    }

    /// Resets the vt100 scrollback tracking without clearing history.
    /// Call this after terminal resize when vt100's scrollback may have been reset.
    pub fn reset_vt100_tracking(&mut self, new_vt100_len: usize) {
        self.last_vt100_scrollback_len = new_vt100_len;
    }

    /// Returns the last known vt100 scrollback length.
    pub fn last_vt100_scrollback_len(&self) -> usize {
        self.last_vt100_scrollback_len
    }

    /// Adds newly captured history rows and updates the vt100 scrollback length.
    ///
    /// `new_rows`: Rows captured from vt100 that just scrolled off (oldest first)
    /// `current_vt100_len`: Current scrollback length from vt100
    pub fn add_history_rows(&mut self, new_rows: Vec<RowSnapshot>, current_vt100_len: usize) {
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

        self.last_vt100_scrollback_len = current_vt100_len;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_row(text: &str) -> RowSnapshot {
        let cells: Vec<_> = text
            .chars()
            .map(|ch| (ch, Color::Default, Color::Default))
            .collect();
        let tabs = vec![None; cells.len()];
        RowSnapshot::new(cells, tabs)
    }

    fn make_rows(texts: &[&str]) -> Vec<RowSnapshot> {
        texts.iter().map(|t| make_row(t)).collect()
    }

    #[test]
    fn test_scrollback_buffer_new() {
        let buffer = ScrollbackBuffer::new();
        assert!(buffer.is_empty());
        assert_eq!(buffer.offset(), 0);
    }

    #[test]
    fn test_add_history_rows() {
        let mut buffer = ScrollbackBuffer::with_capacity(100);
        buffer.set_dimensions(3, 80);

        // Simulate adding rows that scrolled off
        buffer.add_history_rows(make_rows(&["Line 1", "Line 2"]), 2);

        assert_eq!(buffer.len(), 2);
        assert_eq!(buffer.last_vt100_scrollback_len(), 2);
    }

    #[test]
    fn test_scroll_by() {
        let mut buffer = ScrollbackBuffer::with_capacity(100);
        buffer.set_dimensions(3, 80);

        // Add some history
        buffer.add_history_rows(
            make_rows(&["Line 1", "Line 2", "Line 3", "Line 4", "Line 5"]),
            5,
        );

        assert_eq!(buffer.len(), 5);

        // Scroll up
        buffer.scroll_by(1);
        assert_eq!(buffer.offset(), 1);

        buffer.scroll_by(2);
        assert_eq!(buffer.offset(), 3);

        // Can't scroll past history
        buffer.scroll_by(100);
        assert_eq!(buffer.offset(), 5);

        // Scroll back down
        buffer.scroll_by(-2);
        assert_eq!(buffer.offset(), 3);

        buffer.scroll_by(-100);
        assert_eq!(buffer.offset(), 0);
    }

    #[test]
    fn test_view_start() {
        let mut buffer = ScrollbackBuffer::with_capacity(100);
        buffer.set_dimensions(3, 80);

        // Add history
        buffer.add_history_rows(make_rows(&[""; 10]), 10);
        assert_eq!(buffer.len(), 10);

        // At live view
        assert_eq!(buffer.view_start(), 10);

        // Scrolled up by 3
        buffer.scroll_by(3);
        assert_eq!(buffer.view_start(), 7);

        // Scrolled to top
        buffer.scroll_by(100);
        assert_eq!(buffer.view_start(), 0);
    }

    #[test]
    fn test_get_display_rows_live() {
        let mut buffer = ScrollbackBuffer::with_capacity(100);
        buffer.set_dimensions(3, 80);

        let live_rows = make_rows(&["Live 1", "Live 2", "Live 3"]);

        // No history, offset 0 - should return live rows (padded to display width)
        let display = buffer.get_display_rows(&live_rows, 80);
        assert_eq!(display.len(), 3);
        assert_eq!(display[0].text().trim_end(), "Live 1");
        assert_eq!(display[0].cells.len(), 80); // Padded to display width
    }

    #[test]
    fn test_get_display_rows_scrolled() {
        let mut buffer = ScrollbackBuffer::with_capacity(100);
        buffer.set_dimensions(3, 80);

        // Add history
        buffer.add_history_rows(make_rows(&["Hist 1", "Hist 2"]), 2);

        // Scroll up
        buffer.scroll_by(1);

        let live_rows = make_rows(&["Live 1", "Live 2", "Live 3"]);
        let display = buffer.get_display_rows(&live_rows, 80);

        // Should show part history, part live
        assert_eq!(display.len(), 3);
        assert_eq!(display[0].text().trim_end(), "Hist 2");
        assert_eq!(display[1].text().trim_end(), "Live 1");
        assert_eq!(display[2].text().trim_end(), "Live 2");
    }

    #[test]
    fn test_get_all_rows() {
        let mut buffer = ScrollbackBuffer::with_capacity(100);
        buffer.set_dimensions(3, 80);

        // Add history
        buffer.add_history_rows(make_rows(&["Hist 1", "Hist 2"]), 2);

        let live_rows = make_rows(&["Live 1", "Live 2"]);
        let all = buffer.get_all_rows(&live_rows);

        assert_eq!(all.len(), 4);
        assert_eq!(all[0].text().trim_end(), "Hist 1");
        assert_eq!(all[1].text().trim_end(), "Hist 2");
        assert_eq!(all[2].text().trim_end(), "Live 1");
        assert_eq!(all[3].text().trim_end(), "Live 2");
    }

    #[test]
    fn test_clear() {
        let mut buffer = ScrollbackBuffer::with_capacity(100);
        buffer.add_history_rows(make_rows(&[""; 5]), 5);
        buffer.scroll_by(3);

        buffer.clear();
        assert!(buffer.is_empty());
        assert_eq!(buffer.offset(), 0);
    }

    #[test]
    fn test_capacity_limit() {
        let mut buffer = ScrollbackBuffer::with_capacity(3);
        buffer.set_dimensions(3, 80);

        // Add more history than capacity
        buffer.add_history_rows(
            make_rows(&["Line 1", "Line 2", "Line 3", "Line 4", "Line 5"]),
            5,
        );

        // Only 3 lines kept
        assert_eq!(buffer.len(), 3);

        // Should have the last 3
        let live_rows = make_rows(&[""]);
        let all = buffer.get_all_rows(&live_rows);
        assert_eq!(all[0].text().trim_end(), "Line 3");
        assert_eq!(all[1].text().trim_end(), "Line 4");
        assert_eq!(all[2].text().trim_end(), "Line 5");
    }

    #[test]
    fn test_offset_maintained_when_scrolled_up() {
        let mut buffer = ScrollbackBuffer::with_capacity(100);
        buffer.set_dimensions(3, 80);

        // Add initial history
        buffer.add_history_rows(make_rows(&["Line 1", "Line 2", "Line 3"]), 3);

        // User scrolls up
        buffer.scroll_by(2);
        assert_eq!(buffer.offset(), 2);

        // More content scrolls off
        buffer.add_history_rows(make_rows(&["Line 4", "Line 5"]), 5);

        // Offset should increase to maintain view position
        assert_eq!(buffer.offset(), 4);
    }

    #[test]
    fn test_offset_not_changed_when_at_live() {
        let mut buffer = ScrollbackBuffer::with_capacity(100);
        buffer.set_dimensions(3, 80);

        // Add history
        buffer.add_history_rows(make_rows(&[""; 3]), 3);

        // At live view (offset = 0)
        assert_eq!(buffer.offset(), 0);

        // More history
        buffer.add_history_rows(make_rows(&[""; 2]), 5);

        // Offset should stay at 0
        assert_eq!(buffer.offset(), 0);
    }

    #[test]
    fn test_row_snapshot() {
        let row = make_row("Hello");
        assert_eq!(row.text(), "Hello");

        let blank = RowSnapshot::blank(10);
        assert_eq!(blank.cells.len(), 10);
        assert_eq!(blank.text(), "          ");
    }

    #[test]
    fn test_snap_to_bottom() {
        let mut buffer = ScrollbackBuffer::new();

        // Add some history
        buffer.add_history_rows(vec![make_row("Line 1"), make_row("Line 2")], 2);

        // Scroll up
        buffer.scroll_by(2);
        assert!(buffer.is_scrolled_up());
        assert_eq!(buffer.offset(), 2);

        // Snap to bottom
        buffer.snap_to_bottom();
        assert!(!buffer.is_scrolled_up());
        assert_eq!(buffer.offset(), 0);
    }

    #[test]
    fn test_scroll_to_row() {
        let mut buffer = ScrollbackBuffer::new();
        buffer.set_dimensions(24, 80); // 24 rows visible

        // Add 100 history rows
        let history: Vec<_> = (0..100)
            .map(|i| make_row(&format!("history {}", i)))
            .collect();
        buffer.add_history_rows(history, 100);

        // 24 live rows (indices 100-123 in the combined buffer)
        let live_rows_len = 24;

        // Scroll to row 50 (in history)
        buffer.scroll_to_row(50, live_rows_len);

        // Row 50 should be visible (roughly in the middle)
        let view_start = buffer.view_start();
        let view_end = view_start + 24;

        assert!(
            view_start <= 50 && 50 < view_end,
            "Row 50 should be visible. view_start={}, view_end={}, offset={}",
            view_start,
            view_end,
            buffer.offset()
        );

        // Scroll to row 5 (near the beginning)
        buffer.scroll_to_row(5, live_rows_len);
        let view_start = buffer.view_start();
        let view_end = view_start + 24;
        assert!(
            view_start <= 5 && 5 < view_end,
            "Row 5 should be visible. view_start={}, view_end={}",
            view_start,
            view_end
        );

        // Scroll to row 120 (in live area)
        buffer.scroll_to_row(120, live_rows_len);
        let view_start = buffer.view_start();
        let view_end = view_start + 24;
        assert!(
            view_start <= 120 && 120 < view_end,
            "Row 120 should be visible. view_start={}, view_end={}",
            view_start,
            view_end
        );
    }

    #[test]
    fn test_resize_preserves_history_data() {
        let mut buffer = ScrollbackBuffer::new();
        buffer.set_dimensions(24, 40); // Start with 40 columns

        // Add history with different widths
        buffer.add_history_rows(
            vec![
                make_row("Hello World"),
                make_row("This is a longer line that has more text"),
            ],
            2,
        );

        // Verify initial state - rows keep their original width
        assert_eq!(buffer.len(), 2);
        let live = make_rows(&["Live"]);
        let all = buffer.get_all_rows(&live);
        assert_eq!(all[0].cells.len(), 11); // "Hello World" = 11 chars
        assert_eq!(all[1].cells.len(), 40); // 40 chars

        // Resize to 80 columns - original data unchanged
        buffer.set_dimensions(24, 80);
        let all = buffer.get_all_rows(&live);
        assert_eq!(all[0].cells.len(), 11); // Still original width
        assert_eq!(all[1].cells.len(), 40);
        assert_eq!(all[0].text(), "Hello World");

        // But display rows are adjusted to display width
        buffer.scroll_by(2);
        let display = buffer.get_display_rows(&live, 80);
        assert_eq!(display[0].cells.len(), 80); // Padded for display
        assert_eq!(display[0].text().trim_end(), "Hello World");

        // Resize to 20 columns - original data STILL preserved
        buffer.set_dimensions(24, 20);
        let all = buffer.get_all_rows(&live);
        assert_eq!(all[0].cells.len(), 11); // Original preserved
        assert_eq!(all[1].cells.len(), 40); // Original preserved - NOT truncated!
        assert_eq!(all[1].text(), "This is a longer line that has more text");

        // But display is truncated
        let display = buffer.get_display_rows(&live, 20);
        assert_eq!(display[0].cells.len(), 20);
        assert_eq!(display[1].cells.len(), 20);
        assert_eq!(display[1].text(), "This is a longer lin"); // Truncated for display only

        // Resize back to 80 - data is RESTORED because we never truncated it
        buffer.set_dimensions(24, 80);
        let all = buffer.get_all_rows(&live);
        assert_eq!(all[1].text(), "This is a longer line that has more text");
    }

    #[test]
    fn test_resize_preserves_scroll_offset() {
        let mut buffer = ScrollbackBuffer::new();
        buffer.set_dimensions(24, 80);

        // Add lots of history
        let history: Vec<_> = (0..100).map(|i| make_row(&format!("Line {}", i))).collect();
        buffer.add_history_rows(history, 100);

        // Scroll up
        buffer.scroll_by(50);
        assert_eq!(buffer.offset(), 50);

        // Resize - offset should be preserved
        buffer.set_dimensions(30, 100);
        assert_eq!(buffer.offset(), 50);

        // History should still be there
        assert_eq!(buffer.len(), 100);
    }
}
