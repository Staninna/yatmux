/// Context menu state.
#[derive(Clone)]
pub struct ContextMenu {
    /// Menu items (label, action identifier)
    pub items: Vec<(&'static str, ContextMenuAction)>,
    /// Screen position where menu was opened (for reference/debugging)
    #[allow(dead_code)]
    pub click_x: usize,
    /// Screen position where menu was opened (for reference/debugging)
    #[allow(dead_code)]
    pub click_y: usize,
    /// Actual rendered position (adjusted for screen boundaries)
    pub rendered_x: usize,
    pub rendered_y: usize,
    /// Currently hovered item index
    pub hovered: Option<usize>,
}

/// Actions that can be triggered from the context menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextMenuAction {
    Copy,
    Paste,
    SelectAll,
    Search,
    OpenUrl,
    ClearScrollback,
    Reset,
    ScrollToTop,
    ScrollToBottom,
    CopyLastOutput,
    JumpToPrevPrompt,
    JumpToNextPrompt,
}

use super::*;

impl ContextMenu {
    /// Calculate the rendered position for a context menu based on click position and screen boundaries
    pub fn calculate_rendered_position(
        click_x: usize,
        click_y: usize,
        menu_width: usize,
        menu_height: usize,
        buffer_width: usize,
        buffer_height: usize,
    ) -> (usize, usize) {
        let rendered_x = click_x.min(buffer_width.saturating_sub(menu_width));
        let rendered_y = click_y.min(buffer_height.saturating_sub(menu_height));
        (rendered_x, rendered_y)
    }
}

impl App {
    pub(super) fn context_menu_item_at_cursor(&self) -> Option<ContextMenuAction> {
        let menu = self.context_menu.as_ref()?;
        let cursor_pos = self.input.cursor_position;

        let (cell_w, cell_h) = self.renderer.font_renderer.cell_size(&self.config.font);
        let item_height = cell_h + 8; // cell height + padding

        // Calculate actual menu width based on items
        let padding_x = cell_w;
        let max_label_len = menu
            .items
            .iter()
            .map(|(label, _)| label.len())
            .max()
            .unwrap_or(8);
        let menu_width = max_label_len * cell_w + padding_x * 2;

        let x = cursor_pos.x as usize;
        let y = cursor_pos.y as usize;

        // Check if cursor is within menu bounds using rendered position
        if x < menu.rendered_x || x >= menu.rendered_x + menu_width {
            return None;
        }

        if y < menu.rendered_y {
            return None;
        }

        let relative_y = y - menu.rendered_y;
        let item_index = relative_y / item_height;

        menu.items.get(item_index).map(|(_, action)| *action)
    }

    pub(super) fn execute_context_menu_action(&mut self, action: ContextMenuAction) {
        match action {
            ContextMenuAction::Copy => {
                self.handle_copy();
            }
            ContextMenuAction::Paste => {
                self.handle_paste();
            }
            ContextMenuAction::SelectAll => {
                if let Some(tab) = self.active_tab_mut() {
                    if let Some(pane) = tab.focused_pane_mut() {
                        pane.view.select_all();
                    }
                }
                self.request_redraw();
            }
            ContextMenuAction::Search => {
                self.execute_action(Action::SearchFind);
            }
            ContextMenuAction::OpenUrl => {
                if let Some(url) = self.url_at_cursor() {
                    if let Err(e) = self.url_opener.open(&url) {
                        eprintln!("Failed to open URL: {e}");
                    }
                }
            }
            ContextMenuAction::ScrollToTop => {
                self.execute_action(Action::ScrollToTop);
            }
            ContextMenuAction::ScrollToBottom => {
                self.execute_action(Action::ScrollToBottom);
            }
            ContextMenuAction::ClearScrollback => {
                self.execute_action(Action::ClearScrollback);
            }
            ContextMenuAction::Reset => {
                self.execute_action(Action::Reset);
            }
            ContextMenuAction::CopyLastOutput => {
                self.execute_action(Action::CopyLastOutput);
            }
            ContextMenuAction::JumpToPrevPrompt => {
                self.execute_action(Action::JumpToPrevPrompt);
            }
            ContextMenuAction::JumpToNextPrompt => {
                self.execute_action(Action::JumpToNextPrompt);
            }
        }
    }

    pub fn context_menu(&self) -> Option<&ContextMenu> {
        self.context_menu.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_rendered_position() {
        // Test normal case - menu fits on screen
        let (rendered_x, rendered_y) = ContextMenu::calculate_rendered_position(
            100, // click_x
            50,  // click_y
            200, // menu_width
            100, // menu_height
            800, // buffer_width
            600, // buffer_height
        );
        assert_eq!(rendered_x, 100);
        assert_eq!(rendered_y, 50);

        // Test edge case - menu would exceed right boundary
        let (rendered_x, rendered_y) = ContextMenu::calculate_rendered_position(
            700, // click_x
            50,  // click_y
            200, // menu_width
            100, // menu_height
            800, // buffer_width
            600, // buffer_height
        );
        assert_eq!(rendered_x, 600); // 800 - 200
        assert_eq!(rendered_y, 50);

        // Test edge case - menu would exceed bottom boundary
        let (rendered_x, rendered_y) = ContextMenu::calculate_rendered_position(
            100, // click_x
            550, // click_y
            200, // menu_width
            100, // menu_height
            800, // buffer_width
            600, // buffer_height
        );
        assert_eq!(rendered_x, 100);
        assert_eq!(rendered_y, 500); // 600 - 100

        // Test corner case - menu would exceed both boundaries
        let (rendered_x, rendered_y) = ContextMenu::calculate_rendered_position(
            750, // click_x
            580, // click_y
            100, // menu_width
            50,  // menu_height
            800, // buffer_width
            600, // buffer_height
        );
        assert_eq!(rendered_x, 700); // 800 - 100
        assert_eq!(rendered_y, 550); // 600 - 50
    }

    #[test]
    fn test_calculate_rendered_position_zero_size() {
        // Test with zero menu dimensions
        let (rendered_x, rendered_y) = ContextMenu::calculate_rendered_position(
            100, // click_x
            100, // click_y
            0,   // menu_width
            0,   // menu_height
            800, // buffer_width
            600, // buffer_height
        );
        assert_eq!(rendered_x, 100);
        assert_eq!(rendered_y, 100);
    }

    #[test]
    fn test_calculate_rendered_position_small_buffer() {
        // Test with menu larger than buffer
        let (rendered_x, rendered_y) = ContextMenu::calculate_rendered_position(
            50,  // click_x
            50,  // click_y
            200, // menu_width
            200, // menu_height
            100, // buffer_width
            100, // buffer_height
        );
        assert_eq!(rendered_x, 0); // clamped to 0 since buffer_width - menu_width would underflow
        assert_eq!(rendered_y, 0); // clamped to 0 since buffer_height - menu_height would underflow
    }
}
