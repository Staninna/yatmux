/// Context menu state.
#[derive(Clone)]
pub struct ContextMenu {
    /// Menu items (label, action identifier)
    pub items: Vec<(&'static str, ContextMenuAction)>,
    /// Screen position where menu was opened
    pub x: usize,
    pub y: usize,
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

impl App {
    pub(super) fn context_menu_item_at_cursor(&self) -> Option<ContextMenuAction> {
        let menu = self.context_menu.as_ref()?;
        let cursor_pos = self.input.cursor_position;

        let scale = self.config.font.scale.clamp(1, 8);
        let item_height = 8 * scale + 8; // cell height + padding
        let menu_width = 12 * 8 * scale; // ~12 chars width

        let x = cursor_pos.x as usize;
        let y = cursor_pos.y as usize;

        // Check if cursor is within menu bounds
        if x < menu.x || x >= menu.x + menu_width {
            return None;
        }

        if y < menu.y {
            return None;
        }

        let relative_y = y - menu.y;
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
