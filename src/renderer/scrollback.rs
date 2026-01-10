//! Scrollback buffer management for the terminal.
//!
//! This module handles storing and retrieving historical terminal rows
//! for scrollback functionality.

use std::collections::VecDeque;

use vt100::Color;

use crate::constants::SCROLLBACK_CAPACITY;

/// Cell data: character, foreground color, background color.
pub type CellData = (char, Color, Color);

/// A snapshot of a single terminal row.
#[derive(Clone)]
pub struct RowSnapshot {
    pub cells: Vec<CellData>,
    pub tabs: Vec<Option<(usize, usize)>>,
}

impl RowSnapshot {
    /// Creates a blank row with the specified number of columns.
    pub fn blank(cols: usize) -> Self {
        RowSnapshot {
            cells: vec![(' ', Color::Default, Color::Default); cols],
            tabs: vec![None; cols],
        }
    }

    /// Creates a new row from cell data and tab information.
    pub fn new(cells: Vec<CellData>, tabs: Vec<Option<(usize, usize)>>) -> Self {
        RowSnapshot { cells, tabs }
    }
}

/// Manages the scrollback buffer for terminal history.
pub struct ScrollbackBuffer {
    buffer: VecDeque<RowSnapshot>,
    capacity: usize,
    offset: usize,
    view_rows: usize,
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
            buffer: VecDeque::new(),
            capacity,
            offset: 0,
            view_rows: 0,
        }
    }

    /// Updates the view dimensions. Clears the buffer if dimensions change.
    pub fn set_dimensions(&mut self, rows: usize, _cols: usize) {
        if self.view_rows != rows {
            self.view_rows = rows;
            self.buffer.clear();
            self.offset = 0;
        }
    }

    /// Clears the buffer and resets the offset.
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.offset = 0;
    }

    /// Pushes a row into the scrollback buffer.
    #[allow(dead_code)]
    pub fn push_row(&mut self, row: RowSnapshot) {
        self.buffer.push_back(row);
        if self.buffer.len() > self.capacity {
            self.buffer.pop_front();
        }
        self.clamp_offset();
    }

    /// Pushes multiple rows into the scrollback buffer.
    pub fn push_rows(&mut self, rows: &[RowSnapshot]) {
        for row in rows {
            self.buffer.push_back(row.clone());
            if self.buffer.len() > self.capacity {
                self.buffer.pop_front();
            }
        }
        self.clamp_offset();
    }

    /// Scrolls by the given number of lines (positive = up, negative = down).
    pub fn scroll_by(&mut self, delta: isize) {
        if self.buffer.len() <= self.view_rows {
            return;
        }
        let max_offset = self.max_offset();
        let new_offset = (self.offset as isize + delta).clamp(0, max_offset as isize);
        self.offset = new_offset as usize;
    }

    /// Returns the current scroll offset.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Returns the maximum valid scroll offset.
    fn max_offset(&self) -> usize {
        self.buffer.len().saturating_sub(self.view_rows)
    }

    /// Clamps the offset to valid bounds.
    fn clamp_offset(&mut self) {
        let max = self.max_offset();
        if self.offset > max {
            self.offset = max;
        }
    }

    /// Returns the rows to display based on current offset.
    /// If offset is 0, returns None (use live data).
    /// Otherwise returns the historical rows to display.
    pub fn get_display_rows(&self, cols: usize) -> Option<Vec<RowSnapshot>> {
        if self.offset == 0 {
            return None;
        }

        let mut view = Vec::with_capacity(self.view_rows);
        let buffer_len = self.buffer.len();
        let start = buffer_len.saturating_sub(self.view_rows + self.offset);

        for idx in start..start + self.view_rows {
            if let Some(row) = self.buffer.get(idx) {
                view.push(row.clone());
            } else {
                view.push(RowSnapshot::blank(cols));
            }
        }

        Some(view)
    }

    /// Returns the number of rows in the buffer.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Returns true if the buffer is empty.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_row(cols: usize, ch: char) -> RowSnapshot {
        RowSnapshot {
            cells: vec![(ch, Color::Default, Color::Default); cols],
            tabs: vec![None; cols],
        }
    }

    #[test]
    fn test_scrollback_buffer_new() {
        let buffer = ScrollbackBuffer::new();
        assert!(buffer.is_empty());
        assert_eq!(buffer.offset(), 0);
    }

    #[test]
    fn test_push_row() {
        let mut buffer = ScrollbackBuffer::with_capacity(10);
        buffer.push_row(make_row(80, 'A'));
        assert_eq!(buffer.len(), 1);
    }

    #[test]
    fn test_capacity_limit() {
        let mut buffer = ScrollbackBuffer::with_capacity(3);
        buffer.push_row(make_row(80, 'A'));
        buffer.push_row(make_row(80, 'B'));
        buffer.push_row(make_row(80, 'C'));
        buffer.push_row(make_row(80, 'D'));
        assert_eq!(buffer.len(), 3);
    }

    #[test]
    fn test_scroll_by() {
        let mut buffer = ScrollbackBuffer::with_capacity(100);
        buffer.set_dimensions(5, 80);

        // Add more rows than view_rows
        for i in 0..20 {
            buffer.push_row(make_row(80, (b'A' + i) as char));
        }

        assert_eq!(buffer.offset(), 0);

        buffer.scroll_by(3);
        assert_eq!(buffer.offset(), 3);

        buffer.scroll_by(-1);
        assert_eq!(buffer.offset(), 2);

        // Should clamp to 0
        buffer.scroll_by(-100);
        assert_eq!(buffer.offset(), 0);
    }

    #[test]
    fn test_get_display_rows_live() {
        let mut buffer = ScrollbackBuffer::new();
        buffer.set_dimensions(5, 80);

        // With offset 0, should return None (use live data)
        assert!(buffer.get_display_rows(80).is_none());
    }

    #[test]
    fn test_get_display_rows_scrolled() {
        let mut buffer = ScrollbackBuffer::with_capacity(100);
        buffer.set_dimensions(5, 80);

        for i in 0..20 {
            buffer.push_row(make_row(80, (b'A' + i) as char));
        }

        buffer.scroll_by(5);
        let rows = buffer.get_display_rows(80);
        assert!(rows.is_some());
        assert_eq!(rows.unwrap().len(), 5);
    }

    #[test]
    fn test_clear() {
        let mut buffer = ScrollbackBuffer::new();
        buffer.push_row(make_row(80, 'A'));
        buffer.scroll_by(1);

        buffer.clear();
        assert!(buffer.is_empty());
        assert_eq!(buffer.offset(), 0);
    }
}
