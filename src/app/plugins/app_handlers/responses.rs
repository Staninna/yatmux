use super::super::events::PluginEvent;
use crate::app::App;

impl App {
    pub(crate) fn dispatch_state_response(&mut self, id: String) {
        let mut tabs = Vec::new();
        for tab in &self.tabs {
            let pane_ids: Vec<u64> = tab.panes.keys().copied().collect();
            let mut pane_cwds = serde_json::Map::new();
            let mut pane_profiles = serde_json::Map::new();
            for pane_id in tab.panes.keys() {
                let cwd = self.cwd_for_event(Some(tab.id), Some(*pane_id));
                let value = cwd
                    .map(serde_json::Value::String)
                    .unwrap_or(serde_json::Value::Null);
                pane_cwds.insert(pane_id.to_string(), value);

                let profile = self.profile_for_event(Some(tab.id), Some(*pane_id));
                let profile_value = profile
                    .map(serde_json::Value::String)
                    .unwrap_or(serde_json::Value::Null);
                pane_profiles.insert(pane_id.to_string(), profile_value);
            }
            let cwd = self.cwd_for_event(Some(tab.id), Some(tab.focused_pane));
            let profile = self.profile_for_event(Some(tab.id), Some(tab.focused_pane));
            tabs.push(serde_json::json!({
                "id": tab.id,
                "title": tab.title,
                "focused_pane": tab.focused_pane,
                "panes": pane_ids,
                "pane_cwds": pane_cwds,
                "pane_profiles": pane_profiles,
                "cwd": cwd,
                "profile": profile,
            }));
        }

        let target = self.state_owners.remove(&id);
        self.dispatch_plugin_event_to(
            target.as_deref(),
            PluginEvent {
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
            },
        );
    }

    pub(crate) fn dispatch_clipboard_response(&mut self, id: String, text: Option<String>) {
        let target = self.clipboard_owners.remove(&id);
        self.dispatch_plugin_event_to(
            target.as_deref(),
            PluginEvent {
                event: "clipboard_response".to_string(),
                action: None,
                source: None,
                tab_id: self.active_tab().map(|t| t.id),
                pane_id: self.active_tab().map(|t| t.focused_pane),
                data: Some(serde_json::json!({
                    "id": id,
                    "text": text,
                })),
            },
        );
    }

    pub(crate) fn dispatch_prompt_response(&mut self, data: serde_json::Value) {
        let target = data
            .get("id")
            .and_then(|v| v.as_str())
            .and_then(|id| self.prompt_owners.remove(id));
        self.dispatch_plugin_event_to(
            target.as_deref(),
            PluginEvent {
                event: "prompt_response".to_string(),
                action: None,
                source: None,
                tab_id: self.active_tab().map(|t| t.id),
                pane_id: self.active_tab().map(|t| t.focused_pane),
                data: Some(data),
            },
        );
    }
}
