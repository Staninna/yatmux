//! Mouse selection handling for the terminal.
//!
//! This module provides text selection functionality including
//! position tracking, normalization, and hit testing.

/// A position within the terminal grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellPos {
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
#[derive(Default)]
pub struct SelectionManager {
    selection: Option<Selection>,
    view_rows: usize,
    view_cols: usize,
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

    /// Clamps a position to be within valid bounds.
    fn clamp_position(&self, row: usize, col: usize) -> CellPos {
        CellPos {
            row: row.min(self.view_rows.saturating_sub(1)),
            col: col.min(self.view_cols.saturating_sub(1)),
        }
    }

    /// Starts a new selection at the given position.
    pub fn start(&mut self, row: usize, col: usize) {
        let pos = self.clamp_position(row, col);
        self.selection = Some(Selection::new(pos));
    }

    /// Updates the current selection's end position.
    pub fn update(&mut self, row: usize, col: usize) {
        let pos = self.clamp_position(row, col);
        if let Some(ref mut sel) = self.selection {
            sel.update_end(pos);
        }
    }

    /// Clears the current selection.
    pub fn clear(&mut self) {
        self.selection = None;
    }

    /// Checks if a cell is currently selected.
    pub fn is_selected(&self, row: usize, col: usize) -> bool {
        self.selection
            .map(|sel| sel.contains(row, col))
            .unwrap_or(false)
    }

    /// Returns the current selection if any.
    #[allow(dead_code)]
    pub fn current(&self) -> Option<Selection> {
        self.selection
    }

    /// Returns the selection bounds as ((start_row, start_col), (end_row, end_col)).
    /// Returns None if there is no selection.
    pub fn bounds(&self) -> Option<((usize, usize), (usize, usize))> {
        self.selection.map(|sel| {
            let (start, end) = sel.normalized();
            ((start.row, start.col), (end.row, end.col))
        })
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

        mgr.start(100, 100);
        assert!(mgr.is_selected(23, 79));
    }
}
