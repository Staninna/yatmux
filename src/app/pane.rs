//! Pane struct definition and clipboard operations.

use winit::window::CursorIcon;

use term::renderer::TerminalView;
use term::terminal::Terminal;

use crate::app::App;

/// A terminal pane with its view and scale.
pub struct Pane {
    pub terminal: Terminal,
    pub view: TerminalView,
    pub scale: usize,
}

impl App {
    /// Computes the cell size for a given scale.
    pub fn cell_size_for_scale(scale: usize) -> (usize, usize) {
        let scale = scale.clamp(1, 8);
        (8 * scale, 8 * scale)
    }

    /// Updates the cursor icon based on hover state.
    pub fn update_cursor(&self) {
        let Some(graphics) = &self.graphics else {
            return;
        };

        let (buffer_width, buffer_height) = self.last_buffer_size;
        if buffer_width == 0 || buffer_height == 0 {
            return;
        }

        let (rects, _divs) = self.pane_rects(buffer_width as usize, buffer_height as usize);
        let hovered_pane = self
            .pane_at_position(&rects, self.input.cursor_position)
            .map(|(id, _)| id);

        let has_hovered_url = if let Some(pane_id) = hovered_pane {
            self.active_tab()
                .and_then(|t| t.panes.get(&pane_id))
                .map(|p| p.view.has_hovered_url())
                .unwrap_or(false)
        } else {
            false
        };

        let cursor = if has_hovered_url {
            CursorIcon::Pointer
        } else {
            CursorIcon::Text
        };

        graphics.surface.window().set_cursor(cursor);
    }

    /// Handles paste from clipboard.
    pub fn handle_paste(&mut self) {
        let text = self.clipboard.read();
        let Some(text) = text else {
            return;
        };
        if text.is_empty() {
            return;
        }

        if let Some(pane) = self.focused_pane_mut() {
            pane.terminal.write(text.as_bytes());
            self.request_redraw();
        }
    }

    /// Handles copy to clipboard.
    pub fn handle_copy(&mut self) {
        let selected_text = self
            .focused_pane_mut()
            .and_then(|pane| pane.view.get_selected_text());

        if let Some(text) = selected_text {
            if self.clipboard.write(&text) {
                eprintln!("Copied {} characters to clipboard", text.len());
            }
        }
    }
}
