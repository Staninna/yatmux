//! Terminal state management.
//!
//! This module provides the core terminal state, separating terminal logic
//! from rendering concerns.

use std::sync::{Arc, Mutex};

use vt100::Color;

use crate::constants::{DEFAULT_COLS, DEFAULT_ROWS, TAB_STOP_WIDTH};
use crate::pty::PtyWriter;

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
    pty: Arc<dyn PtyWriter>,
}

impl Terminal {
    /// Creates a new terminal with the given PTY.
    pub fn new(pty: Arc<dyn PtyWriter>) -> Self {
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
    pub fn pty(&self) -> Arc<dyn PtyWriter> {
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

        self.pty.resize(rows, cols, width as u16, height as u16);
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

    /// Processes input bytes through the parser (for testing).
    #[cfg(test)]
    pub fn process(&self, bytes: &[u8]) {
        if let Ok(mut parser) = self.parser.lock() {
            parser.process(bytes);
        }
    }

    /// Returns the current screen contents as a string (for testing).
    #[cfg(test)]
    pub fn screen_text(&self) -> String {
        let parser = self.parser.lock().unwrap();
        let screen = parser.screen();
        let (rows, cols) = screen.size();

        let mut text = String::new();
        for row in 0..rows {
            for col in 0..cols {
                if let Some(cell) = screen.cell(row, col) {
                    let contents = cell.contents();
                    if contents.is_empty() {
                        text.push(' ');
                    } else {
                        text.push_str(&contents);
                    }
                }
            }
            text.push('\n');
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pty::mock::MockPty;

    fn create_test_terminal() -> (Terminal, Arc<MockPty>) {
        let mock_pty = Arc::new(MockPty::new());
        let terminal = Terminal::new(mock_pty.clone());
        (terminal, mock_pty)
    }

    #[test]
    fn test_terminal_new() {
        let (terminal, _mock_pty) = create_test_terminal();

        // Terminal should be created with default size
        let parser = terminal.parser();
        let guard = parser.lock().unwrap();
        let screen = guard.screen();
        let (rows, cols) = screen.size();

        assert_eq!(rows, DEFAULT_ROWS);
        assert_eq!(cols, DEFAULT_COLS);
    }

    #[test]
    fn test_terminal_write_forwards_to_pty() {
        let (terminal, mock_pty) = create_test_terminal();

        terminal.write(b"hello");
        terminal.write(b" world");

        assert_eq!(mock_pty.written_string(), "hello world");
    }

    #[test]
    fn test_terminal_write_escape_sequences() {
        let (terminal, mock_pty) = create_test_terminal();

        // Write some escape sequences
        terminal.write(b"\x1b[H"); // Move cursor home
        terminal.write(b"\x1b[2J"); // Clear screen
        terminal.write(b"\x1b[31m"); // Red foreground

        let written = mock_pty.written_string();
        assert!(written.contains("\x1b[H"));
        assert!(written.contains("\x1b[2J"));
        assert!(written.contains("\x1b[31m"));
    }

    #[test]
    fn test_terminal_resize() {
        let (terminal, mock_pty) = create_test_terminal();

        // Resize with 10x20 pixel cells, 800x600 window
        terminal.resize(800, 600, 10, 20);

        // Should calculate 80 cols (800/10) and 30 rows (600/20)
        let resizes = mock_pty.resizes.lock().unwrap();
        assert_eq!(resizes.len(), 1);
        assert_eq!(resizes[0], (30, 80, 800, 600));

        // Parser should also be resized
        let parser = terminal.parser();
        let guard = parser.lock().unwrap();
        let screen = guard.screen();
        let (rows, cols) = screen.size();
        assert_eq!(rows, 30);
        assert_eq!(cols, 80);
    }

    #[test]
    fn test_terminal_resize_minimum_size() {
        let (terminal, mock_pty) = create_test_terminal();

        // Very small window should result in at least 1x1
        terminal.resize(5, 5, 10, 20);

        let resizes = mock_pty.resizes.lock().unwrap();
        assert_eq!(resizes[0].0, 1); // rows
        assert_eq!(resizes[0].1, 1); // cols
    }

    #[test]
    fn test_terminal_process_and_read() {
        let (terminal, _mock_pty) = create_test_terminal();

        // Simulate receiving text from PTY
        terminal.process(b"Hello, World!");

        let screen_text = terminal.screen_text();
        assert!(screen_text.contains("Hello, World!"));
    }

    #[test]
    fn test_terminal_get_selected_text_none() {
        let (terminal, _mock_pty) = create_test_terminal();

        let result = terminal.get_selected_text(None);
        assert!(result.is_none());
    }

    #[test]
    fn test_terminal_get_selected_text_single_line() {
        let (terminal, _mock_pty) = create_test_terminal();

        // Put some text on the screen
        terminal.process(b"Hello, World!");

        // Select "Hello" (row 0, cols 0-4)
        let selection = Some(((0, 0), (0, 4)));
        let result = terminal.get_selected_text(selection);

        assert_eq!(result, Some("Hello".to_string()));
    }

    #[test]
    fn test_terminal_get_selected_text_multi_line() {
        let (terminal, _mock_pty) = create_test_terminal();

        // Put text on multiple lines
        terminal.process(b"Line 1\r\nLine 2\r\nLine 3");

        // Select from "Line 1" to "Line 2"
        let selection = Some(((0, 0), (1, 5)));
        let result = terminal.get_selected_text(selection);

        assert!(result.is_some());
        let text = result.unwrap();
        assert!(text.contains("Line 1"));
        assert!(text.contains("Line 2"));
    }

    #[test]
    fn test_terminal_get_selected_text_trims_trailing_whitespace() {
        let (terminal, _mock_pty) = create_test_terminal();

        terminal.process(b"Short");

        // Select more than the text length - should get trimmed result
        let selection = Some(((0, 0), (0, 20)));
        let result = terminal.get_selected_text(selection);

        // Should be trimmed, not padded with spaces
        assert_eq!(result, Some("Short".to_string()));
    }

    #[test]
    fn test_terminal_get_selected_text_empty_selection() {
        let (terminal, _mock_pty) = create_test_terminal();

        // Empty screen, select an area
        let selection = Some(((5, 0), (5, 10)));
        let result = terminal.get_selected_text(selection);

        // Empty text should return None
        assert!(result.is_none());
    }

    #[test]
    fn test_terminal_capture_screen() {
        let (terminal, _mock_pty) = create_test_terminal();

        terminal.process(b"Test content");

        let snapshot = terminal.capture_screen(24, 80);
        assert!(snapshot.is_some());

        let snapshot = snapshot.unwrap();
        assert_eq!(snapshot.width, 80);
        assert_eq!(snapshot.height, 24);
        assert_eq!(snapshot.rows.len(), 24);

        // First row should contain "Test content"
        let first_row: String = snapshot.rows[0].cells.iter().map(|(ch, _, _)| ch).collect();
        assert!(first_row.starts_with("Test content"));
    }

    #[test]
    fn test_terminal_cursor_position() {
        let (terminal, _mock_pty) = create_test_terminal();

        // Initial cursor should be at (0, 0)
        let snapshot = terminal.capture_screen(24, 80).unwrap();
        assert_eq!(snapshot.cursor, (0, 0));

        // Move cursor with escape sequence
        terminal.process(b"\x1b[5;10H"); // Move to row 5, col 10 (1-indexed)

        let snapshot = terminal.capture_screen(24, 80).unwrap();
        assert_eq!(snapshot.cursor, (4, 9)); // 0-indexed
    }

    #[test]
    fn test_terminal_colors() {
        let (terminal, _mock_pty) = create_test_terminal();

        // Set red foreground and blue background, then write
        terminal.process(b"\x1b[31;44mColored");

        let snapshot = terminal.capture_screen(24, 80).unwrap();
        let first_cell = &snapshot.rows[0].cells[0];

        // Check that colors were captured (specific values depend on vt100 implementation)
        assert_eq!(first_cell.0, 'C');
        // fg should be red (index 1), bg should be blue (index 4)
        assert!(matches!(first_cell.1, Color::Idx(1)));
        assert!(matches!(first_cell.2, Color::Idx(4)));
    }

    #[test]
    fn test_row_snapshot_blank() {
        let row = RowSnapshot::blank(80);

        assert_eq!(row.cells.len(), 80);
        assert_eq!(row.tabs.len(), 80);

        // All cells should be spaces with default colors
        for (ch, fg, bg) in &row.cells {
            assert_eq!(*ch, ' ');
            assert!(matches!(fg, Color::Default));
            assert!(matches!(bg, Color::Default));
        }

        // All tabs should be None
        for tab in &row.tabs {
            assert!(tab.is_none());
        }
    }
}
