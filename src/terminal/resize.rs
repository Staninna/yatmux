use super::*;

use tattoy_wezterm_term::MouseEvent;
use tattoy_wezterm_term::TerminalSize;

impl Terminal {
    /// Writes bytes to the terminal PTY.
    pub fn write(&self, bytes: &[u8]) {
        self.pty.write(bytes);
    }

    /// Returns true if the terminal application wants to receive mouse events.
    pub fn is_mouse_grabbed(&self) -> bool {
        if let Ok(term) = self.term.lock() {
            term.is_mouse_grabbed()
        } else {
            false
        }
    }

    /// Returns true if an application is using the alternate screen.
    ///
    /// Full-screen TUI apps (htop, vim, less) typically activate this.
    pub fn is_alt_screen_active(&self) -> bool {
        if let Ok(term) = self.term.lock() {
            term.is_alt_screen_active()
        } else {
            false
        }
    }

    /// Sends a mouse event to the terminal.
    /// Returns true if the event was handled by the terminal application.
    pub fn mouse_event(
        &self,
        x: usize,
        y: usize,
        button: MouseButton,
        kind: MouseEventKind,
        modifiers: KeyModifiers,
    ) -> bool {
        if let Ok(mut term) = self.term.lock() {
            if !term.is_mouse_grabbed() {
                return false;
            }
            let event = MouseEvent {
                kind,
                button,
                modifiers,
                x,
                y: y as i64,
                x_pixel_offset: 0,
                y_pixel_offset: 0,
            };
            term.mouse_event(event).is_ok()
        } else {
            false
        }
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
}
