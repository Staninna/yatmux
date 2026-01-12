//! Mouse selection handling for the terminal.
//!
//! This module provides text selection functionality including
//! position tracking, normalization, and hit testing.
//!
//! Selections are stored in absolute scrollback coordinates so they
//! scroll with the content rather than staying fixed to screen positions.

/// A position within the terminal grid (absolute scrollback coordinates).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellPos {
    /// Absolute row in the scrollback buffer (0 = oldest row in buffer).
    pub row: usize,
    pub col: usize,
}

impl CellPos {
    #[allow(dead_code)]
    pub fn new(row: usize, col: usize) -> Self {
        CellPos { row, col }
    }
}

/// A text selection defined by start and end positions.
#[derive(Clone, Copy, Debug)]
pub struct Selection {
    start: CellPos,
    end: CellPos,
}

impl Selection {
    /// Creates a new selection at the given position.
    pub fn new(pos: CellPos) -> Self {
        Selection {
            start: pos,
            end: pos,
        }
    }

    /// Updates the end position of the selection.
    pub fn update_end(&mut self, pos: CellPos) {
        self.end = pos;
    }

    /// Returns the selection bounds in normalized order (start <= end).
    pub fn normalized(&self) -> (CellPos, CellPos) {
        if (self.start.row, self.start.col) <= (self.end.row, self.end.col) {
            (self.start, self.end)
        } else {
            (self.end, self.start)
        }
    }

    /// Checks if a cell position is within the selection.
    pub fn contains(&self, row: usize, col: usize) -> bool {
        let (start, end) = self.normalized();

        if row < start.row || row > end.row {
            return false;
        }

        // Single line selection
        if start.row == end.row {
            return col >= start.col && col <= end.col;
        }

        // Multi-line selection
        if row == start.row {
            return col >= start.col;
        }
        if row == end.row {
            return col <= end.col;
        }

        // Middle rows are fully selected
        true
    }

    /// Returns the start position.
    #[allow(dead_code)]
    pub fn start(&self) -> CellPos {
        self.start
    }

    /// Returns the end position.
    #[allow(dead_code)]
    pub fn end(&self) -> CellPos {
        self.end
    }
}

/// Manages the current selection state.
///
/// Selections are stored in absolute scrollback buffer coordinates.
/// When checking if a screen cell is selected, the scroll offset is used
/// to convert between screen and absolute coordinates.
#[derive(Default)]
pub struct SelectionManager {
    selection: Option<Selection>,
    view_rows: usize,
    view_cols: usize,
    /// Current scroll offset (0 = live view, >0 = scrolled back).
    scroll_offset: usize,
    /// Total rows in the scrollback buffer.
    buffer_len: usize,
}

impl SelectionManager {
    pub fn new() -> Self {
        SelectionManager::default()
    }

    /// Updates the view dimensions. Clears selection if dimensions change.
    pub fn set_dimensions(&mut self, rows: usize, cols: usize) {
        if self.view_rows != rows || self.view_cols != cols {
            self.view_rows = rows;
            self.view_cols = cols;
            self.selection = None;
        }
    }

    /// Updates the scroll state (offset and buffer length).
    pub fn set_scroll_state(&mut self, offset: usize, buffer_len: usize) {
        self.scroll_offset = offset;
        self.buffer_len = buffer_len;
    }

    /// Converts a screen row to an absolute buffer row.
    fn screen_to_absolute(&self, screen_row: usize) -> usize {
        // The visible window starts at: buffer_len - view_rows - scroll_offset
        let window_start = self
            .buffer_len
            .saturating_sub(self.view_rows + self.scroll_offset);
        window_start + screen_row
    }

    /// Converts an absolute buffer row to a screen row.
    /// Returns None if the row is not currently visible.
    #[allow(dead_code)]
    fn absolute_to_screen(&self, abs_row: usize) -> Option<usize> {
        let window_start = self
            .buffer_len
            .saturating_sub(self.view_rows + self.scroll_offset);
        let window_end = window_start + self.view_rows;

        if abs_row >= window_start && abs_row < window_end {
            Some(abs_row - window_start)
        } else {
            None
        }
    }

    /// Clamps a screen position to be within valid bounds.
    fn clamp_screen_position(&self, row: usize, col: usize) -> (usize, usize) {
        (
            row.min(self.view_rows.saturating_sub(1)),
            col.min(self.view_cols.saturating_sub(1)),
        )
    }

    /// Starts a new selection at the given screen position.
    pub fn start(&mut self, screen_row: usize, col: usize) {
        let (screen_row, col) = self.clamp_screen_position(screen_row, col);
        let abs_row = self.screen_to_absolute(screen_row);
        let pos = CellPos { row: abs_row, col };
        self.selection = Some(Selection::new(pos));
    }

    /// Updates the current selection's end position (screen coordinates).
    pub fn update(&mut self, screen_row: usize, col: usize) {
        let (screen_row, col) = self.clamp_screen_position(screen_row, col);
        let abs_row = self.screen_to_absolute(screen_row);
        let pos = CellPos { row: abs_row, col };
        if let Some(ref mut sel) = self.selection {
            sel.update_end(pos);
        }
    }

    /// Clears the current selection.
    pub fn clear(&mut self) {
        self.selection = None;
    }

    /// Returns true if there is an active selection.
    pub fn has_selection(&self) -> bool {
        self.selection.is_some()
    }

    /// Selects all text in the buffer.
    pub fn select_all(&mut self) {
        if self.buffer_len == 0 || self.view_cols == 0 {
            return;
        }
        let start = CellPos { row: 0, col: 0 };
        let end = CellPos {
            row: self.buffer_len.saturating_sub(1),
            col: self.view_cols.saturating_sub(1),
        };
        let mut sel = Selection::new(start);
        sel.update_end(end);
        self.selection = Some(sel);
    }

    /// Checks if a screen cell is currently selected.
    pub fn is_selected(&self, screen_row: usize, col: usize) -> bool {
        let Some(sel) = self.selection else {
            return false;
        };
        let abs_row = self.screen_to_absolute(screen_row);
        sel.contains(abs_row, col)
    }

    /// Returns the current selection if any.
    #[allow(dead_code)]
    pub fn current(&self) -> Option<Selection> {
        self.selection
    }

    /// Returns the selection bounds in absolute coordinates.
    /// Returns None if there is no selection.
    pub fn bounds(&self) -> Option<((usize, usize), (usize, usize))> {
        self.selection.map(|sel| {
            let (start, end) = sel.normalized();
            ((start.row, start.col), (end.row, end.col))
        })
    }

    /// Returns the selection bounds in screen coordinates if visible.
    /// Clamps to visible range.
    #[allow(dead_code)]
    pub fn visible_bounds(&self) -> Option<((usize, usize), (usize, usize))> {
        let sel = self.selection?;
        let (start, end) = sel.normalized();

        // Convert to screen coordinates, clamping to visible range
        let window_start = self
            .buffer_len
            .saturating_sub(self.view_rows + self.scroll_offset);
        let window_end = window_start + self.view_rows;

        // Check if selection overlaps with visible window
        if end.row < window_start || start.row >= window_end {
            return None; // Selection entirely outside visible window
        }

        let vis_start_row = if start.row < window_start {
            0
        } else {
            start.row - window_start
        };
        let vis_end_row = if end.row >= window_end {
            self.view_rows.saturating_sub(1)
        } else {
            end.row - window_start
        };

        // Adjust columns if row is clamped
        let vis_start_col = if start.row < window_start {
            0
        } else {
            start.col
        };
        let vis_end_col = if end.row >= window_end {
            self.view_cols.saturating_sub(1)
        } else {
            end.col
        };

        Some(((vis_start_row, vis_start_col), (vis_end_row, vis_end_col)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_pos_new() {
        let pos = CellPos::new(5, 10);
        assert_eq!(pos.row, 5);
        assert_eq!(pos.col, 10);
    }

    #[test]
    fn test_selection_new() {
        let sel = Selection::new(CellPos::new(1, 2));
        assert_eq!(sel.start(), sel.end());
    }

    #[test]
    fn test_selection_normalized() {
        let mut sel = Selection::new(CellPos::new(5, 10));
        sel.update_end(CellPos::new(2, 5));
        let (start, end) = sel.normalized();
        assert_eq!(start.row, 2);
        assert_eq!(end.row, 5);
    }

    #[test]
    fn test_selection_contains_single_line() {
        let mut sel = Selection::new(CellPos::new(3, 5));
        sel.update_end(CellPos::new(3, 10));

        assert!(sel.contains(3, 5));
        assert!(sel.contains(3, 7));
        assert!(sel.contains(3, 10));
        assert!(!sel.contains(3, 4));
        assert!(!sel.contains(3, 11));
        assert!(!sel.contains(2, 7));
    }

    #[test]
    fn test_selection_contains_multi_line() {
        let mut sel = Selection::new(CellPos::new(2, 5));
        sel.update_end(CellPos::new(5, 10));

        // First line
        assert!(sel.contains(2, 5));
        assert!(sel.contains(2, 80));
        assert!(!sel.contains(2, 4));

        // Middle line - fully selected
        assert!(sel.contains(3, 0));
        assert!(sel.contains(4, 50));

        // Last line
        assert!(sel.contains(5, 10));
        assert!(sel.contains(5, 0));
        assert!(!sel.contains(5, 11));

        // Outside
        assert!(!sel.contains(1, 5));
        assert!(!sel.contains(6, 5));
    }

    #[test]
    fn test_selection_manager() {
        let mut mgr = SelectionManager::new();
        mgr.set_dimensions(24, 80);
        // Simulate a buffer with 24 rows at live view (offset 0)
        mgr.set_scroll_state(0, 24);

        assert!(!mgr.is_selected(5, 5));

        mgr.start(5, 5);
        assert!(mgr.is_selected(5, 5));

        mgr.update(5, 10);
        assert!(mgr.is_selected(5, 7));

        mgr.clear();
        assert!(!mgr.is_selected(5, 7));
    }

    #[test]
    fn test_selection_manager_clamps() {
        let mut mgr = SelectionManager::new();
        mgr.set_dimensions(24, 80);
        mgr.set_scroll_state(0, 24);

        mgr.start(100, 100);
        assert!(mgr.is_selected(23, 79));
    }

    #[test]
    fn test_selection_scrolls_with_content() {
        let mut mgr = SelectionManager::new();
        mgr.set_dimensions(24, 80);
        // Buffer has 48 rows, viewing last 24 (live view)
        mgr.set_scroll_state(0, 48);

        // Select row 10 on screen (absolute row 34)
        mgr.start(10, 5);
        mgr.update(10, 15);

        assert!(mgr.is_selected(10, 10));
        assert!(!mgr.is_selected(9, 10));

        // Scroll up by 5 lines - selection should now be at screen row 15
        mgr.set_scroll_state(5, 48);

        assert!(!mgr.is_selected(10, 10)); // No longer at screen row 10
        assert!(mgr.is_selected(15, 10)); // Now at screen row 15
    }
}
