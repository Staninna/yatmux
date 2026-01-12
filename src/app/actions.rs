//! Action execution for the terminal application.

use term::config::Action;

use crate::app::App;
use crate::app::layout::SplitDir;

impl App {
    /// Executes a configured action.
    pub fn execute_action(&mut self, action: Action) {
        match action {
            // Disabled action - do nothing
            Action::None => {}

            // Tab actions
            Action::NewTab => {
                self.new_tab();
                self.request_redraw();
            }
            Action::CloseTab => {
                self.close_active_tab();
                self.request_redraw();
            }
            Action::NextTab => self.next_tab(),
            Action::PrevTab => self.prev_tab(),
            Action::Tab1 => self.goto_tab(0),
            Action::Tab2 => self.goto_tab(1),
            Action::Tab3 => self.goto_tab(2),
            Action::Tab4 => self.goto_tab(3),
            Action::Tab5 => self.goto_tab(4),
            Action::Tab6 => self.goto_tab(5),
            Action::Tab7 => self.goto_tab(6),
            Action::Tab8 => self.goto_tab(7),
            Action::Tab9 => self.goto_tab(8),

            // Pane actions
            Action::SplitVertical => {
                self.split_pane(SplitDir::Vertical);
            }
            Action::SplitHorizontal => {
                self.split_pane(SplitDir::Horizontal);
            }

            Action::FocusLeft => self.focus_move(SplitDir::Vertical, false),
            Action::FocusRight => self.focus_move(SplitDir::Vertical, true),
            Action::FocusUp => self.focus_move(SplitDir::Horizontal, false),
            Action::FocusDown => self.focus_move(SplitDir::Horizontal, true),

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

            Action::ZoomIn => self.zoom_focused(1),
            Action::ZoomOut => self.zoom_focused(-1),
            Action::ZoomReset => self.zoom_reset_focused(),

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

            // Search mode actions are handled inside `apply_search_input`.
            Action::SearchClose
            | Action::SearchNext
            | Action::SearchPrev
            | Action::SearchToggleCase
            | Action::SearchToggleRegex
            | Action::SearchConfirm => {}
        }
    }

    /// Moves focus in the given direction within the active tab.
    fn focus_move(&mut self, dir: SplitDir, positive: bool) {
        let (buffer_width, buffer_height) = self.last_buffer_size;
        if buffer_width == 0 || buffer_height == 0 {
            return;
        }

        let tab_bar_height = self.tab_bar_height();
        let pane_height = (buffer_height as usize).saturating_sub(tab_bar_height);

        let Some(tab) = self.active_tab_mut() else {
            return;
        };

        let (rects, _) = tab.pane_rects(buffer_width as usize, pane_height);
        if tab.focus_move(dir, positive, &rects) {
            self.update_cursor();
            self.request_redraw();
        }
    }

    /// Resizes the focused pane in the given direction.
    fn resize_focused(&mut self, dir: SplitDir, negative: bool) {
        if let Some(tab) = self.active_tab_mut() {
            if tab.resize_focused(dir, negative) {
                self.layout_dirty = true;
                self.request_redraw();
            }
        }
    }

    /// Closes the focused pane in the active tab.
    fn close_focused_pane(&mut self) {
        let should_close_tab = self
            .active_tab_mut()
            .map(|t| t.close_focused_pane())
            .unwrap_or(false);

        if should_close_tab {
            self.close_active_tab();
        }

        self.layout_dirty = true;
        self.update_cursor();
        self.request_redraw();
    }

    /// Splits the focused pane in the given direction.
    fn split_pane(&mut self, dir: SplitDir) {
        let scale = self.config.font.scale;
        let scrollback = self.config.terminal.scrollback_lines;
        let min_size = self.config.pane.min_size();
        let proxy = self.event_proxy.clone();

        // Get the current focused pane's rect
        let focused_rect = self.focused_pane_rect();

        if let Some(tab) = self.active_tab_mut() {
            if tab.split_focused(
                dir,
                scale,
                scrollback,
                proxy.as_ref(),
                focused_rect,
                min_size,
            ) {
                self.layout_dirty = true;
                self.request_redraw();
            }
        }
    }

    /// Returns the rectangle of the currently focused pane, if any.
    fn focused_pane_rect(&self) -> Option<crate::app::layout::Rect> {
        let Some(graphics) = &self.graphics else {
            return None;
        };
        let size = graphics.surface.window().inner_size();
        let tab_bar_height = self.tab_bar_height();
        let pane_height = (size.height as usize).saturating_sub(tab_bar_height);

        let tab = self.active_tab()?;
        let (rects, _) = tab.pane_rects(size.width as usize, pane_height);
        rects
            .into_iter()
            .find(|(id, _)| *id == tab.focused_pane)
            .map(|(_, rect)| rect)
    }

    /// Zooms the focused pane by the given delta.
    fn zoom_focused(&mut self, delta: isize) {
        let Some(pane) = self.focused_pane_mut() else {
            return;
        };

        let new_scale = (pane.scale as isize + delta).clamp(1, 8) as usize;
        if new_scale == pane.scale {
            return;
        }

        pane.scale = new_scale;
        self.layout_dirty = true;
        self.update_cursor();
        self.request_redraw();
    }

    /// Resets the focused pane's zoom to the default scale.
    fn zoom_reset_focused(&mut self) {
        let new_scale = self.config.font.scale.clamp(1, 8);

        let Some(pane) = self.focused_pane_mut() else {
            return;
        };

        if pane.scale == new_scale {
            return;
        }

        pane.scale = new_scale;
        self.layout_dirty = true;
        self.update_cursor();
        self.request_redraw();
    }
}
