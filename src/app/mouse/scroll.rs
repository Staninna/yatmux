use super::super::*;

impl App {
    pub(crate) fn handle_scroll(&mut self, delta: MouseScrollDelta) {
        let lines = match delta {
            MouseScrollDelta::LineDelta(_, y) => {
                (y * self.config.terminal.scroll_speed).round() as isize
            }
            MouseScrollDelta::PixelDelta(pos) => {
                // Use the focused pane's cell height as a reasonable heuristic.
                let cell_h = self
                    .active_tab()
                    .and_then(|t| t.focused_pane())
                    .map(|p| {
                        let mut pane_font_config = self.config.font.clone();
                        pane_font_config.scale = p.scale;
                        self.renderer.font_renderer.cell_size(&pane_font_config).1
                    })
                    .unwrap_or_else(|| self.renderer.font_renderer.cell_size(&self.config.font).1);
                (pos.y / cell_h as f64).round() as isize
            }
        };

        if lines == 0 {
            return;
        }

        // When the help overlay is open, scroll it instead of the terminal.
        if self.show_help {
            if lines > 0 {
                self.help_scroll = self.help_scroll.saturating_sub(lines as usize);
            } else {
                self.help_scroll = (self.help_scroll + (-lines) as usize).min(self.help_max_scroll);
            }
            self.request_redraw();
            return;
        }

        let (buffer_width, buffer_height) = self.last_buffer_size;
        let cursor_pos = self.input.cursor_position;

        // Determine target pane and compute cell size before mutable borrow
        let (target, is_grabbed, cell_w, cell_h) = {
            let Some(tab) = self.active_tab() else {
                return;
            };

            let target = if buffer_width > 0 && buffer_height > 0 {
                let (rects, _divs) = tab.pane_rects(buffer_width as usize, buffer_height as usize);
                rects
                    .iter()
                    .find(|(_, r)| r.contains(cursor_pos.x, cursor_pos.y))
                    .map(|(id, _)| *id)
                    .unwrap_or(tab.focused_pane)
            } else {
                tab.focused_pane
            };

            let Some(pane) = tab.panes.get(&target) else {
                return;
            };

            let mut pane_font_config = self.config.font.clone();
            pane_font_config.scale = pane.scale;
            let cell_size = self.renderer.font_renderer.cell_size(&pane_font_config);
            let grabbed = pane.terminal.is_mouse_grabbed();

            (target, grabbed, cell_size.0, cell_size.1)
        };

        // Get modifiers before mutable borrow
        let modifiers = self.terminal_key_modifiers();

        let Some(tab) = self.active_tab_mut() else {
            return;
        };

        if let Some(pane) = tab.panes.get_mut(&target) {
            // Check if terminal wants mouse events (for scroll in apps like less, vim)
            if is_grabbed {
                use yatmux::terminal::{MouseButton as TermMouseButton, MouseEventKind};
                let button = if lines > 0 {
                    TermMouseButton::WheelUp(lines as usize)
                } else {
                    TermMouseButton::WheelDown((-lines) as usize)
                };
                // Use current cursor position in cell coordinates
                let (row, col) = pane
                    .view
                    .window_to_cell(cursor_pos.x, cursor_pos.y, cell_w, cell_h)
                    .unwrap_or((0, 0));
                pane.terminal
                    .mouse_event(col, row, button, MouseEventKind::Press, modifiers);
            } else {
                pane.view.scrollback_scroll_by(lines);
            }
            self.request_redraw();
        }
    }
}
