//! Pane struct definition and clipboard operations.

use winit::window::CursorIcon;

use yatmux::renderer::TerminalView;
use yatmux::terminal::{ShellIntegrationStatus, Terminal};

use crate::app::App;

/// Shadow prompt state for type-ahead during command execution.
#[derive(Debug, Default, Clone)]
pub struct ShadowPrompt {
    /// Buffered input that will be sent when command completes.
    pub buffer: String,
    /// Cursor position within the buffer.
    pub cursor: usize,
    /// Whether the shadow prompt should be visible.
    pub visible: bool,
}

impl ShadowPrompt {
    /// Insert a character at the cursor position.
    pub fn insert(&mut self, ch: char) {
        self.buffer.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
        self.visible = true;
    }

    /// Insert a string at the cursor position.
    pub fn insert_str(&mut self, s: &str) {
        self.buffer.insert_str(self.cursor, s);
        self.cursor += s.len();
        self.visible = true;
    }

    /// Delete character before cursor (backspace).
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            // Find the previous character boundary
            let prev = self.buffer[..self.cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.buffer.remove(prev);
            self.cursor = prev;
        }
    }

    /// Delete character at cursor (delete key).
    pub fn delete(&mut self) {
        if self.cursor < self.buffer.len() {
            self.buffer.remove(self.cursor);
        }
    }

    /// Move cursor left.
    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.buffer[..self.cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    /// Move cursor right.
    pub fn move_right(&mut self) {
        if self.cursor < self.buffer.len() {
            self.cursor = self.buffer[self.cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor + i)
                .unwrap_or(self.buffer.len());
        }
    }

    /// Move cursor to start.
    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to end.
    pub fn move_end(&mut self) {
        self.cursor = self.buffer.len();
    }

    /// Clear the buffer and reset state.
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
        self.visible = false;
    }

    /// Take the buffer contents, clearing the state.
    pub fn take(&mut self) -> String {
        let s = std::mem::take(&mut self.buffer);
        self.cursor = 0;
        self.visible = false;
        s
    }

    /// Check if buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

/// A terminal pane with its view and scale.
pub struct Pane {
    pub terminal: Terminal,
    pub view: TerminalView,
    pub scale: usize,

    pub shell_title: Option<String>,
    pub shell_cwd: Option<String>,

    pub shell_integration: ShellIntegrationStatus,

    /// Shadow prompt for type-ahead during command execution.
    pub shadow_prompt: ShadowPrompt,
    /// Whether shadow prompt is enabled for this pane.
    pub shadow_prompt_enabled: bool,
    /// Cached state: whether a command is currently running (updated on PTY output only).
    pub command_running: bool,
}

impl App {
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
                self.show_toast("Copied to clipboard");
            }
        }
    }
}
