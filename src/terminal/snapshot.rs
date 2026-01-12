use vt100::Color;

use crate::core::grid::RowSnapshot;

use super::Terminal;
use super::adapters::color_attr_to_vt100;

/// A complete snapshot of the terminal screen state.
#[allow(dead_code)]
pub struct ScreenSnapshot {
    pub rows: Vec<RowSnapshot>,
    pub cursor: (u16, u16),
    pub width: usize,
    pub height: usize,
    pub cursor_visible: bool,
}

impl Terminal {
    /// Returns the physical row index for the start of the visible area.
    pub fn visible_start_row(&self) -> usize {
        let term = match self.term.lock() {
            Ok(t) => t,
            Err(_) => return 0,
        };
        let (term_rows, _) = match self.size.lock() {
            Ok(s) => *s,
            Err(_) => return 0,
        };

        let screen = term.screen();
        let total_lines = screen.scrollback_rows();
        total_lines.saturating_sub(term_rows as usize)
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
                let mut hyperlinks = Vec::with_capacity(cols);

                for col in 0..cols {
                    if let Some(cell) = line.get_cell(col) {
                        let grapheme = cell.str();
                        let ch = grapheme.chars().next().unwrap_or(' ');
                        let attrs = cell.attrs();
                        let fg = color_attr_to_vt100(attrs.foreground());
                        let bg = color_attr_to_vt100(attrs.background());
                        cells.push((ch, fg, bg));

                        // Extract OSC 8 hyperlink if present
                        let link = attrs.hyperlink().map(|h| h.uri().to_string());
                        hyperlinks.push(link);
                    } else {
                        cells.push((' ', Color::Default, Color::Default));
                        hyperlinks.push(None);
                    }
                }

                let tabs = vec![None; cols];
                out.push(RowSnapshot::with_hyperlinks(cells, tabs, hyperlinks));
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
