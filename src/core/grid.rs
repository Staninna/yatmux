//! Shared grid snapshot types used across the project.

use crate::core::color::Color;

/// Cell data: character, foreground color, background color.
pub type CellData = (char, Color, Color);

/// A snapshot of a single terminal row.
#[derive(Clone, Debug, PartialEq)]
pub struct RowSnapshot {
    pub cells: Vec<CellData>,
    pub tabs: Vec<Option<(usize, usize)>>,
    /// OSC 8 hyperlinks per cell (None if no hyperlink, Some(url) otherwise).
    pub hyperlinks: Vec<Option<String>>,
}

impl RowSnapshot {
    /// Creates a blank row with the specified number of columns.
    pub fn blank(cols: usize) -> Self {
        RowSnapshot {
            cells: vec![(' ', Color::Default, Color::Default); cols],
            tabs: vec![None; cols],
            hyperlinks: vec![None; cols],
        }
    }

    /// Creates a new row from cell data and tab information.
    pub fn new(cells: Vec<CellData>, tabs: Vec<Option<(usize, usize)>>) -> Self {
        let len = cells.len();
        RowSnapshot {
            cells,
            tabs,
            hyperlinks: vec![None; len],
        }
    }

    /// Creates a new row from cell data, tab information, and hyperlinks.
    pub fn with_hyperlinks(
        cells: Vec<CellData>,
        tabs: Vec<Option<(usize, usize)>>,
        hyperlinks: Vec<Option<String>>,
    ) -> Self {
        RowSnapshot {
            cells,
            tabs,
            hyperlinks,
        }
    }

    /// Returns just the text content of the row.
    pub fn text(&self) -> String {
        self.cells.iter().map(|(ch, _, _)| ch).collect()
    }
}
