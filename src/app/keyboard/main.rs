use super::bindings::ResolvedKeybind;
use super::super::plugins::ActionSource;
use super::super::*;

impl App {
    pub(crate) fn handle_keyboard(&mut self, event: &winit::event::KeyEvent) {
        if event.state != ElementState::Pressed {
            return;
        }

        if self.prompt.is_some() {
            let update = {
                let prompt = self.prompt.as_mut().expect("prompt checked");
                App::handle_prompt_input(prompt, event, self.input.modifiers)
            };
            if update.needs_redraw {
                self.request_redraw();
            }
            if let Some(resolution) = update.resolution {
                self.finish_prompt(
                    resolution.ok,
                    resolution.value,
                    resolution.index,
                    resolution.reason,
                );
            }
            return;
        }

        let modifiers = self.input.modifiers;
        let ctrl = modifiers.control_key();
        let alt = modifiers.alt_key();

        let key_str = key_event_to_string(event);

        if self.handle_help_overlay_input(key_str.as_deref(), event, modifiers) {
            return;
        }

        let ResolvedKeybind {
            action,
            plugin_binding,
            non_search_action,
        } = self.resolve_keybinding(key_str.as_deref(), modifiers);

        if let Some(plugin_binding) = plugin_binding {
            self.dispatch_plugin_keybind_event(&plugin_binding, ActionSource::User);
            return;
        }

        // Tab and pane actions are always handled, even while searching.
        if let Some(action) = non_search_action {
            if matches!(
                action,
                Action::NewTab
                    | Action::CloseTab
                    | Action::NextTab
                    | Action::PrevTab
                    | Action::Tab1
                    | Action::Tab2
                    | Action::Tab3
                    | Action::Tab4
                    | Action::Tab5
                    | Action::Tab6
                    | Action::Tab7
                    | Action::Tab8
                    | Action::Tab9
                    | Action::SplitVertical
                    | Action::SplitHorizontal
                    | Action::FocusLeft
                    | Action::FocusRight
                    | Action::FocusUp
                    | Action::FocusDown
                    | Action::ResizeLeft
                    | Action::ResizeRight
                    | Action::ResizeUp
                    | Action::ResizeDown
                    | Action::ClosePane
                    | Action::ToggleHelp
                    | Action::CopyLastOutput
                    | Action::JumpToPrevPrompt
                    | Action::JumpToNextPrompt
                    | Action::ToggleShadowPrompt
                    | Action::CycleProfile
                    | Action::CycleProfileReverse
                    | Action::SwitchToProfile1
                    | Action::SwitchToProfile2
                    | Action::SwitchToProfile3
                    | Action::SwitchToProfile4
                    | Action::SwitchToProfile5
                    | Action::SwitchToProfile6
                    | Action::SwitchToProfile7
                    | Action::SwitchToProfile8
                    | Action::SwitchToProfile9
                    | Action::ReloadConfig
            ) {
                self.execute_action(action);
                return;
            }
        }

        let mut needs_redraw = false;
        let mut action_to_execute: Option<Action> = None;
        let shadow_mode = self.config.shell_integration.shadow_prompt;

        {
            let Some(tab) = self.active_tab_mut() else {
                return;
            };
            let Some(pane) = tab.focused_pane_mut() else {
                return;
            };

            if pane.view.is_search_active() {
                let search_action = action.filter(|a| a.is_search_mode_only());
                needs_redraw |= apply_search_input(&mut pane.view, modifiers, action, event);
                if let Some(search_action) = search_action {
                    self.dispatch_action_event(search_action, ActionSource::User);
                }
            } else if let Some(action) = non_search_action {
                action_to_execute = Some(action);
            } else {
                // Check if this is Enter key - only snap to bottom on Enter
                let is_enter = matches!(event.logical_key, Key::Named(NamedKey::Enter));

                // Check if we should use shadow prompt (command is running - use cached state)
                // Never use shadow prompt in alt-screen apps (htop, vim, less).
                let is_command_running = shadow_mode != ShadowPromptMode::Off
                    && pane.shadow_prompt_enabled
                    && pane.command_running
                    && !pane.terminal.is_alt_screen_active();

                if is_command_running {
                    // Route input to shadow prompt instead of terminal.
                    // If a key isn't handled by the shadow prompt (eg. Ctrl+C), forward it to PTY.
                    let handled =
                        Self::handle_shadow_prompt_input(pane, &event.logical_key, modifiers);
                    needs_redraw |= handled;

                    if !handled {
                        if let Some(bytes) = key_to_pty_bytes(&event.logical_key, modifiers) {
                            pane.terminal.write(&bytes);
                            needs_redraw = true;
                        }
                    }
                } else {
                    // Regular terminal input
                    if !ctrl && !alt {
                        if let Some(text) = &event.text {
                            if !text.is_empty() {
                                if is_enter {
                                    pane.view.scrollback_snap_to_bottom();
                                    // Mark command as running when Enter is pressed
                                    pane.command_running = true;
                                }
                                pane.terminal.write(text.as_bytes());
                                needs_redraw = true;
                            }
                        }
                    }

                    if !needs_redraw {
                        if let Some(bytes) = key_to_pty_bytes(&event.logical_key, modifiers) {
                            if is_enter {
                                pane.view.scrollback_snap_to_bottom();
                                // Mark command as running when Enter is pressed
                                pane.command_running = true;
                            }
                            pane.terminal.write(&bytes);
                            needs_redraw = true;
                        }
                    }
                }
            }
        }

        if let Some(action) = action_to_execute {
            self.execute_action(action);
            return;
        }

        if needs_redraw {
            self.request_redraw();
        }
    }
}
