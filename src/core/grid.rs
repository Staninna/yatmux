//! Shared grid snapshot types used across the project.

use vt100::Color;

/// Cell data: character, foreground color, background color.
pub type CellData = (char, Color, Color);

/// A snapshot of a single terminal row.
#[derive(Clone, PartialEq)]
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

    /// Returns just the text content of the row.
    pub fn text(&self) -> String {
        self.cells.iter().map(|(ch, _, _)| ch).collect()
    }
}
