//! Shared test utilities for terminal tests.

use yatmux::core::color::Color;
use yatmux::core::grid::RowSnapshot;

/// Creates a row snapshot from a text string.
#[allow(dead_code)]
pub fn make_row(text: &str) -> RowSnapshot {
    let cells: Vec<_> = text
        .chars()
        .map(|ch| (ch, Color::Default, Color::Default))
        .collect();
    let tabs = vec![None; cells.len()];
    RowSnapshot::new(cells, tabs)
}

/// Creates multiple row snapshots from a slice of text strings.
#[allow(dead_code)]
pub fn make_rows(texts: &[&str]) -> Vec<RowSnapshot> {
    texts.iter().map(|t| make_row(t)).collect()
}
