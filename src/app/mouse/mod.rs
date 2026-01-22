mod buttons;
mod context_menu;
mod drag;
mod scroll;

use super::*;

impl App {
    pub(super) fn handle_mouse_button(&mut self, state: ElementState, button: MouseButton) {
        match button {
            MouseButton::Left => self.handle_left_click(state),
            MouseButton::Right => self.handle_right_click(state),
            MouseButton::Middle => self.handle_middle_click(state),
            _ => {}
        }
    }

    fn terminal_key_modifiers(&self) -> yatmux::terminal::KeyModifiers {
        use yatmux::terminal::KeyModifiers;

        let mut out = KeyModifiers::NONE;
        if self.input.modifiers.shift_key() {
            out |= KeyModifiers::SHIFT;
        }
        if self.input.modifiers.alt_key() {
            out |= KeyModifiers::ALT;
        }
        if self.input.modifiers.control_key() {
            out |= KeyModifiers::CTRL;
        }
        out
    }

    fn normalize_cursor_position(
        &mut self,
        position: PhysicalPosition<f64>,
        scale: f64,
        logical_size: winit::dpi::LogicalSize<f64>,
    ) -> PhysicalPosition<f64> {
        if scale == 1.0 {
            self.input.cursor_coords_are_physical = Some(true);
            return position;
        }

        if position.x > logical_size.width || position.y > logical_size.height {
            self.input.cursor_coords_are_physical = Some(true);
        }

        match self.input.cursor_coords_are_physical {
            Some(true) => position,
            Some(false) | None => {
                let logical = winit::dpi::LogicalPosition::new(position.x, position.y);
                logical.to_physical(scale)
            }
        }
    }

    pub(super) fn handle_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        let normalized = if let Some(graphics) = &self.graphics {
            let window = graphics.surface.window();
            let scale = window.scale_factor();
            let logical_size = window.inner_size().to_logical::<f64>(scale);
            self.normalize_cursor_position(position, scale, logical_size)
        } else {
            position
        };
        self.input.cursor_position = normalized;

        let (buffer_width, buffer_height) = self.last_buffer_size;
        if buffer_width == 0 || buffer_height == 0 {
            self.update_cursor();
            return;
        }

        let tab_bar_height = self.tab_bar_height();
        if self.update_tab_drag_on_move(position, tab_bar_height) {
            return;
        }

        if self.update_context_menu_hover(position) {
            return;
        }

        let (rects, _divs) = self.pane_rects(buffer_width as usize, buffer_height as usize);
        let Some((pane_id, pane_rect)) = self.pane_at_position(&rects, position) else {
            self.update_cursor();
            return;
        };

        let local = Self::localize_pos(pane_rect, position);
        let mouse_selecting = self.input.mouse_selecting;

        // Check if terminal wants mouse events
        let is_mouse_grabbed = self
            .active_tab()
            .and_then(|t| t.panes.get(&pane_id))
            .map(|p| p.terminal.is_mouse_grabbed())
            .unwrap_or(false);

        if is_mouse_grabbed {
            let Some(tab) = self.active_tab() else {
                return;
            };
            let Some(pane) = tab.panes.get(&pane_id) else {
                return;
            };
            let mut pane_font_config = self.config.font.clone();
            pane_font_config.scale = pane.scale;
            let (cell_w, cell_h) = self.renderer.font_renderer.cell_size(&pane_font_config);
            if let Some((row, col)) = pane.view.window_to_cell(local.x, local.y, cell_w, cell_h) {
                use yatmux::terminal::{MouseButton as TermMouseButton, MouseEventKind};
                let modifiers = self.terminal_key_modifiers();
                pane.terminal.mouse_event(
                    col,
                    row,
                    TermMouseButton::None,
                    MouseEventKind::Move,
                    modifiers,
                );
            }
            return;
        }

        // Get pane scale and compute cell size before mutable borrow
        let (_pane_scale, focused_pane, cell_w, cell_h) = {
            let Some(tab) = self.active_tab() else {
                return;
            };
            let Some(pane) = tab.panes.get(&pane_id) else {
                return;
            };
            let mut pane_font_config = self.config.font.clone();
            pane_font_config.scale = pane.scale;
            let cell_size = self.renderer.font_renderer.cell_size(&pane_font_config);
            (pane.scale, tab.focused_pane, cell_size.0, cell_size.1)
        };

        let Some(tab) = self.active_tab_mut() else {
            return;
        };

        let Some(pane) = tab.panes.get_mut(&pane_id) else {
            return;
        };

        if mouse_selecting && pane_id == focused_pane {
            if let Some((row, col)) = pane.view.window_to_cell(local.x, local.y, cell_w, cell_h) {
                pane.view.update_selection(row, col);
                self.request_redraw();
            }
        } else {
            if let Some((row, col)) = pane.view.window_to_cell(local.x, local.y, cell_w, cell_h) {
                if pane.view.update_url_hover(row, col) {
                    self.request_redraw();
                }
            } else {
                pane.view.clear_url_hover();
            }
            self.update_cursor();
        }
    }

    /// Calculates which tab index contains the given x-coordinate
    fn calculate_tab_at_position(&self, x: f64) -> Option<usize> {
        let buffer_width = self.last_buffer_size.0 as usize;
        let num_tabs = self.tabs.len();

        if num_tabs == 0 {
            return None;
        }

        let style = &yatmux::renderer::UiStyle::from_config(&self.config);
        let mut tab_font_config = self.config.font.clone();
        tab_font_config.scale = style.tab_font_scale;

        let (cell_w, _) = self.renderer.font_renderer.cell_size(&tab_font_config);
        let tab_gap = style.tab_gap_px;
        let side_padding = style.tab_side_padding_px;
        let total_gap_width = tab_gap * (num_tabs.saturating_sub(1)) + side_padding * 2;
        let available_width = buffer_width.saturating_sub(total_gap_width);
        let tab_width = (available_width / num_tabs)
            .min(cell_w * style.tab_max_width_cells + style.tab_max_width_px_extra);

        let click_x = x as usize;

        if click_x < side_padding {
            return None;
        }

        let mut x_offset = side_padding;
        for (idx, _) in self.tabs.iter().enumerate() {
            let tab_x0 = x_offset;
            let tab_x1 = if idx == num_tabs - 1 {
                (x_offset + tab_width).min(buffer_width.saturating_sub(side_padding))
            } else {
                (x_offset + tab_width).min(buffer_width)
            };

            if click_x >= tab_x0 && click_x < tab_x1 {
                return Some(idx);
            }

            x_offset = tab_x1 + tab_gap;
            if x_offset >= buffer_width {
                break;
            }
        }

        None
    }
}
