//! Action execution for the terminal application.

use term::config::Action;

use crate::app::App;
use crate::app::layout::SplitDir;

impl App {
    /// Executes a configured action.
    pub fn execute_action(&mut self, action: Action) {
        match action {
            Action::SplitVertical => self.split_focused(SplitDir::Vertical),
            Action::SplitHorizontal => self.split_focused(SplitDir::Horizontal),

            Action::FocusLeft => self.focus_move(SplitDir::Vertical, false),
            Action::FocusRight => self.focus_move(SplitDir::Vertical, true),
            Action::FocusUp => self.focus_move(SplitDir::Horizontal, false),
            Action::FocusDown => self.focus_move(SplitDir::Horizontal, true),

            // Resize: we interpret arrows as "expand the pane in that direction".
            Action::ResizeLeft => self.resize_focused(SplitDir::Vertical, false),
            Action::ResizeRight => self.resize_focused(SplitDir::Vertical, true),
            Action::ResizeUp => self.resize_focused(SplitDir::Horizontal, false),
            Action::ResizeDown => self.resize_focused(SplitDir::Horizontal, true),
            Action::ClosePane => self.close_focused_pane(),
            Action::ToggleHelp => {
                self.show_help = !self.show_help;
                if self.show_help {
                    self.help_scroll = 0;
                    self.help_max_scroll = 0;
                }
                self.request_redraw();
            }

            Action::ZoomIn => {
                self.zoom_focused(1);
            }
            Action::ZoomOut => {
                self.zoom_focused(-1);
            }
            Action::ZoomReset => {
                self.zoom_reset_focused();
            }

            Action::Copy => self.handle_copy(),
            Action::Paste => self.handle_paste(),

            Action::ScrollPageUp => {
                if let Some(pane) = self.focused_pane_mut() {
                    pane.view.scrollback_scroll_by(24);
                }
                self.request_redraw();
            }
            Action::ScrollPageDown => {
                if let Some(pane) = self.focused_pane_mut() {
                    pane.view.scrollback_scroll_by(-24);
                }
                self.request_redraw();
            }
            Action::ScrollLineUp => {
                if let Some(pane) = self.focused_pane_mut() {
                    pane.view.scrollback_scroll_by(1);
                }
                self.request_redraw();
            }
            Action::ScrollLineDown => {
                if let Some(pane) = self.focused_pane_mut() {
                    pane.view.scrollback_scroll_by(-1);
                }
                self.request_redraw();
            }
            Action::ScrollToTop => {
                if let Some(pane) = self.focused_pane_mut() {
                    pane.view.scrollback_scroll_by(isize::MAX);
                }
                self.request_redraw();
            }
            Action::ScrollToBottom => {
                if let Some(pane) = self.focused_pane_mut() {
                    pane.view.scrollback_scroll_by(isize::MIN);
                }
                self.request_redraw();
            }
            Action::ClearScrollback => {
                if let Some(pane) = self.focused_pane_mut() {
                    pane.terminal.clear_scrollback();
                    pane.view.clear_scrollback();
                }
                self.request_redraw();
            }
            Action::Reset => {
                if let Some(pane) = self.focused_pane_mut() {
                    pane.terminal.clear_scrollback();
                    pane.view.clear_scrollback();
                    pane.view.clear_selection();
                }
                self.request_redraw();
            }
            Action::SearchFind => {
                if let Some(pane) = self.focused_pane_mut() {
                    pane.view.activate_search();
                }
                self.request_redraw();
            }

            // Search mode actions are handled inside `handle_search_keyboard`.
            Action::SearchClose
            | Action::SearchNext
            | Action::SearchPrev
            | Action::SearchToggleCase
            | Action::SearchConfirm => {}
        }
    }
}
