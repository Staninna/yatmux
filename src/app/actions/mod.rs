//! Action execution for the terminal application.

mod pane;
mod scroll;
mod search;
mod tab;

use yatmux::config::Action;

use super::plugins::{ActionSource, PluginEvent};
use serde_json::json;

use crate::app::layout::SplitDir;
use crate::app::App;

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

            Action::ScrollPageUp => self.scroll_page(24),
            Action::ScrollPageDown => self.scroll_page(-24),
            Action::ScrollLineUp => self.scroll_page(1),
            Action::ScrollLineDown => self.scroll_page(-1),
            Action::ScrollToTop => self.scroll_to_top(),
            Action::ScrollToBottom => self.scroll_to_bottom(),
            Action::ClearScrollback => self.clear_scrollback(),
            Action::Reset => self.reset_terminal(),

            Action::SearchFind => self.open_search(),

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

            // Profile actions
            Action::CycleProfile => self.cycle_profile(false),
            Action::CycleProfileReverse => self.cycle_profile(true),
            Action::SwitchToProfile1 => self.switch_to_profile_by_index(1),
            Action::SwitchToProfile2 => self.switch_to_profile_by_index(2),
            Action::SwitchToProfile3 => self.switch_to_profile_by_index(3),
            Action::SwitchToProfile4 => self.switch_to_profile_by_index(4),
            Action::SwitchToProfile5 => self.switch_to_profile_by_index(5),
            Action::SwitchToProfile6 => self.switch_to_profile_by_index(6),
            Action::SwitchToProfile7 => self.switch_to_profile_by_index(7),
            Action::SwitchToProfile8 => self.switch_to_profile_by_index(8),
            Action::SwitchToProfile9 => self.switch_to_profile_by_index(9),
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

    /// Cycles to the next or previous profile.
    fn cycle_profile(&mut self, reverse: bool) {
        let profile_names = self.config.profiles.profile_names();
        if profile_names.len() <= 1 {
            self.show_toast("Only one profile available");
            return;
        }

        let (new_profile, tab_id, pane_id) = {
            let Some(pane) = self.focused_pane_mut() else {
                return;
            };

            let current_idx = profile_names
                .iter()
                .position(|n| n == &pane.profile)
                .unwrap_or(0);

            let next_idx = if reverse {
                if current_idx == 0 {
                    profile_names.len() - 1
                } else {
                    current_idx - 1
                }
            } else {
                (current_idx + 1) % profile_names.len()
            };

            let new_profile = profile_names[next_idx].clone();
            pane.profile = new_profile.clone();

            let tab_id = self.active_tab().map(|t| t.id);
            let pane_id = self.active_tab().map(|t| t.focused_pane);
            (new_profile, tab_id, pane_id)
        };

        self.show_toast(&format!("Profile: {}", new_profile));
        self.request_redraw();

        // Dispatch profile_changed event
        if let (Some(tab_id), Some(pane_id)) = (tab_id, pane_id) {
            let cwd = self.cwd_for_event(Some(tab_id), Some(pane_id));
            self.dispatch_plugin_event(PluginEvent {
                event: "profile_changed".to_string(),
                action: None,
                source: None,
                tab_id: Some(tab_id),
                pane_id: Some(pane_id),
                data: Some(json!({ "profile": new_profile, "cwd": cwd })),
            });
        }
    }

    /// Switches to a profile by index (1-based).
    fn switch_to_profile_by_index(&mut self, index: usize) {
        let profile_names = self.config.profiles.profile_names();
        let idx = index - 1; // 1-based to 0-based

        if idx >= profile_names.len() {
            self.show_toast(&format!("Profile {} does not exist", index));
            return;
        }

        let (new_profile, tab_id, pane_id) = {
            let Some(pane) = self.focused_pane_mut() else {
                return;
            };
            let new_profile = profile_names[idx].clone();
            pane.profile = new_profile.clone();

            let tab_id = self.active_tab().map(|t| t.id);
            let pane_id = self.active_tab().map(|t| t.focused_pane);
            (new_profile, tab_id, pane_id)
        };

        self.show_toast(&format!("Profile: {}", new_profile));
        self.request_redraw();

        // Dispatch profile_changed event
        if let (Some(tab_id), Some(pane_id)) = (tab_id, pane_id) {
            let cwd = self.cwd_for_event(Some(tab_id), Some(pane_id));
            self.dispatch_plugin_event(PluginEvent {
                event: "profile_changed".to_string(),
                action: None,
                source: None,
                tab_id: Some(tab_id),
                pane_id: Some(pane_id),
                data: Some(json!({ "profile": new_profile, "cwd": cwd })),
            });
        }
    }
}
