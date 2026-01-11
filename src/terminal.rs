//!
//! Terminal state management built on a terminal-core dependency.
//!
//! We use `tattoy-wezterm-term` as the terminal model because it supports
//! robust resize behavior (rewrapping logical lines instead of truncating
//! and losing data when the viewport shrinks).

use std::io::{self, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use vt100::Color;

use crate::constants::{DEFAULT_COLS, DEFAULT_ROWS, SCROLLBACK_CAPACITY};
use crate::core::grid::RowSnapshot;
use crate::pty::PtyWriter;

use tattoy_wezterm_cell::color::ColorAttribute;
use tattoy_wezterm_term::color::ColorPalette;
use tattoy_wezterm_term::{Terminal as WezTerminal, TerminalConfiguration, TerminalSize};

#[derive(Debug)]
struct TermConfig {
    scrollback: usize,
}

impl TerminalConfiguration for TermConfig {
    fn scrollback_size(&self) -> usize {
        self.scrollback
    }

    fn color_palette(&self) -> ColorPalette {
        ColorPalette::default()
    }
}

#[derive(Clone)]
struct PtyWriteAdapter {
    pty: Arc<dyn PtyWriter>,
}

impl Write for PtyWriteAdapter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.pty.write(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn color_attr_to_vt100(color: ColorAttribute) -> Color {
    match color {
        ColorAttribute::Default => Color::Default,
        ColorAttribute::PaletteIndex(idx) => Color::Idx(idx),
        ColorAttribute::TrueColorWithDefaultFallback(srgba) => {
            let (r, g, b, _) = srgba.as_rgba_u8();
            Color::Rgb(r, g, b)
        }
        ColorAttribute::TrueColorWithPaletteFallback(srgba, _fallback) => {
            let (r, g, b, _) = srgba.as_rgba_u8();
            Color::Rgb(r, g, b)
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
    pub cursor_visible: bool,
}

/// Core terminal state, independent of rendering.
///
/// Internally uses a robust terminal model that reflows on resize.
pub struct Terminal {
    term: Mutex<WezTerminal>,
    pty: Arc<dyn PtyWriter>,
    size: Mutex<(u16, u16)>,
    generation: AtomicU64,
}

impl Terminal {
    /// Creates a new terminal with the given PTY.
    pub fn new(pty: Arc<dyn PtyWriter>) -> Self {
        Self::new_with_scrollback(pty, SCROLLBACK_CAPACITY)
    }

    pub fn new_with_scrollback(pty: Arc<dyn PtyWriter>, scrollback_lines: usize) -> Self {
        let config: Arc<dyn TerminalConfiguration + Send + Sync> = Arc::new(TermConfig {
            scrollback: scrollback_lines,
        });

        let size = TerminalSize {
            rows: DEFAULT_ROWS as usize,
            cols: DEFAULT_COLS as usize,
            pixel_width: 0,
            pixel_height: 0,
            dpi: 0,
        };

        let writer: Box<dyn Write + Send> = Box::new(PtyWriteAdapter { pty: pty.clone() });

        let term = WezTerminal::new(size, config, "term", "0.1.0", writer);

        Terminal {
            term: Mutex::new(term),
            pty,
            size: Mutex::new((DEFAULT_ROWS, DEFAULT_COLS)),
            generation: AtomicU64::new(1),
        }
    }

    fn bump_generation(&self) {
        self.generation.fetch_add(1, Ordering::Relaxed);
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Relaxed)
    }

    /// Writes bytes to the terminal PTY.
    pub fn write(&self, bytes: &[u8]) {
        self.pty.write(bytes);
    }

    /// Resizes the terminal to fit the given pixel dimensions.
    pub fn resize(&self, width: u32, height: u32, cell_w: usize, cell_h: usize) {
        let cols = (width as usize / cell_w).max(1) as u16;
        let rows = (height as usize / cell_h).max(1) as u16;

        {
            let mut size_guard = self.size.lock().unwrap();
            *size_guard = (rows, cols);
        }

        if let Ok(mut term) = self.term.lock() {
            term.resize(TerminalSize {
                rows: rows as usize,
                cols: cols as usize,
                pixel_width: width as usize,
                pixel_height: height as usize,
                dpi: 0,
            });
        }

        self.pty.resize(rows, cols, width as u16, height as u16);
        self.bump_generation();
    }

    /// Processes input bytes through the terminal model (simulates PTY output).
    pub fn process(&self, bytes: &[u8]) {
        if let Ok(mut term) = self.term.lock() {
            term.advance_bytes(bytes);
        }
        self.bump_generation();
    }

    /// Clears scrollback history (keeps viewport content).
    pub fn clear_scrollback(&self) {
        if let Ok(mut term) = self.term.lock() {
            term.erase_scrollback();
        }
        self.bump_generation();
    }

    /// Captures the current screen state as a snapshot.
    pub fn capture_screen(&self) -> Option<ScreenSnapshot> {
        let (rows, cols) = *self.size.lock().ok()?;
        let rows_usize = rows as usize;
        let cols_usize = cols as usize;

        let term = self.term.lock().ok()?;
        let screen = term.screen();
        let cursor = term.cursor_pos();

        let mut all_rows: Vec<RowSnapshot> = Vec::with_capacity(screen.scrollback_rows());
        screen.for_each_phys_line(|_idx, line| {
            let mut cells = Vec::with_capacity(cols_usize);

            for col in 0..cols_usize {
                if let Some(cell) = line.get_cell(col) {
                    let grapheme = cell.str();
                    let ch = grapheme.chars().next().unwrap_or(' ');
                    let attrs = cell.attrs();
                    let fg = color_attr_to_vt100(attrs.foreground());
                    let bg = color_attr_to_vt100(attrs.background());
                    cells.push((ch, fg, bg));
                } else {
                    cells.push((' ', Color::Default, Color::Default));
                }
            }

            let tabs = vec![None; cols_usize];
            all_rows.push(RowSnapshot::new(cells, tabs));
        });

        let cursor_visible = matches!(
            cursor.visibility,
            tattoy_wezterm_surface::CursorVisibility::Visible
        );

        // Ensure at least `rows` rows in the snapshot (pad at top).
        let rows_data = if all_rows.len() >= rows_usize {
            all_rows
        } else {
            let mut padded = Vec::with_capacity(rows_usize);
            for _ in 0..(rows_usize - all_rows.len()) {
                padded.push(RowSnapshot::blank(cols_usize));
            }
            padded.extend(all_rows);
            padded
        };

        let cursor_tuple = (cursor.y.max(0) as u16, cursor.x as u16);

        Some(ScreenSnapshot {
            rows: rows_data,
            cursor: cursor_tuple,
            width: cols_usize,
            height: rows_usize,
            cursor_visible,
        })
    }

    pub fn buffer_len(&self) -> usize {
        let term = self.term.lock().unwrap();
        term.screen().scrollback_rows()
    }

    pub fn cursor(&self) -> ((u16, u16), bool) {
        let term = self.term.lock().unwrap();
        let cursor = term.cursor_pos();
        let cursor_visible = matches!(
            cursor.visibility,
            tattoy_wezterm_surface::CursorVisibility::Visible
        );
        ((cursor.y.max(0) as u16, cursor.x as u16), cursor_visible)
    }

    /// Returns a window of rows from the scrollback+viewport buffer.
    ///
    /// `start` is an absolute row index (0 = oldest row).
    pub fn rows_in_range(&self, start: usize, count: usize, cols: usize) -> Vec<RowSnapshot> {
        if count == 0 {
            return Vec::new();
        }

        let term = self.term.lock().unwrap();
        let screen = term.screen();
        let buffer_len = screen.scrollback_rows();

        if start >= buffer_len {
            return Vec::new();
        }

        let end = (start + count).min(buffer_len);
        let mut out: Vec<RowSnapshot> = Vec::with_capacity(end - start);

        screen.with_phys_lines(start..end, |lines| {
            for line in lines {
                let mut cells = Vec::with_capacity(cols);

                for col in 0..cols {
                    if let Some(cell) = line.get_cell(col) {
                        let grapheme = cell.str();
                        let ch = grapheme.chars().next().unwrap_or(' ');
                        let attrs = cell.attrs();
                        let fg = color_attr_to_vt100(attrs.foreground());
                        let bg = color_attr_to_vt100(attrs.background());
                        cells.push((ch, fg, bg));
                    } else {
                        cells.push((' ', Color::Default, Color::Default));
                    }
                }

                let tabs = vec![None; cols];
                out.push(RowSnapshot::new(cells, tabs));
            }
        });

        out
    }

    /// Returns all rows (scrollback + viewport) as snapshots.
    ///
    /// This is relatively expensive; prefer `rows_in_range` for rendering.
    pub fn all_rows(&self, cols: usize) -> (Vec<RowSnapshot>, (u16, u16), bool) {
        let buffer_len = self.buffer_len();
        let rows = self.rows_in_range(0, buffer_len, cols);
        let (cursor, cursor_visible) = self.cursor();
        (rows, cursor, cursor_visible)
    }

    /// Returns the current screen contents as a string.
    ///
    /// Useful for testing and debugging.
    pub fn screen_text(&self) -> String {
        let (rows, cols) = *self.size.lock().unwrap();
        let (all_rows, _cursor, _cursor_visible) = self.all_rows(cols as usize);

        // Only show the viewport (last N rows)
        let rows_usize = rows as usize;
        let start = all_rows.len().saturating_sub(rows_usize);
        let mut text = String::new();

        for row in all_rows.iter().skip(start) {
            for col in 0..(cols as usize) {
                let ch = row.cells.get(col).map(|(c, _, _)| *c).unwrap_or(' ');
                text.push(ch);
            }
            text.push('\n');
        }

        text
    }

    /// Gets the selected text from the terminal viewport.
    pub fn get_selected_text(
        &self,
        selection: Option<((usize, usize), (usize, usize))>,
    ) -> Option<String> {
        let ((start_row, start_col), (end_row, end_col)) = selection?;

        let (rows_u16, cols_u16) = *self.size.lock().ok()?;
        let rows = rows_u16 as usize;
        let cols = cols_u16 as usize;

        let (all_rows, _cursor, _cursor_visible) = self.all_rows(cols);
        let viewport_start = all_rows.len().saturating_sub(rows);
        let viewport_rows = &all_rows[viewport_start..];

        let mut text = String::new();

        for row in start_row..=end_row {
            let row_start = if row == start_row { start_col } else { 0 };
            let row_end = if row == end_row { end_col + 1 } else { cols };

            if let Some(row_data) = viewport_rows.get(row) {
                for col in row_start..row_end {
                    let ch = row_data.cells.get(col).map(|(c, _, _)| *c).unwrap_or(' ');
                    text.push(ch);
                }
            }

            if row != end_row {
                text.push('\n');
            }
        }

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
        let screen = terminal.screen_text();
        assert!(!screen.is_empty());
    }

    #[test]
    fn test_terminal_write_forwards_to_pty() {
        let (terminal, mock_pty) = create_test_terminal();

        terminal.write(b"hello");
        terminal.write(b" world");

        assert_eq!(mock_pty.written_string(), "hello world");
    }

    #[test]
    fn test_terminal_resize_updates_pty() {
        let (terminal, mock_pty) = create_test_terminal();
        terminal.resize(800, 600, 10, 20);

        let resizes = mock_pty.resizes.lock().unwrap();
        assert_eq!(resizes.len(), 1);
        assert_eq!(resizes[0], (30, 80, 800, 600));
    }

    #[test]
    fn test_terminal_handles_output_and_resize_reflow() {
        let (terminal, _mock_pty) = create_test_terminal();
        terminal.process(b"hello world this is a long line");

        // Shrink a lot, then grow again; content should still be present.
        terminal.resize(80, 200, 10, 20); // 8 cols
        terminal.resize(800, 200, 10, 20); // 80 cols

        let screen = terminal.screen_text();
        assert!(screen.contains("hello"));
        assert!(screen.contains("world"));
    }
}
