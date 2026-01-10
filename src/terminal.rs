//! Terminal state management.
//!
//! This module provides the core terminal state, separating terminal logic
//! from rendering concerns.

use std::sync::{Arc, Mutex};

use portable_pty::PtySize;
use vt100::Color;

use crate::constants::{DEFAULT_COLS, DEFAULT_ROWS, TAB_STOP_WIDTH};
use crate::pty::Pty;

/// Data for a single cell: character, foreground color, background color.
#[allow(dead_code)]
pub type CellData = (char, Color, Color);

/// A snapshot of a single terminal row.
#[derive(Clone)]
#[allow(dead_code)]
pub struct RowSnapshot {
    pub cells: Vec<CellData>,
    pub tabs: Vec<Option<(usize, usize)>>,
}

impl RowSnapshot {
    /// Creates a blank row with the specified number of columns.
    #[allow(dead_code)]
    pub fn blank(cols: usize) -> Self {
        RowSnapshot {
            cells: vec![(' ', Color::Default, Color::Default); cols],
            tabs: vec![None; cols],
        }
    }
}

/// A complete snapshot of the terminal screen state.
#[allow(dead_code)]
pub struct ScreenSnapshot {
    pub rows: Vec<RowSnapshot>,
    pub cursor: (u16, u16),
    pub width: usize,
    pub height: usize,
}

/// Core terminal state, independent of rendering.
pub struct Terminal {
    parser: Arc<Mutex<vt100::Parser>>,
    pty: Arc<Pty>,
}

impl Terminal {
    /// Creates a new terminal with the given PTY.
    pub fn new(pty: Arc<Pty>) -> Self {
        let parser = Arc::new(Mutex::new(vt100::Parser::new(
            DEFAULT_ROWS,
            DEFAULT_COLS,
            0,
        )));
        Terminal { parser, pty }
    }

    /// Returns a clone of the parser Arc for use in the PTY reader thread.
    pub fn parser(&self) -> Arc<Mutex<vt100::Parser>> {
        Arc::clone(&self.parser)
    }

    /// Returns a clone of the PTY Arc.
    #[allow(dead_code)]
    pub fn pty(&self) -> Arc<Pty> {
        Arc::clone(&self.pty)
    }

    /// Writes bytes to the terminal PTY.
    pub fn write(&self, bytes: &[u8]) {
        self.pty.write(bytes);
    }

    /// Resizes the terminal to fit the given pixel dimensions.
    pub fn resize(&self, width: u32, height: u32, cell_w: usize, cell_h: usize) {
        let cols = (width as usize / cell_w).max(1) as u16;
        let rows = (height as usize / cell_h).max(1) as u16;

        if let Ok(mut parser) = self.parser.lock() {
            parser.set_size(rows, cols);
        }

        self.pty.resize(PtySize {
            rows,
            cols,
            pixel_width: width as u16,
            pixel_height: height as u16,
        });
    }

    /// Captures the current screen state as a snapshot.
    #[allow(dead_code)]
    pub fn capture_screen(&self, view_rows: usize, view_cols: usize) -> Option<ScreenSnapshot> {
        let parser = self.parser.lock().ok()?;
        let screen = parser.screen();
        let cursor = screen.cursor_position();

        let mut rows = Vec::with_capacity(view_rows);
        for row in 0..view_rows {
            let row_data = self.capture_row(&screen, row, view_cols);
            rows.push(row_data);
        }

        Some(ScreenSnapshot {
            rows,
            cursor,
            width: view_cols,
            height: view_rows,
        })
    }

    /// Captures a single row from the screen.
    #[allow(dead_code)]
    fn capture_row(&self, screen: &vt100::Screen, row: usize, cols: usize) -> RowSnapshot {
        let mut cells = Vec::with_capacity(cols);
        let mut tabs = vec![None; cols];

        for col in 0..cols {
            let cell = screen.cell(row as u16, col as u16);
            let contents = cell.map(|c| c.contents()).unwrap_or_default();
            let ch = contents.chars().next().unwrap_or(' ');
            let fg = cell.map(|c| c.fgcolor()).unwrap_or(Color::Default);
            let bg = cell.map(|c| c.bgcolor()).unwrap_or(Color::Default);

            if ch == '\t' {
                let end_col = ((col / TAB_STOP_WIDTH) + 1) * TAB_STOP_WIDTH;
                let end_col = end_col.min(cols);
                for c in col..end_col {
                    tabs[c] = Some((col, end_col));
                }
            }

            cells.push((ch, fg, bg));
        }

        RowSnapshot { cells, tabs }
    }

    /// Gets the selected text from the terminal screen.
    pub fn get_selected_text(
        &self,
        selection: Option<((usize, usize), (usize, usize))>,
    ) -> Option<String> {
        let ((start_row, start_col), (end_row, end_col)) = selection?;
        let parser = self.parser.lock().ok()?;
        let screen = parser.screen();

        let mut text = String::new();

        for row in start_row..=end_row {
            let row_start = if row == start_row { start_col } else { 0 };
            let row_end = if row == end_row {
                end_col + 1
            } else {
                screen.size().1 as usize
            };

            for col in row_start..row_end {
                if let Some(cell) = screen.cell(row as u16, col as u16) {
                    let contents = cell.contents();
                    if !contents.is_empty() {
                        text.push_str(&contents);
                    } else {
                        text.push(' ');
                    }
                }
            }

            if row != end_row {
                text.push('\n');
            }
        }

        // Trim trailing whitespace from each line but keep newlines
        let trimmed: String = text
            .lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n");

        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }
}
