use super::super::plugins::PluginEvent;
use super::super::*;
use serde_json::json;

impl App {
    pub(super) fn handle_left_click(&mut self, state: ElementState) {
        // Close context menu on any left click
        if self.context_menu.is_some() {
            if state == ElementState::Pressed {
                // Check if clicking on a menu item
                if let Some(action) = self.context_menu_item_at_cursor() {
                    self.execute_context_menu_action(action);
                }
                self.context_menu = None;
                self.request_redraw();
            }
            return;
        }

        let (buffer_width, buffer_height) = self.last_buffer_size;
        if buffer_width == 0 || buffer_height == 0 {
            return;
        }

        // Handle tab drag completion on release (before anything else)
        if state == ElementState::Released {
            if let Some(drag) = self.input.tab_dragging.take() {
                let cursor_pos = self.input.cursor_position;
                let tab_bar_height = self.tab_bar_height();

                if drag.committed {
                    if let Some(drop_idx) = drag.last_drop_idx {
                        if drop_idx != drag.tab_index {
                            self.reorder_tab(drag.tab_index, drop_idx);
                            self.layout_dirty = true;
                        }
                    }
                } else if cursor_pos.y >= 0.0 && (cursor_pos.y as usize) < tab_bar_height {
                    // Click without drag in tab bar - switch to clicked tab
                    self.goto_tab(drag.tab_index);
                }

                self.request_redraw();
                self.update_cursor();
                return;
            }
        }

        // Check if click is in tab bar
        let tab_bar_height = self.tab_bar_height();
        let cursor_pos = self.input.cursor_position;
        if tab_bar_height > 0 && (cursor_pos.y as usize) < tab_bar_height {
            if state == ElementState::Pressed {
                // Initiate potential drag operation (only if multiple tabs)
                if self.tabs.len() > 1 {
                    if let Some(tab_idx) = self.calculate_tab_at_position(cursor_pos.x) {
                        self.input.tab_dragging = Some(crate::app::input::TabDragState {
                            tab_index: tab_idx,
                            start_x: cursor_pos.x,
                            committed: false,
                            last_drop_idx: None,
                        });
                    }
                }
            }
            return;
        }

        let (rects, _divs) = self.pane_rects(buffer_width as usize, buffer_height as usize);
        let Some((pane_id, pane_rect)) = self.pane_at_position(&rects, cursor_pos) else {
            return;
        };

        let tab_id = if let Some(tab) = self.active_tab_mut() {
            tab.set_focus(pane_id);
            Some(tab.id)
        } else {
            None
        };
        if let Some(tab_id) = tab_id {
            let cwd = self.cwd_for_event(Some(tab_id), Some(pane_id));
            let profile = self.profile_for_event(Some(tab_id), Some(pane_id));
            self.dispatch_plugin_event(PluginEvent {
                event: "pane_focus_changed".to_string(),
                action: None,
                source: None,
                tab_id: Some(tab_id),
                pane_id: Some(pane_id),
                data: Some(json!({ "reason": "mouse", "cwd": cwd, "profile": profile })),
            });
        }
        self.refresh_active_tab_title_from_focused_pane();

        // Extract what we need before borrowing tab mutably
        let local = Self::localize_pos(pane_rect, cursor_pos);

        // Check if terminal wants mouse events
        let (is_mouse_grabbed, cell_coords, _scale) = {
            let Some(tab) = self.active_tab() else {
                return;
            };
            let Some(pane) = tab.panes.get(&pane_id) else {
                return;
            };
            let mut pane_font_config = self.config.font.clone();
            pane_font_config.scale = pane.scale;
            let (cell_w, cell_h) = self.renderer.font_renderer.cell_size(&pane_font_config);
            let coords = pane.view.window_to_cell(local.x, local.y, cell_w, cell_h);
            (pane.terminal.is_mouse_grabbed(), coords, pane.scale)
        };

        // If terminal application wants mouse events, forward them
        if is_mouse_grabbed {
            if let Some((row, col)) = cell_coords {
                use yatmux::terminal::{MouseButton as TermMouseButton, MouseEventKind};
                let kind = match state {
                    ElementState::Pressed => MouseEventKind::Press,
                    ElementState::Released => MouseEventKind::Release,
                };
                let modifiers = self.terminal_key_modifiers();
                if let Some(tab) = self.active_tab() {
                    if let Some(pane) = tab.panes.get(&pane_id) {
                        pane.terminal
                            .mouse_event(col, row, TermMouseButton::Left, kind, modifiers);
                    }
                }
            }
            return;
        }

        match state {
            ElementState::Pressed => {
                // Get pane scale and cell size first
                let (cell_w, cell_h) = {
                    let Some(tab) = self.active_tab() else {
                        return;
                    };
                    let Some(pane) = tab.panes.get(&pane_id) else {
                        return;
                    };
                    let mut pane_font_config = self.config.font.clone();
                    pane_font_config.scale = pane.scale;
                    self.renderer.font_renderer.cell_size(&pane_font_config)
                };

                // Now get URL with mutable borrow
                let (_scale, url_to_open, cell_coords) = {
                    let Some(tab) = self.active_tab_mut() else {
                        return;
                    };
                    let Some(pane) = tab.panes.get_mut(&pane_id) else {
                        return;
                    };

                    let coords = pane.view.window_to_cell(local.x, local.y, cell_w, cell_h);
                    let url = coords.and_then(|(row, col)| pane.view.url_at(row, col));
                    (pane.scale, url, coords)
                };

                if let Some(url) = url_to_open {
                    if let Err(e) = self.url_opener.open(&url) {
                        eprintln!("Failed to open URL: {e}");
                    }
                    return;
                }

                if let Some((row, col)) = cell_coords {
                    // Click-to-position cursor in shell input when semantic zones are available.
                    if self.try_click_move_shell_cursor(pane_id, row, col) {
                        self.request_redraw();
                        return;
                    }

                    self.input.mouse_selecting = true;
                    if let Some(tab) = self.active_tab_mut() {
                        if let Some(pane) = tab.panes.get_mut(&pane_id) {
                            pane.view.start_selection(row, col);
                        }
                    }
                    self.request_redraw();
                }
            }
            ElementState::Released => {
                self.input.mouse_selecting = false;
            }
        }
    }

    pub(super) fn handle_right_click(&mut self, state: ElementState) {
        if state != ElementState::Pressed {
            return;
        }

        // Close existing menu if clicking elsewhere
        if self.context_menu.is_some() {
            self.context_menu = None;
            self.request_redraw();
            return;
        }

        let cursor_pos = self.input.cursor_position;
        let x = cursor_pos.x as usize;
        let y = cursor_pos.y as usize;

        let tab_bar_height = self.tab_bar_height();
        if tab_bar_height > 0 && y < tab_bar_height {
            self.open_tab_bar_context_menu(cursor_pos.x, x, y);
            return;
        }

        self.open_pane_context_menu(x, y);
    }

    pub(super) fn handle_middle_click(&mut self, state: ElementState) {
        if state != ElementState::Pressed {
            return;
        }

        // Close context menu if open
        if self.context_menu.is_some() {
            self.context_menu = None;
            self.request_redraw();
            return;
        }

        // Paste from clipboard
        self.handle_paste();
    }
}
