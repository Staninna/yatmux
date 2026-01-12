//!
//! Terminal state management built on a terminal-core dependency.
//!
//! We use `tattoy-wezterm-term` as the terminal model because it supports
//! robust resize behavior (rewrapping logical lines instead of truncating
//! and losing data when the viewport shrinks).

use std::io::{self, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow};
use vt100::Color;

use crate::constants::{DEFAULT_COLS, DEFAULT_ROWS, SCROLLBACK_CAPACITY};
use crate::core::grid::RowSnapshot;
use crate::pty::PtyWriter;

use tattoy_wezterm_cell::color::ColorAttribute;
use tattoy_wezterm_term::color::ColorPalette;
use tattoy_wezterm_term::{
    Alert, AlertHandler, Terminal as WezTerminal, TerminalConfiguration, TerminalSize,
};

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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ShellIntegrationStatus {
    pub osc7_cwd: bool,
    pub osc133_semantic: bool,
    pub osc_title: bool,
}

impl ShellIntegrationStatus {
    pub fn any(&self) -> bool {
        self.osc7_cwd || self.osc133_semantic || self.osc_title
    }
}

/// Information about the current prompt for sticky prompt display.
#[derive(Debug, Clone)]
pub struct StickyPromptInfo {
    pub rows: Vec<RowSnapshot>,
    /// Cursor position relative to the prompt rows (row, col).
    pub cursor: Option<(usize, usize)>,
}

#[derive(Debug, Default)]
struct ShellIntegrationState {
    osc7_cwd: bool,
    osc133_semantic: bool,
    osc_title: bool,
}

#[derive(Clone)]
struct ShellIntegrationAlertHandler {
    state: Arc<Mutex<ShellIntegrationState>>,
}

impl AlertHandler for ShellIntegrationAlertHandler {
    fn alert(&mut self, alert: Alert) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };

        match alert {
            Alert::CurrentWorkingDirectoryChanged => {
                state.osc7_cwd = true;
            }
            Alert::WindowTitleChanged(_)
            | Alert::TabTitleChanged(_)
            | Alert::IconTitleChanged(_) => {
                state.osc_title = true;
            }
            _ => {}
        }
    }
}

/// Core terminal state, independent of rendering.
///
/// Internally uses a robust terminal model that reflows on resize.
pub struct Terminal {
    term: Mutex<WezTerminal>,
    pty: Arc<dyn PtyWriter>,
    size: Mutex<(u16, u16)>,
    generation: AtomicU64,
    shell_integration: Arc<Mutex<ShellIntegrationState>>,
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

        let shell_integration = Arc::new(Mutex::new(ShellIntegrationState::default()));

        let mut term = WezTerminal::new(size, config, "yatmux", env!("CARGO_PKG_VERSION"), writer);
        term.set_notification_handler(Box::new(ShellIntegrationAlertHandler {
            state: shell_integration.clone(),
        }));

        Terminal {
            term: Mutex::new(term),
            pty,
            size: Mutex::new((DEFAULT_ROWS, DEFAULT_COLS)),
            generation: AtomicU64::new(1),
            shell_integration,
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

    pub fn shell_integration_status(&self) -> ShellIntegrationStatus {
        let Ok(state) = self.shell_integration.lock() else {
            return ShellIntegrationStatus::default();
        };

        ShellIntegrationStatus {
            osc7_cwd: state.osc7_cwd,
            osc133_semantic: state.osc133_semantic,
            osc_title: state.osc_title,
        }
    }

    /// Returns the current shell-reported title, if any.
    pub fn shell_title(&self) -> Option<String> {
        // Avoid false positives from the terminal model's default title.
        if !self.shell_integration_status().osc_title {
            return None;
        }

        let term = self.term.lock().ok()?;
        let title = term.get_title().trim();
        if title.is_empty() {
            None
        } else {
            Some(title.to_string())
        }
    }

    /// Returns the current shell-reported working directory (OSC 7), if any.
    pub fn shell_cwd(&self) -> Option<String> {
        if !self.shell_integration_status().osc7_cwd {
            return None;
        }

        let term = self.term.lock().ok()?;
        term.get_current_dir().map(|u| u.to_string())
    }

    /// Computes semantic zones from OSC 133 markers (prompt/input/output).
    pub fn semantic_zones(&self) -> Result<Vec<tattoy_wezterm_term::SemanticZone>> {
        let mut term = self
            .term
            .lock()
            .map_err(|_| anyhow!("terminal mutex poisoned"))?;
        let zones = term.get_semantic_zones()?;

        let has_markers = zones
            .iter()
            .any(|z| z.semantic_type != tattoy_wezterm_term::SemanticType::Output);

        if has_markers {
            if let Ok(mut state) = self.shell_integration.lock() {
                state.osc133_semantic = true;
            }
        }

        Ok(zones)
    }

    /// Returns the content of the current (last) prompt and input lines, if available.
    /// This is used for sticky prompt display when scrolled up.
    /// Returns the rows that make up the prompt+input (may be multiple lines) and cursor position.
    pub fn current_prompt_rows(&self) -> Option<StickyPromptInfo> {
        let mut term = self.term.lock().ok()?;
        let zones = term.get_semantic_zones().ok()?;

        // Find the last Prompt zone - this confirms we have shell integration active
        let _prompt_zone = zones
            .iter()
            .rev()
            .find(|z| z.semantic_type == tattoy_wezterm_term::SemanticType::Prompt)?;

        let screen = term.screen();
        let cursor = term.cursor_pos();
        let (term_rows, cols) = *self.size.lock().ok()?;
        let cols_usize = cols as usize;
        let term_rows_usize = term_rows as usize;

        // cursor.y is screen-relative (0 = top of visible area)
        // scrollback_rows() returns total lines in buffer (scrollback + visible)
        // Visible lines start at index: total_lines - terminal_height
        // So physical line of cursor = (total_lines - terminal_height) + cursor.y
        let total_lines = screen.scrollback_rows();
        let visible_start = total_lines.saturating_sub(term_rows_usize);
        let cursor_phys_y = visible_start + cursor.y as usize;
        let cursor_x = cursor.x as usize;

        // Collect the line at the cursor position
        let mut result_rows = Vec::new();

        screen.for_each_phys_line(|idx, line| {
            if idx == cursor_phys_y {
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
                result_rows.push(RowSnapshot::new(cells, tabs));
            }
        });

        if result_rows.is_empty() {
            return None;
        }

        // Cursor is on row 0 (the only row we collected), at column cursor_x
        let relative_cursor = Some((0, cursor_x));

        Some(StickyPromptInfo {
            rows: result_rows,
            cursor: relative_cursor,
        })
    }

    /// Returns the text content of the last command's output (requires shell integration).
    /// This finds the most recent Output zone that comes after a Prompt/Input zone.
    pub fn last_command_output(&self) -> Option<String> {
        let mut term = self.term.lock().ok()?;
        let zones = term.get_semantic_zones().ok()?;
        let (_term_rows, cols) = *self.size.lock().ok()?;
        let cols_usize = cols as usize;

        // Find the last Output zone that follows a Prompt or Input zone
        // We look for the pattern: Prompt -> Input -> Output
        let mut last_output_zone: Option<&tattoy_wezterm_term::SemanticZone> = None;

        for zone in zones.iter().rev() {
            if zone.semantic_type == tattoy_wezterm_term::SemanticType::Output {
                // Check if there's a prompt/input before this output
                let has_prompt_before = zones.iter().any(|z| {
                    (z.semantic_type == tattoy_wezterm_term::SemanticType::Prompt
                        || z.semantic_type == tattoy_wezterm_term::SemanticType::Input)
                        && z.end_y < zone.start_y
                });
                if has_prompt_before {
                    last_output_zone = Some(zone);
                    break;
                }
            }
        }

        let output_zone = last_output_zone?;
        let start_y = output_zone.start_y as usize;
        let end_y = output_zone.end_y as usize;

        let screen = term.screen();
        let mut lines = Vec::new();

        screen.for_each_phys_line(|idx, line| {
            if idx >= start_y && idx <= end_y {
                let mut line_text = String::new();
                for col in 0..cols_usize {
                    if let Some(cell) = line.get_cell(col) {
                        let grapheme = cell.str();
                        line_text.push_str(grapheme);
                    } else {
                        line_text.push(' ');
                    }
                }
                // Trim trailing whitespace from each line
                lines.push(line_text.trim_end().to_string());
            }
        });

        if lines.is_empty() {
            return None;
        }

        // Join lines and trim trailing empty lines
        let mut text = lines.join("\n");
        while text.ends_with('\n') {
            text.pop();
        }

        Some(text)
    }

    /// Returns a list of all prompt line indices (physical row indices).
    /// Used for jumping between prompts.
    pub fn prompt_positions(&self) -> Vec<usize> {
        let mut term = match self.term.lock() {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };

        let zones = match term.get_semantic_zones() {
            Ok(z) => z,
            Err(_) => return Vec::new(),
        };

        zones
            .iter()
            .filter(|z| z.semantic_type == tattoy_wezterm_term::SemanticType::Prompt)
            .map(|z| z.start_y as usize)
            .collect()
    }

    /// Returns true if a command is currently running (not at a prompt).
    /// This is detected by checking if the last semantic zone is Output,
    /// or if the cursor is past the Input zone.
    pub fn is_command_running(&self) -> bool {
        let mut term = match self.term.lock() {
            Ok(t) => t,
            Err(_) => return false,
        };

        let zones = match term.get_semantic_zones() {
            Ok(z) => z,
            Err(_) => return false,
        };

        // If no zones, we can't determine state (shell integration not active)
        if zones.is_empty() {
            return false;
        }

        // Get the last zone
        let last_zone = match zones.last() {
            Some(z) => z,
            None => return false,
        };

        // If last zone is Output, a command is running
        if last_zone.semantic_type == tattoy_wezterm_term::SemanticType::Output {
            return true;
        }

        // If last zone is Input and we've moved past it (command submitted but no output yet)
        // Check if cursor is past the input zone
        if last_zone.semantic_type == tattoy_wezterm_term::SemanticType::Input {
            let cursor = term.cursor_pos();
            let screen = term.screen();
            let (term_rows, _) = match self.size.lock() {
                Ok(s) => *s,
                Err(_) => return false,
            };
            let total_lines = screen.scrollback_rows();
            let visible_start = total_lines.saturating_sub(term_rows as usize);
            let cursor_phys_y = visible_start + cursor.y as usize;

            // If cursor is past the end of the input zone, command was submitted
            if cursor_phys_y > last_zone.end_y as usize {
                return true;
            }
        }

        false
    }

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

    #[test]
    fn test_shell_title_from_osc() {
        let (terminal, _mock_pty) = create_test_terminal();

        terminal.process(b"\x1b]2;my title\x1b\\");
        assert_eq!(terminal.shell_title().as_deref(), Some("my title"));
    }

    #[test]
    fn test_shell_cwd_from_osc7() {
        let (terminal, _mock_pty) = create_test_terminal();

        terminal.process(b"\x1b]7;file://host/home/alice\x1b\\");
        assert_eq!(
            terminal.shell_cwd().as_deref(),
            Some("file://host/home/alice")
        );
    }

    #[test]
    fn test_semantic_zones_from_osc133() {
        use tattoy_wezterm_term::SemanticType;

        let (terminal, _mock_pty) = create_test_terminal();

        // Prompt
        terminal.process(b"\x1b]133;A\x1b\\");
        terminal.process(b"$ ");

        // Input
        terminal.process(b"\x1b]133;B\x1b\\");
        terminal.process(b"echo hi");

        // Output
        terminal.process(b"\x1b]133;C\x1b\\");
        terminal.process(b"\r\nhi\r\n");

        let zones = terminal.semantic_zones().unwrap();
        assert!(
            zones
                .iter()
                .any(|z| z.semantic_type == SemanticType::Prompt)
        );
        assert!(zones.iter().any(|z| z.semantic_type == SemanticType::Input));
    }

    #[test]
    fn test_shell_integration_status_detection() {
        let (terminal, _mock_pty) = create_test_terminal();

        let status = terminal.shell_integration_status();
        assert!(!status.any());

        terminal.process(b"\x1b]7;file://host/home/alice\x1b\\");
        assert!(terminal.shell_integration_status().osc7_cwd);

        terminal.process(b"\x1b]2;my title\x1b\\");
        assert!(terminal.shell_integration_status().osc_title);

        terminal.process(b"\x1b]133;A\x1b\\");
        terminal.process(b"$ ");
        terminal.process(b"\x1b]133;B\x1b\\");
        terminal.process(b"echo hi");

        // osc133 is only marked as detected once we compute zones.
        let _ = terminal.semantic_zones().unwrap();
        assert!(terminal.shell_integration_status().osc133_semantic);
    }
}
