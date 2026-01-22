use yatmux::config::{Action, PluginKeybind};

use super::super::events::{ActionSource, PluginEvent};
use crate::app::App;

impl App {
    pub(crate) fn dispatch_plugin_event(&mut self, event: PluginEvent) {
        self.dispatch_plugin_event_to(None, event);
    }

    pub(crate) fn dispatch_plugin_event_to(&mut self, target: Option<&str>, event: PluginEvent) {
        const MAX_PLUGIN_DISPATCH_DEPTH: usize = 2;
        if self.plugin_dispatch_depth >= MAX_PLUGIN_DISPATCH_DEPTH {
            return;
        }
        self.plugins
            .dispatch(event, self.event_proxy.clone(), target);
    }

    pub(crate) fn dispatch_action_event(&mut self, action: Action, source: ActionSource) {
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

    pub(crate) fn dispatch_plugin_command_event(
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

    pub(crate) fn dispatch_plugin_keybind_event(
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

        self.dispatch_plugin_event_to(
            Some(plugin.plugin.as_str()),
            PluginEvent {
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
            },
        );
    }

    pub(crate) fn dispatch_startup_event(&mut self) {
        if self.plugins_started {
            return;
        }
        self.plugins_started = true;
        let (tab_id, pane_id) = self
            .active_tab()
            .map(|t| (Some(t.id), Some(t.focused_pane)))
            .unwrap_or((None, None));
        self.dispatch_plugin_event_to(
            None,
            PluginEvent {
                event: "startup".to_string(),
                action: None,
                source: None,
                tab_id,
                pane_id,
                data: None,
            },
        );
    }
}

fn action_to_string(action: Action) -> String {
    serde_json::to_value(action)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| format!("{action:?}"))
}
