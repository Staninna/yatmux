use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
#[cfg(not(test))]
use wait_timeout::ChildExt;
use winit::event_loop::EventLoopProxy;

use yatmux::config::{Action, Config, KeybindAction, PluginConfig, PluginKeybind};

use super::{App, AppEvent};
use crate::app::layout::PaneId;
use crate::app::tab::TabId;

#[derive(Debug, Clone)]
pub struct PluginManager {
    plugins: Vec<Plugin>,
    config_path: Option<PathBuf>,
    subscriptions: std::collections::HashMap<String, HashSet<String>>,
}

#[derive(Debug, Clone)]
struct Plugin {
    name: String,
    root: PathBuf,
    script: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum PluginCommand {
    Action { action: Action },
    Toast { message: String },
    SetTabTitle { title: String, tab_id: Option<u64> },
    SetWindowTitle { title: String },
    NewTab { cwd: String, title: Option<String> },
    SetTabCwd { cwd: String, tab_id: Option<u64> },
    SetPaneCwd {
        cwd: String,
        tab_id: Option<u64>,
        pane_id: Option<u64>,
    },
    Prompt {
        id: String,
        title: String,
        message: Option<String>,
        default: Option<String>,
    },
    Confirm {
        id: String,
        title: String,
        message: Option<String>,
        ok_label: Option<String>,
        cancel_label: Option<String>,
    },
    Pick {
        id: String,
        title: String,
        message: Option<String>,
        items: Vec<String>,
        selected: Option<usize>,
    },
    RequestState { id: String },
    ClipboardRead { id: String },
    ClipboardWrite { text: String },
    SendText {
        text: String,
        tab_id: Option<u64>,
        pane_id: Option<u64>,
    },
    FocusTab { tab_id: u64 },
    CloseTab { tab_id: u64 },
    ClosePane { tab_id: u64, pane_id: u64 },
    Subscribe { events: Vec<String> },
    ConfigPatch { toml: String, persist: Option<bool> },
    ReloadConfig,
    PluginCommand { name: String, args: Option<serde_json::Value> },
    RegisterKeybind {
        key: String,
        action: KeybindAction,
        persist: Option<bool>,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum ActionSource {
    User,
    Plugin,
}

#[derive(Debug, Serialize)]
pub struct PluginEvent {
    pub event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pane_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl PluginManager {
    pub fn new(config: &Config) -> Self {
        let plugins = if config.plugins.enabled {
            discover_plugins(config, &config.plugins)
        } else {
            Vec::new()
        };
        let subscriptions = plugins
            .iter()
            .map(|plugin| (plugin.name.clone(), HashSet::new()))
            .collect();
        Self {
            plugins,
            config_path: Config::config_path(),
            subscriptions,
        }
    }

    pub fn reload(&mut self, config: &Config) {
        *self = Self::new(config);
    }

    pub fn dispatch(
        &self,
        event: PluginEvent,
        proxy: Option<EventLoopProxy<AppEvent>>,
        target_plugin: Option<&str>,
    ) {
        let Some(proxy) = proxy else {
            return;
        };
        if self.plugins.is_empty() {
            return;
        }
        let plugins = self.plugins.clone();
        let config_path = self.config_path.clone();
        let subscriptions = self.subscriptions.clone();
        let target_plugin = target_plugin.map(|s| s.to_string());
        thread::spawn(move || {
            let payload = match serde_json::to_string(&event) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Plugin event serialize failed: {e}");
                    return;
                }
            };

            for plugin in plugins {
                if let Some(target) = target_plugin.as_deref() {
                    if plugin.name != target {
                        continue;
                    }
                } else if !should_deliver_event(&subscriptions, &plugin.name, &event.event) {
                    continue;
                }
                if let Some(commands) = run_plugin(&plugin, &payload, config_path.as_deref()) {
                    if !commands.is_empty() {
                        let _ = proxy.send_event(AppEvent::PluginCommands {
                            plugin: plugin.name.clone(),
                            commands,
                        });
                    }
                }
            }
        });
    }

    pub fn set_subscription(&mut self, plugin: &str, events: Vec<String>) {
        let set: HashSet<String> = events
            .into_iter()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        self.subscriptions.insert(plugin.to_string(), set);
    }
}

impl App {
    pub(super) fn dispatch_plugin_event(&mut self, event: PluginEvent) {
        self.dispatch_plugin_event_to(None, event);
    }

    pub(super) fn dispatch_plugin_event_to(&mut self, target: Option<&str>, event: PluginEvent) {
        const MAX_PLUGIN_DISPATCH_DEPTH: usize = 2;
        if self.plugin_dispatch_depth >= MAX_PLUGIN_DISPATCH_DEPTH {
            return;
        }
        self.plugins
            .dispatch(event, self.event_proxy.clone(), target);
    }

    pub(super) fn handle_plugin_commands(&mut self, plugin: String, commands: Vec<PluginCommand>) {
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
                PluginCommand::Prompt {
                    id,
                    title,
                    message,
                    default,
                } => {
                    self.prompt_owners.insert(id.clone(), plugin.clone());
                    self.open_prompt(super::PromptState::input(
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
                    self.open_prompt(super::PromptState::confirm(
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
                    self.open_prompt(super::PromptState::pick(
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
                        graphics.palette = std::sync::Arc::new(yatmux::renderer::create_palette_with_ansi(
                            self.config.colors.palette,
                        ));
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

    pub(super) fn dispatch_action_event(&mut self, action: Action, source: ActionSource) {
        let action_name = action_to_string(action);
        let (tab_id, pane_id) = self
            .active_tab()
            .map(|t| (Some(t.id), Some(t.focused_pane)))
            .unwrap_or((None, None));
        let cwd = self.cwd_for_event(tab_id, pane_id);

        let source_str = match source {
            ActionSource::User => "user",
            ActionSource::Plugin => "plugin",
        };

        self.dispatch_plugin_event(PluginEvent {
            event: "action".to_string(),
            action: Some(action_name),
            source: Some(source_str.to_string()),
            tab_id,
            pane_id,
            data: Some(serde_json::json!({ "cwd": cwd })),
        });
    }

    pub(super) fn dispatch_plugin_command_event(
        &mut self,
        name: String,
        args: Option<serde_json::Value>,
        source: ActionSource,
    ) {
        let (tab_id, pane_id) = self
            .active_tab()
            .map(|t| (Some(t.id), Some(t.focused_pane)))
            .unwrap_or((None, None));
        let cwd = self.cwd_for_event(tab_id, pane_id);
        let source_str = match source {
            ActionSource::User => "user",
            ActionSource::Plugin => "plugin",
        };

        self.dispatch_plugin_event(PluginEvent {
            event: "plugin_command".to_string(),
            action: None,
            source: Some(source_str.to_string()),
            tab_id,
            pane_id,
            data: Some(serde_json::json!({
                "name": name,
                "args": args,
                "cwd": cwd,
            })),
        });
    }

    pub(super) fn dispatch_plugin_keybind_event(
        &mut self,
        plugin: &PluginKeybind,
        source: ActionSource,
    ) {
        let (tab_id, pane_id) = self
            .active_tab()
            .map(|t| (Some(t.id), Some(t.focused_pane)))
            .unwrap_or((None, None));
        let cwd = self.cwd_for_event(tab_id, pane_id);
        let args = plugin
            .args
            .as_ref()
            .and_then(|args| serde_json::to_value(args).ok());
        let source_str = match source {
            ActionSource::User => "user",
            ActionSource::Plugin => "plugin",
        };

        self.dispatch_plugin_event_to(Some(plugin.plugin.as_str()), PluginEvent {
            event: "plugin_command".to_string(),
            action: None,
            source: Some(source_str.to_string()),
            tab_id,
            pane_id,
            data: Some(serde_json::json!({
                "plugin": plugin.plugin,
                "command": plugin.command,
                "args": args,
                "cwd": cwd,
            })),
        });
    }

    pub(super) fn dispatch_startup_event(&mut self) {
        if self.plugins_started {
            return;
        }
        self.plugins_started = true;
        let (tab_id, pane_id) = self
            .active_tab()
            .map(|t| (Some(t.id), Some(t.focused_pane)))
            .unwrap_or((None, None));
        self.dispatch_plugin_event_to(None, PluginEvent {
            event: "startup".to_string(),
            action: None,
            source: None,
            tab_id,
            pane_id,
            data: None,
        });
    }

    pub(super) fn cwd_for_event(
        &self,
        tab_id: Option<TabId>,
        pane_id: Option<PaneId>,
    ) -> Option<String> {
        let tab_id = tab_id?;
        let pane_id = pane_id?;
        let tab = self.tabs.iter().find(|t| t.id == tab_id)?;
        let pane = tab.panes.get(&pane_id)?;
        let shell_cwd = pane
            .shell_cwd
            .as_deref()
            .map(|s| s.to_string())
            .or_else(|| pane.terminal.shell_cwd());
        shell_cwd
            .as_deref()
            .and_then(cwd_url_to_path)
            .map(|p| p.to_string_lossy().to_string())
    }

    pub(super) fn active_pane_cwd_path(&self) -> Option<std::path::PathBuf> {
        let tab = self.active_tab()?;
        let pane = tab.panes.get(&tab.focused_pane)?;
        let shell_cwd = pane
            .shell_cwd
            .as_deref()
            .map(|s| s.to_string())
            .or_else(|| pane.terminal.shell_cwd());
        shell_cwd.as_deref().and_then(cwd_url_to_path)
    }

    pub(super) fn set_tab_cwd(&mut self, tab_id: TabId, cwd: &str) {
        let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) else {
            return;
        };
        let command = format!("cd -- '{}'\n", escape_shell_single_quotes(cwd));
        for pane in tab.panes.values() {
            pane.terminal.write(command.as_bytes());
        }
    }

    pub(super) fn set_pane_cwd(&mut self, tab_id: TabId, pane_id: Option<u64>, cwd: &str) {
        let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) else {
            return;
        };
        let target = pane_id
            .map(|id| id as PaneId)
            .unwrap_or(tab.focused_pane);
        if let Some(pane) = tab.panes.get(&target) {
            let command = format!("cd -- '{}'\n", escape_shell_single_quotes(cwd));
            pane.terminal.write(command.as_bytes());
        }
    }

    pub(super) fn send_text(&mut self, tab_id: TabId, pane_id: Option<u64>, text: &str) {
        let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) else {
            return;
        };
        let target = pane_id
            .map(|id| id as PaneId)
            .unwrap_or(tab.focused_pane);
        if let Some(pane) = tab.panes.get(&target) {
            pane.terminal.write(text.as_bytes());
        }
    }

    pub(super) fn dispatch_state_response(&mut self, id: String) {
        let mut tabs = Vec::new();
        for tab in &self.tabs {
            let pane_ids: Vec<u64> = tab.panes.keys().copied().collect();
            let mut pane_cwds = serde_json::Map::new();
            for pane_id in tab.panes.keys() {
                let cwd = self.cwd_for_event(Some(tab.id), Some(*pane_id));
                let value = cwd.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null);
                pane_cwds.insert(pane_id.to_string(), value);
            }
            let cwd = self.cwd_for_event(Some(tab.id), Some(tab.focused_pane));
            tabs.push(serde_json::json!({
                "id": tab.id,
                "title": tab.title,
                "focused_pane": tab.focused_pane,
                "panes": pane_ids,
                "pane_cwds": pane_cwds,
                "cwd": cwd,
            }));
        }

        let target = self.state_owners.remove(&id);
        self.dispatch_plugin_event_to(target.as_deref(), PluginEvent {
            event: "state_response".to_string(),
            action: None,
            source: None,
            tab_id: self.active_tab().map(|t| t.id),
            pane_id: self.active_tab().map(|t| t.focused_pane),
            data: Some(serde_json::json!({
                "id": id,
                "active_tab": self.active_tab().map(|t| t.id),
                "tabs": tabs,
            })),
        });
    }

    pub(super) fn dispatch_clipboard_response(&mut self, id: String, text: Option<String>) {
        let target = self.clipboard_owners.remove(&id);
        self.dispatch_plugin_event_to(target.as_deref(), PluginEvent {
            event: "clipboard_response".to_string(),
            action: None,
            source: None,
            tab_id: self.active_tab().map(|t| t.id),
            pane_id: self.active_tab().map(|t| t.focused_pane),
            data: Some(serde_json::json!({
                "id": id,
                "text": text,
            })),
        });
    }

    pub(super) fn dispatch_prompt_response(&mut self, data: serde_json::Value) {
        let target = data
            .get("id")
            .and_then(|v| v.as_str())
            .and_then(|id| self.prompt_owners.remove(id));
        self.dispatch_plugin_event_to(target.as_deref(), PluginEvent {
            event: "prompt_response".to_string(),
            action: None,
            source: None,
            tab_id: self.active_tab().map(|t| t.id),
            pane_id: self.active_tab().map(|t| t.focused_pane),
            data: Some(data),
        });
    }
}

fn action_to_string(action: Action) -> String {
    serde_json::to_value(action)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| format!("{action:?}"))
}

/// Timeout for plugin execution in seconds
const PLUGIN_TIMEOUT_SECS: u64 = 30;

fn plugin_timeout_secs() -> u64 {
    #[cfg(test)]
    {
        if let Ok(raw) = std::env::var("YATMUX_TEST_PLUGIN_TIMEOUT_SECS") {
            if let Ok(value) = raw.parse::<u64>() {
                return value;
            }
        }
    }
    PLUGIN_TIMEOUT_SECS
}

/// Sanitize plugin name for use in environment variables
/// Only allows alphanumeric characters, underscores, and hyphens
fn sanitize_plugin_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect()
}

/// Validate that plugin root is an absolute path and exists
fn validate_plugin_root(root: &Path) -> bool {
    root.is_absolute() && root.exists()
}

fn run_plugin(
    plugin: &Plugin,
    payload: &str,
    config_path: Option<&Path>,
) -> Option<Vec<PluginCommand>> {
    // Validate plugin root path
    if !validate_plugin_root(&plugin.root) {
        eprintln!(
            "Plugin {} has invalid root path: {}",
            plugin.name,
            plugin.root.display()
        );
        return None;
    }

    let mut child = match Command::new("bash")
        .arg(&plugin.script)
        .env("YATMUX_PLUGIN_EVENT", &payload)
        .env("YATMUX_PLUGIN_NAME", &sanitize_plugin_name(&plugin.name))
        .env("YATMUX_PLUGIN_ROOT", &plugin.root)
        .env(
            "YATMUX_CONFIG_PATH",
            config_path.unwrap_or_else(|| Path::new("")).as_os_str(),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            eprintln!(
                "Failed to start plugin {} ({}): {e}",
                plugin.name,
                plugin.script.display()
            );
            return None;
        }
    };

    // Take stdout before waiting
    let stdout_handle = child.stdout.take();

    if let Some(mut stdin) = child.stdin.take() {
        if let Err(e) = stdin.write_all(payload.as_bytes()) {
            eprintln!(
                "Failed to write plugin event to {}: {e}",
                plugin.script.display()
            );
        }
    }

    // Wait for plugin with timeout
    let timeout = Duration::from_secs(plugin_timeout_secs());
    let status = match wait_for_child(&mut child, timeout) {
        Ok(Some(status)) => status,
        Ok(None) => {
            // Timeout occurred
            eprintln!(
                "Plugin {} timed out after {}s, killing process",
                plugin.name, PLUGIN_TIMEOUT_SECS
            );
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        Err(e) => {
            eprintln!("Failed to wait for plugin {}: {e}", plugin.name);
            return None;
        }
    };

    // Read output after process has finished
    let stdout = if let Some(mut handle) = stdout_handle {
        let mut output = Vec::new();
        if let Err(e) = std::io::Read::read_to_end(&mut handle, &mut output) {
            eprintln!("Failed to read plugin output from {}: {e}", plugin.name);
            return None;
        }
        String::from_utf8_lossy(&output).to_string()
    } else {
        String::new()
    };

    if !status.success() {
        eprintln!(
            "Plugin {} exited with status {}",
            plugin.name, status
        );
    }

    Some(parse_plugin_commands(&stdout))
}

fn wait_for_child(child: &mut std::process::Child, timeout: Duration) -> std::io::Result<Option<std::process::ExitStatus>> {
    #[cfg(test)]
    {
        let start = std::time::Instant::now();
        loop {
            match child.try_wait()? {
                Some(status) => return Ok(Some(status)),
                None => {
                    if start.elapsed() >= timeout {
                        return Ok(None);
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        }
    }
    #[cfg(not(test))]
    {
        child.wait_timeout(timeout)
    }
}

fn parse_plugin_commands(stdout: &str) -> Vec<PluginCommand> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    if trimmed.starts_with('[') {
        match serde_json::from_str::<Vec<PluginCommand>>(trimmed) {
            Ok(commands) => return commands,
            Err(e) => {
                eprintln!("Plugin output JSON array invalid: {e}");
                return Vec::new();
            }
        }
    }

    let mut commands = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<PluginCommand>(line) {
            Ok(command) => commands.push(command),
            Err(e) => eprintln!("Plugin output JSON invalid: {e}"),
        }
    }
    commands
}

fn cwd_url_to_path(cwd_url: &str) -> Option<std::path::PathBuf> {
    let mut s = cwd_url.trim();
    if let Some(stripped) = s.strip_prefix("file://") {
        s = stripped;
    }
    if !s.starts_with('/') {
        if let Some(idx) = s.find('/') {
            s = &s[idx..];
        }
    }
    s = s.split(['?', '#']).next().unwrap_or(s);
    while s.starts_with("//") {
        s = &s[1..];
    }
    if !s.starts_with('/') {
        return None;
    }
    if s.is_empty() {
        return None;
    }
    Some(std::path::PathBuf::from(s))
}

fn escape_shell_single_quotes(input: &str) -> String {
    input.replace('\'', "'\\''")
}

fn should_deliver_event(
    subscriptions: &std::collections::HashMap<String, HashSet<String>>,
    plugin: &str,
    event: &str,
) -> bool {
    if matches!(event, "startup" | "shutdown") {
        return true;
    }
    let Some(set) = subscriptions.get(plugin) else {
        return false;
    };
    if set.is_empty() {
        return false;
    }
    set.contains("all") || set.contains(&event.to_lowercase())
}

fn discover_plugins(_config: &Config, plugin_cfg: &PluginConfig) -> Vec<Plugin> {
    let base_dir = Config::config_path()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    let mut paths = Vec::new();
    if plugin_cfg.enable_default_dir {
        if let Some(dir) = dirs::config_dir() {
            paths.push(dir.join("yatmux").join("plugins"));
        }
    }
    for path in &plugin_cfg.paths {
        paths.push(resolve_path(&base_dir, path));
    }

    let mut seen = HashSet::new();
    let mut plugins = Vec::new();

    for path in paths {
        if let Some(list) = discover_from_path(&path) {
            for (root, script) in list {
                if !seen.insert(script.clone()) {
                    continue;
                }
                let name = root
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("plugin")
                    .to_string();
                plugins.push(Plugin { name, root, script });
            }
        }
    }

    plugins
}

fn discover_from_path(path: &Path) -> Option<Vec<(PathBuf, PathBuf)>> {
    if !path.exists() {
        return None;
    }
    if path.is_file() {
        if path.file_name().and_then(|s| s.to_str()) == Some("plugin.sh") {
            let root = path.parent().unwrap_or(path).to_path_buf();
            return Some(vec![(root, path.to_path_buf())]);
        }
        return None;
    }

    if path.is_dir() {
        let script = path.join("plugin.sh");
        if script.exists() {
            return Some(vec![(path.to_path_buf(), script)]);
        }

        let mut out = Vec::new();
        let entries = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(e) => {
                eprintln!("Failed to read plugins dir {}: {e}", path.display());
                return None;
            }
        };

        let mut dirs = Vec::new();
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_dir() {
                    dirs.push(entry.path());
                }
            }
        }
        dirs.sort();

        for dir in dirs {
            let script = dir.join("plugin.sh");
            if script.exists() {
                out.push((dir, script));
            }
        }
        return Some(out);
    }

    None
}

fn resolve_path(base_dir: &Path, input: &str) -> PathBuf {
    let s = input.trim();
    if s == "~" || s.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            if s == "~" {
                return home;
            }
            return home.join(&s[2..]);
        }
    }

    let p = PathBuf::from(s);
    if p.is_absolute() {
        p
    } else {
        base_dir.join(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let mut path = std::env::temp_dir();
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            path.push(format!("yatmux-test-{nanos}-{}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[cfg(unix)]
    fn write_script(dir: &Path, name: &str, contents: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(name);
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(contents.as_bytes()).unwrap();
        let mut perms = file.metadata().unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).unwrap();
        path
    }

    #[test]
    fn parse_plugin_commands_handles_lines_and_array() {
        let stdout = "{\"command\":\"toast\",\"message\":\"hi\"}\n{\"command\":\"reload_config\"}\n";
        let commands = parse_plugin_commands(stdout);
        assert_eq!(commands.len(), 2);

        let stdout = r#"[{"command":"toast","message":"hi"}]"#;
        let commands = parse_plugin_commands(stdout);
        assert_eq!(commands.len(), 1);
    }

    #[test]
    fn parse_plugin_commands_ignores_invalid() {
        let commands = parse_plugin_commands("not-json");
        assert!(commands.is_empty());
    }

    #[test]
    fn parse_plugin_commands_skips_invalid_lines() {
        let stdout = "{\"command\":\"toast\",\"message\":\"ok\"}\nnot-json\n{\"command\":\"reload_config\"}";
        let commands = parse_plugin_commands(stdout);
        assert_eq!(commands.len(), 2);
    }

    #[test]
    fn parse_plugin_commands_invalid_array_returns_empty() {
        let stdout = r#"[{"command":"toast","message":"ok"}, {"command":"nope"}]"#;
        let commands = parse_plugin_commands(stdout);
        assert!(commands.is_empty());
    }

    #[test]
    fn sanitize_plugin_name_strips_invalid_chars() {
        let name = "my plugin!@#_name-1";
        let sanitized = sanitize_plugin_name(name);
        assert_eq!(sanitized, "myplugin_name-1");
    }

    #[test]
    fn validate_plugin_root_requires_absolute_and_exists() {
        let temp = TempDir::new();
        let absolute = temp.path.join("plugin");
        fs::create_dir_all(&absolute).unwrap();
        assert!(validate_plugin_root(&absolute));

        let relative = PathBuf::from("relative-plugin");
        assert!(!validate_plugin_root(&relative));

        let missing = temp.path.join("missing");
        assert!(!validate_plugin_root(&missing));
    }

    proptest! {
        #[test]
        fn sanitize_plugin_name_filters_disallowed(input in ".*") {
            let sanitized = sanitize_plugin_name(&input);
            prop_assert!(sanitized.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-'));
            prop_assert!(sanitized.len() <= input.len());
        }
    }

    proptest! {
        #[test]
        fn escape_shell_single_quotes_matches_replace(input in ".*") {
            let escaped = escape_shell_single_quotes(&input);
            prop_assert_eq!(escaped, input.replace('\'', "'\\''"));
        }
    }

    #[test]
    fn subscriptions_filter_events() {
        let mut subscriptions = std::collections::HashMap::new();
        let mut set = HashSet::new();
        set.insert("action".to_string());
        subscriptions.insert("alpha".to_string(), set);

        assert!(should_deliver_event(&subscriptions, "alpha", "ACTION"));
        assert!(!should_deliver_event(&subscriptions, "alpha", "plugin_command"));
        assert!(should_deliver_event(&subscriptions, "beta", "startup"));
        assert!(should_deliver_event(&subscriptions, "beta", "shutdown"));
    }

    #[test]
    fn resolve_path_expands_relative_and_home() {
        let temp = TempDir::new();
        let base = temp.path.join("base");
        fs::create_dir_all(&base).unwrap();
        let old_home = std::env::var("HOME").ok();

        // SAFETY: This test modifies environment variables which is unsafe in multi-threaded
        // contexts. However, this is acceptable in test code as long as tests are run serially
        // or don't depend on the HOME variable. We restore the original value afterward.
        unsafe {
            std::env::set_var("HOME", &temp.path);
        }

        let rel = resolve_path(&base, "plugins");
        assert_eq!(rel, base.join("plugins"));

        let home = resolve_path(&base, "~/plug");
        assert_eq!(home, temp.path.join("plug"));

        if let Some(old_home) = old_home {
            // SAFETY: Restoring original HOME value. See above comment.
            unsafe {
                std::env::set_var("HOME", old_home);
            }
        }
    }

    #[test]
    fn cwd_url_to_path_strips_host() {
        let path = cwd_url_to_path("file://example.com/tmp/testing").unwrap();
        assert_eq!(path, PathBuf::from("/tmp/testing"));
        let path = cwd_url_to_path("file:///tmp/testing").unwrap();
        assert_eq!(path, PathBuf::from("/tmp/testing"));
    }

    #[test]
    fn discover_from_path_finds_plugin_sh() {
        let temp = TempDir::new();
        let plugin_dir = temp.path.join("example");
        fs::create_dir_all(&plugin_dir).unwrap();
        let script = plugin_dir.join("plugin.sh");
        fs::write(&script, "#!/usr/bin/env bash\n").unwrap();

        let found = discover_from_path(&plugin_dir).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, plugin_dir);
        assert_eq!(found[0].1, script);
    }

    #[cfg(unix)]
    #[test]
    fn run_plugin_executes_script_and_parses() {
        let temp = TempDir::new();
        let plugin_dir = temp.path.join("plugin");
        fs::create_dir_all(&plugin_dir).unwrap();
        let script = write_script(
            &plugin_dir,
            "plugin.sh",
            r#"#!/usr/bin/env bash
set -euo pipefail
echo '{"command":"toast","message":"ok"}'
"#,
        );

        let plugin = Plugin {
            name: "test".to_string(),
            root: plugin_dir.clone(),
            script,
        };
        let commands = run_plugin(&plugin, r#"{"event":"startup"}"#, None).unwrap();
        assert_eq!(commands.len(), 1);
        match &commands[0] {
            PluginCommand::Toast { message } => assert_eq!(message, "ok"),
            _ => panic!("unexpected command"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn run_plugin_writes_stdin_and_sets_config_path() {
        let temp = TempDir::new();
        let plugin_dir = temp.path.join("plugin");
        fs::create_dir_all(&plugin_dir).unwrap();
        let script = write_script(
            &plugin_dir,
            "plugin.sh",
            r#"#!/usr/bin/env bash
set -euo pipefail
cat > "$YATMUX_PLUGIN_ROOT/payload.json"
printf '%s' "$YATMUX_CONFIG_PATH" > "$YATMUX_PLUGIN_ROOT/config_path.txt"
echo '{"command":"toast","message":"ok"}'
"#,
        );

        let plugin = Plugin {
            name: "test".to_string(),
            root: plugin_dir.clone(),
            script,
        };
        let config_path = temp.path.join("config.toml");
        let payload = r#"{"event":"startup"}"#;
        let commands = run_plugin(&plugin, payload, Some(&config_path)).unwrap();
        assert_eq!(commands.len(), 1);
        let seen_payload = fs::read_to_string(plugin_dir.join("payload.json")).unwrap();
        assert_eq!(seen_payload, payload);
        let seen_config = fs::read_to_string(plugin_dir.join("config_path.txt")).unwrap();
        assert_eq!(seen_config, config_path.to_string_lossy());
    }

    #[cfg(unix)]
    #[test]
    fn run_plugin_parses_output_even_on_failure_status() {
        let temp = TempDir::new();
        let plugin_dir = temp.path.join("plugin");
        fs::create_dir_all(&plugin_dir).unwrap();
        let script = write_script(
            &plugin_dir,
            "plugin.sh",
            r#"#!/usr/bin/env bash
set -euo pipefail
echo '{"command":"toast","message":"ok"}'
exit 1
"#,
        );

        let plugin = Plugin {
            name: "test".to_string(),
            root: plugin_dir.clone(),
            script,
        };
        let commands = run_plugin(&plugin, r#"{"event":"startup"}"#, None).unwrap();
        assert_eq!(commands.len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn run_plugin_times_out_with_test_override() {
        let temp = TempDir::new();
        let plugin_dir = temp.path.join("plugin");
        fs::create_dir_all(&plugin_dir).unwrap();
        let script = write_script(
            &plugin_dir,
            "plugin.sh",
            r#"#!/usr/bin/env bash
set -euo pipefail
sleep 2
echo '{"command":"toast","message":"late"}'
"#,
        );

        let plugin = Plugin {
            name: "test".to_string(),
            root: plugin_dir.clone(),
            script,
        };

        let old_timeout = std::env::var("YATMUX_TEST_PLUGIN_TIMEOUT_SECS").ok();
        unsafe {
            std::env::set_var("YATMUX_TEST_PLUGIN_TIMEOUT_SECS", "1");
        }
        let start = std::time::Instant::now();
        let commands = run_plugin(&plugin, r#"{"event":"startup"}"#, None);
        let elapsed = start.elapsed();
        if let Some(old_timeout) = old_timeout {
            unsafe {
                std::env::set_var("YATMUX_TEST_PLUGIN_TIMEOUT_SECS", old_timeout);
            }
        } else {
            unsafe {
                std::env::remove_var("YATMUX_TEST_PLUGIN_TIMEOUT_SECS");
            }
        }

        assert!(commands.is_none());
        assert!(elapsed < std::time::Duration::from_secs(3));
    }
}
