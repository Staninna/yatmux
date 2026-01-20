use super::*;
use super::plugins::PluginEvent;
use serde_json::json;

impl App {
    pub(super) fn handle_mouse_button(&mut self, state: ElementState, button: MouseButton) {
        match button {
            MouseButton::Left => self.handle_left_click(state),
            MouseButton::Right => self.handle_right_click(state),
            MouseButton::Middle => self.handle_middle_click(state),
            _ => {}
        }
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

    fn handle_left_click(&mut self, state: ElementState) {
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
            self.dispatch_plugin_event(PluginEvent {
                event: "pane_focus_changed".to_string(),
                action: None,
                source: None,
                tab_id: Some(tab_id),
                pane_id: Some(pane_id),
                data: Some(json!({ "reason": "mouse", "cwd": cwd })),
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
                use yatmux::terminal::{
                    KeyModifiers, MouseButton as TermMouseButton, MouseEventKind,
                };
                let kind = match state {
                    ElementState::Pressed => MouseEventKind::Press,
                    ElementState::Released => MouseEventKind::Release,
                };
                let modifiers = KeyModifiers::NONE; // TODO: track modifier keys
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

    /// Calculates the drop position (insertion index) for a tab being dragged to x-coordinate
    fn calculate_tab_drop_position(&self, x: f64) -> usize {
        let buffer_width = self.last_buffer_size.0 as usize;
        let num_tabs = self.tabs.len();

        if num_tabs == 0 {
            return 0;
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

        let cursor_x = x as usize;

        // If before first tab, drop at position 0
        if cursor_x < side_padding {
            return 0;
        }

        // Calculate midpoints of each tab boundary
        let mut x_offset = side_padding;
        for idx in 0..num_tabs {
            let tab_x0 = x_offset;
            let tab_x1 = if idx == num_tabs - 1 {
                (x_offset + tab_width).min(buffer_width.saturating_sub(side_padding))
            } else {
                (x_offset + tab_width).min(buffer_width)
            };

            let tab_midpoint = (tab_x0 + tab_x1) / 2;

            if cursor_x < tab_midpoint {
                return idx;
            }

            x_offset = tab_x1 + tab_gap;
        }

        // After all tabs, drop at end
        num_tabs
    }

    fn handle_tab_bar_click(&mut self) {
        if let Some(tab_idx) = self.calculate_tab_at_position(self.input.cursor_position.x) {
            self.goto_tab(tab_idx);
        }
    }

    fn handle_right_click(&mut self, state: ElementState) {
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
            if let Some(tab_idx) = self.calculate_tab_at_position(cursor_pos.x) {
                self.goto_tab(tab_idx);
            } else {
                return;
            }

            let mut items: Vec<(&'static str, ContextMenuAction)> = Vec::new();
            items.push(("New Tab", ContextMenuAction::NewTab));
            items.push(("Close Tab", ContextMenuAction::CloseTab));

            if self.active_tab > 0 {
                items.push(("Move Tab Left", ContextMenuAction::MoveTabLeft));
                items.push(("Move Tab to Start", ContextMenuAction::MoveTabToStart));
            }
            if self.active_tab + 1 < self.tabs.len() {
                items.push(("Move Tab Right", ContextMenuAction::MoveTabRight));
                items.push(("Move Tab to End", ContextMenuAction::MoveTabToEnd));
            }
            if self.tabs.len() > 1 {
                items.push(("Next Tab", ContextMenuAction::NextTab));
                items.push(("Previous Tab", ContextMenuAction::PrevTab));
                items.push(("Close Other Tabs", ContextMenuAction::CloseOtherTabs));
            }
            if self.active_tab + 1 < self.tabs.len() {
                items.push((
                    "Close Tabs to the Right",
                    ContextMenuAction::CloseTabsToRight,
                ));
            }

            let (cell_w, cell_h) = self.renderer.font_renderer.cell_size(&self.config.font);
            let padding_x = cell_w;
            let max_label_len = items
                .iter()
                .map(|(label, _)| label.len())
                .max()
                .unwrap_or(8);
            let menu_width = max_label_len * cell_w + padding_x * 2;
            let item_height = cell_h + 8; // cell height + padding
            let menu_height = items.len() * item_height;

            let (rendered_x, rendered_y) = ContextMenu::calculate_rendered_position(
                x,
                y,
                menu_width,
                menu_height,
                self.last_buffer_size.0 as usize,
                self.last_buffer_size.1 as usize,
            );

            self.context_menu = Some(ContextMenu {
                items,
                click_x: x,
                click_y: y,
                rendered_x,
                rendered_y,
                hovered: Some(0),
            });
            self.request_redraw();
            return;
        }

        // Build context menu items based on current state
        let mut items: Vec<(&'static str, ContextMenuAction)> = Vec::new();

        let focused_pane = self.active_tab().and_then(|t| t.focused_pane());

        let has_selection = focused_pane
            .map(|p| p.view.has_selection())
            .unwrap_or(false);
        let has_last_output = focused_pane
            .and_then(|pane| pane.terminal.last_command_output())
            .is_some();
        let has_prompts = focused_pane
            .map(|pane| !pane.terminal.prompt_positions().is_empty())
            .unwrap_or(false);

        // Check if there's a URL under cursor
        let has_url = self.url_at_cursor().is_some();

        if has_selection {
            items.push(("Copy", ContextMenuAction::Copy));
        }
        items.push(("Paste", ContextMenuAction::Paste));
        items.push(("Select All", ContextMenuAction::SelectAll));
        items.push(("Search", ContextMenuAction::Search));
        if has_url {
            items.push(("Open URL", ContextMenuAction::OpenUrl));
        }

        items.push(("Scroll to Top", ContextMenuAction::ScrollToTop));
        items.push(("Scroll to Bottom", ContextMenuAction::ScrollToBottom));
        items.push(("Clear Scrollback", ContextMenuAction::ClearScrollback));
        items.push(("Reset Terminal", ContextMenuAction::Reset));
        if has_last_output {
            items.push(("Copy Last Output", ContextMenuAction::CopyLastOutput));
        }
        if has_prompts {
            items.push((
                "Jump to Previous Prompt",
                ContextMenuAction::JumpToPrevPrompt,
            ));
            items.push(("Jump to Next Prompt", ContextMenuAction::JumpToNextPrompt));
        }

        // Calculate menu dimensions for position adjustment
        let (cell_w, cell_h) = self.renderer.font_renderer.cell_size(&self.config.font);
        let padding_x = cell_w;
        let max_label_len = items
            .iter()
            .map(|(label, _)| label.len())
            .max()
            .unwrap_or(8);
        let menu_width = max_label_len * cell_w + padding_x * 2;
        let item_height = cell_h + 8; // cell height + padding
        let menu_height = items.len() * item_height;

        // Calculate rendered position adjusted for screen boundaries
        let (rendered_x, rendered_y) = ContextMenu::calculate_rendered_position(
            x,
            y,
            menu_width,
            menu_height,
            self.last_buffer_size.0 as usize,
            self.last_buffer_size.1 as usize,
        );

        self.context_menu = Some(ContextMenu {
            items,
            click_x: x,
            click_y: y,
            rendered_x,
            rendered_y,
            hovered: Some(0),
        });
        self.request_redraw();
    }

    fn handle_middle_click(&mut self, state: ElementState) {
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

        // Handle tab dragging
        let tab_bar_height = self.tab_bar_height();
        let mut drag_committed = false;
        let dragging_info = if let Some(ref mut drag) = self.input.tab_dragging {
            let distance = (position.x - drag.start_x).abs();
            let was_committed = drag.committed;

            // Commit drag if moved beyond threshold (5px)
            if !drag.committed && distance > 5.0 {
                drag.committed = true;
            }

            drag_committed = drag.committed;

            Some((was_committed, drag.committed))
        } else {
            None
        };

        if drag_committed && tab_bar_height > 0 {
            let drop_idx = self.calculate_tab_drop_position(position.x);
            if let Some(ref mut drag) = self.input.tab_dragging {
                drag.last_drop_idx = Some(drop_idx);
            }
        }

        if let Some((was_committed, now_committed)) = dragging_info {
            if !was_committed && now_committed {
                self.update_cursor(); // Change to grabbing cursor
            }
            if now_committed {
                self.request_redraw(); // Redraw for visual feedback
            }
            return; // Don't process other hover logic during drag
        }

        // Update context menu hover state if menu is open
        if let Some(ref mut menu) = self.context_menu {
            let (cell_w, cell_h) = self.renderer.font_renderer.cell_size(&self.config.font);
            let item_height = cell_h + 8;

            // Calculate actual menu width based on items
            let padding_x = cell_w;
            let max_label_len = menu
                .items
                .iter()
                .map(|(label, _)| label.len())
                .max()
                .unwrap_or(8);
            let menu_width = max_label_len * cell_w + padding_x * 2;

            let x = position.x as usize;
            let y = position.y as usize;

            // Check if cursor is within menu bounds using rendered position
            if x >= menu.rendered_x && x < menu.rendered_x + menu_width && y >= menu.rendered_y {
                let relative_y = y - menu.rendered_y;
                let item_index = relative_y / item_height;
                if item_index < menu.items.len() {
                    menu.hovered = Some(item_index);
                } else {
                    menu.hovered = None;
                }
            } else {
                menu.hovered = None;
            }
            self.request_redraw();
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
                use yatmux::terminal::{
                    KeyModifiers, MouseButton as TermMouseButton, MouseEventKind,
                };
                let modifiers = KeyModifiers::NONE;
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

    pub(super) fn handle_scroll(&mut self, delta: MouseScrollDelta) {
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

        let Some(tab) = self.active_tab_mut() else {
            return;
        };

        if let Some(pane) = tab.panes.get_mut(&target) {
            // Check if terminal wants mouse events (for scroll in apps like less, vim)
            if is_grabbed {
                use yatmux::terminal::{
                    KeyModifiers, MouseButton as TermMouseButton, MouseEventKind,
                };
                let button = if lines > 0 {
                    TermMouseButton::WheelUp(lines as usize)
                } else {
                    TermMouseButton::WheelDown((-lines) as usize)
                };
                let modifiers = KeyModifiers::NONE;
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
