use super::super::*;

impl App {
    pub(super) fn update_context_menu_hover(&mut self, position: PhysicalPosition<f64>) -> bool {
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
            return true;
        }

        false
    }

    pub(super) fn open_tab_bar_context_menu(&mut self, cursor_x: f64, x: usize, y: usize) {
        if let Some(tab_idx) = self.calculate_tab_at_position(cursor_x) {
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
    }

    pub(super) fn open_pane_context_menu(&mut self, x: usize, y: usize) {
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
}
