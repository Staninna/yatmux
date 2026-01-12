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
