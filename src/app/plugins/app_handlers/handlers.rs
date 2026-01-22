use super::super::command::PluginCommand;
use super::super::events::ActionSource;
use crate::app::App;

impl App {
    pub(crate) fn handle_plugin_commands(&mut self, plugin: String, commands: Vec<PluginCommand>) {
        self.plugin_dispatch_depth = self.plugin_dispatch_depth.saturating_add(1);
        for command in commands {
            match command {
                PluginCommand::Action { action } => {
                    self.execute_action_with_source(action, ActionSource::Plugin);
                }
                PluginCommand::Toast { message } => self.show_toast(message),
                PluginCommand::SetTabTitle { title, tab_id } => {
                    if let Some(target) = tab_id {
                        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == target) {
                            tab.title = title;
                        }
                    } else if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                        tab.title = title;
                    }
                    self.sync_window_title();
                    self.request_redraw();
                }
                PluginCommand::SetWindowTitle { title } => {
                    if let Some(graphics) = &self.graphics {
                        graphics.surface.window().set_title(&title);
                        self.last_window_title = Some(title);
                    }
                }
                PluginCommand::NewTab { cwd, title } => {
                    let cwd_path = std::path::PathBuf::from(cwd);
                    self.new_tab_with_cwd(Some(cwd_path), title);
                }
                PluginCommand::SetTabCwd { cwd, tab_id } => {
                    let tab_id = tab_id.or_else(|| self.active_tab().map(|t| t.id));
                    if let Some(tab_id) = tab_id {
                        self.set_tab_cwd(tab_id, &cwd);
                    }
                }
                PluginCommand::SetPaneCwd {
                    cwd,
                    tab_id,
                    pane_id,
                } => {
                    let tab_id = tab_id.or_else(|| self.active_tab().map(|t| t.id));
                    if let Some(tab_id) = tab_id {
                        self.set_pane_cwd(tab_id, pane_id, &cwd);
                    }
                }
                PluginCommand::SetPaneProfile {
                    profile,
                    tab_id,
                    pane_id,
                } => {
                    let tab_id = tab_id.or_else(|| self.active_tab().map(|t| t.id));
                    if let Some(tab_id) = tab_id {
                        self.set_pane_profile(tab_id, pane_id, &profile);
                    }
                }
                PluginCommand::Prompt {
                    id,
                    title,
                    message,
                    default,
                } => {
                    self.prompt_owners.insert(id.clone(), plugin.clone());
                    self.open_prompt(crate::app::PromptState::input(
                        id,
                        title,
                        message,
                        default,
                    ));
                }
                PluginCommand::Confirm {
                    id,
                    title,
                    message,
                    ok_label,
                    cancel_label,
                } => {
                    self.prompt_owners.insert(id.clone(), plugin.clone());
                    self.open_prompt(crate::app::PromptState::confirm(
                        id,
                        title,
                        message,
                        ok_label,
                        cancel_label,
                    ));
                }
                PluginCommand::Pick {
                    id,
                    title,
                    message,
                    items,
                    selected,
                } => {
                    self.prompt_owners.insert(id.clone(), plugin.clone());
                    self.open_prompt(crate::app::PromptState::pick(
                        id,
                        title,
                        message,
                        items,
                        selected,
                    ));
                }
                PluginCommand::RequestState { id } => {
                    self.state_owners.insert(id.clone(), plugin.clone());
                    self.dispatch_state_response(id);
                }
                PluginCommand::ClipboardRead { id } => {
                    self.clipboard_owners.insert(id.clone(), plugin.clone());
                    let text = self.clipboard.read();
                    self.dispatch_clipboard_response(id, text);
                }
                PluginCommand::ClipboardWrite { text } => {
                    let _ = self.clipboard.write(&text);
                }
                PluginCommand::SendText {
                    text,
                    tab_id,
                    pane_id,
                } => {
                    let tab_id = tab_id.or_else(|| self.active_tab().map(|t| t.id));
                    if let Some(tab_id) = tab_id {
                        self.send_text(tab_id, pane_id, &text);
                    }
                }
                PluginCommand::FocusTab { tab_id } => {
                    self.focus_tab_by_id(tab_id);
                }
                PluginCommand::CloseTab { tab_id } => {
                    self.close_tab_by_id(tab_id);
                }
                PluginCommand::ClosePane { tab_id, pane_id } => {
                    self.close_pane_by_id(tab_id, pane_id);
                }
                PluginCommand::ConfigPatch { toml, persist } => {
                    let patch = match toml.parse::<toml::Value>() {
                        Ok(v) => v,
                        Err(e) => {
                            eprintln!("Plugin config_patch TOML invalid: {e}");
                            continue;
                        }
                    };
                    if let Err(e) = self.config.apply_toml_patch(patch) {
                        eprintln!("Plugin config_patch failed: {e}");
                        continue;
                    }
                    if persist.unwrap_or(false) {
                        if let Err(e) = self.config.save() {
                            eprintln!("Plugin config_patch save failed: {e}");
                        }
                    }
                    self.sync_font_scale_clamp();
                    if let Some(graphics) = &mut self.graphics {
                        graphics.palette = std::sync::Arc::new(
                            yatmux::renderer::create_palette_with_ansi(self.config.colors.palette),
                        );
                    }
                    self.layout_dirty = true;
                    self.request_redraw();
                }
                PluginCommand::ReloadConfig => self.reload_config(),
                PluginCommand::PluginCommand { name, args } => {
                    self.dispatch_plugin_command_event(name, args, ActionSource::Plugin);
                }
                PluginCommand::RegisterKeybind {
                    key,
                    action,
                    persist,
                } => {
                    self.config.keybinds.bindings.insert(key, action);
                    if persist.unwrap_or(false) {
                        if let Err(e) = self.config.save() {
                            eprintln!("Plugin register_keybind save failed: {e}");
                        }
                    }
                }
                PluginCommand::Subscribe { events } => {
                    self.plugins.set_subscription(&plugin, events);
                }
            }
        }
        self.plugin_dispatch_depth = self.plugin_dispatch_depth.saturating_sub(1);
    }
}
