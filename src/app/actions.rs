//! Action execution for the terminal application.

use yatmux::config::Action;

use super::plugins::{ActionSource, PluginEvent};
use serde_json::json;

use crate::app::App;
use crate::app::layout::SplitDir;

impl App {
    /// Executes a configured action.
    pub fn execute_action(&mut self, action: Action) {
        self.execute_action_with_source(action, ActionSource::User);
    }

    pub fn execute_action_with_source(&mut self, action: Action, source: ActionSource) {
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
                } else {
                    // Deactivate filter when closing help
                    self.help_filter.deactivate();
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

            // Config
            Action::ReloadConfig => self.reload_config(),

            // Shell integration actions
            Action::CopyLastOutput => self.copy_last_output(),
            Action::JumpToPrevPrompt => self.jump_to_prompt(false),
            Action::JumpToNextPrompt => self.jump_to_prompt(true),
            Action::ToggleShadowPrompt => {
                use yatmux::config::ShadowPromptMode;

                if self.config.shell_integration.shadow_prompt == ShadowPromptMode::Off {
                    self.show_toast("Shadow prompt is disabled in config");
                    return;
                }

                let message = if let Some(pane) = self.focused_pane_mut() {
                    pane.shadow_prompt_enabled = !pane.shadow_prompt_enabled;
                    if !pane.shadow_prompt_enabled {
                        pane.shadow_prompt.clear();
                    }
                    if pane.shadow_prompt_enabled {
                        "Shadow prompt: ON"
                    } else {
                        "Shadow prompt: OFF"
                    }
                } else {
                    return;
                };

                self.show_toast(message);
            }
        }

        self.dispatch_action_event(action, source);
    }

    /// Copies the last command's output to clipboard.
    fn copy_last_output(&mut self) {
        let output = self
            .focused_pane_mut()
            .and_then(|pane| pane.terminal.last_command_output());

        if let Some(text) = output {
            if self.clipboard.write(&text) {
                self.show_toast("Copied command output");
            }
        } else {
            self.show_toast("No command output found");
        }
    }

    /// Jumps to the previous or next prompt in scrollback.
    fn jump_to_prompt(&mut self, forward: bool) {
        // First gather the data we need with an immutable borrow
        let (prompts, visible_start, current_offset) = {
            let Some(pane) = self.focused_pane_mut() else {
                return;
            };
            let prompts = pane.terminal.prompt_positions();
            let visible_start = pane.terminal.visible_start_row();
            let current_offset = pane.view.scrollback_offset();
            (prompts, visible_start, current_offset)
        };

        if prompts.is_empty() {
            return;
        }

        // Current view is showing rows starting at: visible_start - current_offset
        let current_top = visible_start.saturating_sub(current_offset);

        // Find the target prompt
        let target_prompt = if forward {
            // Find the next prompt after current view
            prompts.iter().find(|&&p| p > current_top).copied()
        } else {
            // Find the previous prompt before current view
            prompts.iter().rev().find(|&&p| p < current_top).copied()
        };

        let Some(target) = target_prompt else {
            return;
        };

        // Calculate the scroll offset to show this prompt at the top
        // offset = visible_start - target
        let new_offset = visible_start.saturating_sub(target);

        // Get mutable reference and update
        if let Some(pane) = self.focused_pane_mut() {
            pane.view.scrollback_scroll_to(new_offset);
        }
        self.request_redraw();
    }

    /// Moves focus in the given direction within the active tab.
    fn focus_move(&mut self, dir: SplitDir, positive: bool) {
        let (buffer_width, buffer_height) = self.last_buffer_size;
        if buffer_width == 0 || buffer_height == 0 {
            return;
        }

        let tab_bar_height = self.tab_bar_height();
        let pane_height = (buffer_height as usize).saturating_sub(tab_bar_height);
        let overlap_weight = self.config.interaction.focus_move_overlap_weight;

        let (moved, tab_id, pane_id) = {
            let Some(tab) = self.active_tab_mut() else {
                return;
            };
            let (rects, _) = tab.pane_rects(buffer_width as usize, pane_height);
            let moved = tab.focus_move(dir, positive, &rects, overlap_weight);
            (moved, tab.id, tab.focused_pane)
        };

        if moved {
            self.refresh_active_tab_title_from_focused_pane();
            self.update_cursor();
            self.request_redraw();
            let direction = match (dir, positive) {
                (SplitDir::Vertical, false) => "left",
                (SplitDir::Vertical, true) => "right",
                (SplitDir::Horizontal, false) => "up",
                (SplitDir::Horizontal, true) => "down",
            };
            let cwd = self.cwd_for_event(Some(tab_id), Some(pane_id));
            self.dispatch_plugin_event(PluginEvent {
                event: "pane_focus_changed".to_string(),
                action: None,
                source: None,
                tab_id: Some(tab_id),
                pane_id: Some(pane_id),
                data: Some(json!({ "direction": direction, "cwd": cwd })),
            });
        }
    }

    /// Resizes the focused pane in the given direction.
    fn resize_focused(&mut self, dir: SplitDir, negative: bool) {
        let step = self.config.interaction.pane_resize_step;
        if let Some(tab) = self.active_tab_mut() {
            if tab.resize_focused(dir, negative, step) {
                self.layout_dirty = true;
                self.request_redraw();
            }
        }
    }

    /// Closes the focused pane in the active tab.
    fn close_focused_pane(&mut self) {
        let tab_id = self.active_tab().map(|t| t.id);
        let previous_focus = self.active_tab().map(|t| t.focused_pane);
        let should_close_tab = self
            .active_tab_mut()
            .map(|t| t.close_focused_pane())
            .unwrap_or(false);

        if should_close_tab {
            self.close_active_tab();
        }

        self.layout_dirty = true;
        self.refresh_active_tab_title_from_focused_pane();
        self.update_cursor();
        self.request_redraw();

        if let (Some(tab_id), Some(pane_id)) = (tab_id, previous_focus) {
            let cwd = self.cwd_for_event(Some(tab_id), Some(pane_id));
            self.dispatch_plugin_event(PluginEvent {
                event: "pane_closed".to_string(),
                action: None,
                source: None,
                tab_id: Some(tab_id),
                pane_id: Some(pane_id),
                data: Some(json!({ "cwd": cwd })),
            });
        }

        let current_focus = self.active_tab().map(|t| t.focused_pane);
        let current_tab_id = self.active_tab().map(|t| t.id);
        if current_focus != previous_focus {
            if let (Some(tab_id), Some(pane_id)) = (current_tab_id, current_focus) {
                let cwd = self.cwd_for_event(Some(tab_id), Some(pane_id));
                self.dispatch_plugin_event(PluginEvent {
                    event: "pane_focus_changed".to_string(),
                    action: None,
                    source: None,
                    tab_id: Some(tab_id),
                    pane_id: Some(pane_id),
                    data: Some(json!({ "reason": "close", "cwd": cwd })),
                });
            }
        }
    }

    /// Splits the focused pane in the given direction.
    fn split_pane(&mut self, dir: SplitDir) {
        let scale = self.config.font.scale;
        let scrollback = self.config.terminal.scrollback_lines;
        let min_size = self.config.pane.min_size();
        let inherit_cwd = self.config.pane.inherit_cwd_on_split();
        let shadow_default = self
            .config
            .shell_integration
            .shadow_prompt_enabled_by_default;
        let proxy = self.event_proxy.clone();
        let cwd = if inherit_cwd {
            self.active_pane_cwd_path()
        } else {
            None
        };
        let parent_shell_cwd = if inherit_cwd {
            self.active_tab()
                .and_then(|t| t.panes.get(&t.focused_pane))
                .and_then(|p| p.shell_cwd.clone())
        } else {
            None
        };

        // Get the current focused pane's rect
        let focused_rect = self.focused_pane_rect();

        let (new_pane, tab_id) = {
            let Some(tab) = self.active_tab_mut() else {
                return;
            };
            let new_pane = tab.split_focused(
                dir,
                scale,
                scrollback,
                proxy.as_ref(),
                focused_rect,
                min_size,
                shadow_default,
                cwd.as_deref(),
            );
            if inherit_cwd {
                if let (Some(new_id), Some(cwd_url)) =
                    (new_pane, parent_shell_cwd.as_deref())
                {
                    if let Some(pane) = tab.panes.get_mut(&new_id) {
                        pane.shell_cwd = Some(cwd_url.to_string());
                    }
                }
            }
            (new_pane, tab.id)
        };

        if let Some(new_pane) = new_pane {
            self.layout_dirty = true;
            self.refresh_active_tab_title_from_focused_pane();
            self.request_redraw();
            let direction = match dir {
                SplitDir::Vertical => "vertical",
                SplitDir::Horizontal => "horizontal",
            };
            let cwd = self.cwd_for_event(Some(tab_id), Some(new_pane));
            self.dispatch_plugin_event(PluginEvent {
                event: "pane_split".to_string(),
                action: None,
                source: None,
                tab_id: Some(tab_id),
                pane_id: Some(new_pane),
                data: Some(json!({ "direction": direction, "cwd": cwd })),
            });
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
        let (scale_min, scale_max) = self.config.font_scale_clamp();

        let Some(pane) = self.focused_pane_mut() else {
            return;
        };

        // Use fixed 0.25 increments
        let new_scale = if delta > 0 {
            (pane.scale + 0.25).clamp(scale_min, scale_max)
        } else {
            (pane.scale - 0.25).clamp(scale_min, scale_max)
        };
        if (new_scale - pane.scale).abs() < 0.01 {
            return;
        }

        pane.scale = new_scale;
        self.layout_dirty = true;
        self.update_cursor();
        self.request_redraw();
    }

    /// Resets the focused pane's zoom to the default scale.
    fn zoom_reset_focused(&mut self) {
        let (scale_min, scale_max) = self.config.font_scale_clamp();
        let new_scale = self.config.font.scale.clamp(scale_min, scale_max);

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
