use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use super::{App, AppEvent};
use super::plugins::PluginEvent;

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.initialize_first_tab();

        if self.graphics.is_none() {
            self.create_window(event_loop);
        }

        self.dispatch_startup_event();
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::PtyOutput { tab, pane, bytes } => {
                let Some(tab_idx) = self.tabs.iter().position(|t| t.id == tab) else {
                    return;
                };

                // Check for prompt marker in raw bytes BEFORE processing
                // OSC 133;A marks prompt start - means command finished
                let has_prompt_marker = bytes.windows(6).any(|w| w == b"]133;A" || w == b"]133;B");

                {
                    let t = &self.tabs[tab_idx];
                    if let Some(p) = t.panes.get(&pane) {
                        p.terminal.process(&bytes);
                    }
                }

                // Update prompt state
                if let Some(pane_state) = self.tabs[tab_idx].panes.get_mut(&pane) {
                    // If we detected a prompt marker, flush shadow prompt
                    if has_prompt_marker {
                        pane_state.command_running = false;
                        let buffered = pane_state.shadow_prompt.take();
                        if !buffered.is_empty() {
                            pane_state.terminal.write(buffered.as_bytes());
                        }
                    }
                }

                self.apply_shell_integration_updates(tab_idx, pane);
                self.request_redraw();
            }
            AppEvent::PtyExited { tab, pane } => {
                // Find the tab index
                if let Some(tab_idx) = self.tabs.iter().position(|t| t.id == tab) {
                    let should_close_tab = {
                        let t = &mut self.tabs[tab_idx];
                        if t.panes.contains_key(&pane) {
                            t.close_pane(pane)
                        } else {
                            false
                        }
                    };

                    if should_close_tab {
                        self.close_tab(tab_idx);
                    }

                    self.layout_dirty = true;
                    self.request_redraw();
                }

                if self.should_exit {
                    event_loop.exit();
                }
            }
            AppEvent::PluginCommands { plugin, commands } => {
                self.handle_plugin_commands(plugin, commands);
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.dispatch_plugin_event(PluginEvent {
                    event: "shutdown".to_string(),
                    action: None,
                    source: None,
                    tab_id: self.active_tab().map(|t| t.id),
                    pane_id: self.active_tab().map(|t| t.focused_pane),
                    data: None,
                });
                event_loop.exit();
            }

            WindowEvent::RedrawRequested => {
                self.render();
                if self.should_exit {
                    event_loop.exit();
                }
            }

            WindowEvent::Resized(_) => {
                self.handle_resize();
                self.request_redraw();
            }

            WindowEvent::ModifiersChanged(mods) => {
                self.input.modifiers = mods.state();
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.handle_cursor_moved(position);
            }

            WindowEvent::MouseInput { state, button, .. } => {
                self.handle_mouse_button(state, button);
            }

            WindowEvent::MouseWheel { delta, .. } => {
                self.handle_scroll(delta);
            }

            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard(&event);
            }

            _ => {}
        }
    }
}
