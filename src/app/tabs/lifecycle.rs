use std::path::PathBuf;

use super::super::plugins::PluginEvent;
use super::super::*;
use serde_json::json;

impl App {
    pub fn new_tab(&mut self) -> TabId {
        self.new_tab_with_cwd(None, None)
    }

    pub fn new_tab_with_cwd(&mut self, cwd: Option<PathBuf>, title: Option<String>) -> TabId {
        let previous_tab_id = self.active_tab().map(|t| t.id);
        let id = self.next_tab_id;
        self.next_tab_id += 1;

        let mut tab = Tab::new(id);
        if let Some(title) = title {
            tab.title = title;
        }
        tab.spawn_initial_pane(
            self.config.font.scale,
            self.config.terminal.scrollback_lines as usize,
            self.event_proxy.as_ref(),
            self.config
                .shell_integration
                .shadow_prompt_enabled_by_default,
            cwd.as_deref(),
            "default".to_string(),
        );

        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
        self.layout_dirty = true;
        self.refresh_active_tab_title_from_focused_pane();
        self.dispatch_plugin_event(PluginEvent {
            event: "tab_created".to_string(),
            action: None,
            source: None,
            tab_id: Some(id),
            pane_id: None,
            data: None,
        });
        if previous_tab_id != Some(id) {
            let cwd = self.cwd_for_event(Some(id), self.tabs.last().map(|t| t.focused_pane));
            self.dispatch_plugin_event(PluginEvent {
                event: "tab_changed".to_string(),
                action: None,
                source: None,
                tab_id: Some(id),
                pane_id: None,
                data: Some(json!({
                    "from_tab_id": previous_tab_id,
                    "to_tab_id": id,
                    "cwd": cwd,
                })),
            });
        }
        id
    }

    pub fn close_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }

        let closed_id = self.tabs[index].id;
        self.tabs.remove(index);

        if self.tabs.is_empty() {
            self.dispatch_plugin_event(PluginEvent {
                event: "tab_closed".to_string(),
                action: None,
                source: None,
                tab_id: Some(closed_id),
                pane_id: None,
                data: None,
            });
            self.should_exit = true;
            return;
        }

        // Adjust active tab index
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }

        let active_id = self.tabs.get(self.active_tab).map(|t| t.id);
        self.dispatch_plugin_event(PluginEvent {
            event: "tab_closed".to_string(),
            action: None,
            source: None,
            tab_id: Some(closed_id),
            pane_id: None,
            data: None,
        });
        if active_id != Some(closed_id) {
            let cwd = self.cwd_for_event(active_id, self.active_tab().map(|t| t.focused_pane));
            self.dispatch_plugin_event(PluginEvent {
                event: "tab_changed".to_string(),
                action: None,
                source: None,
                tab_id: active_id,
                pane_id: None,
                data: Some(json!({
                    "from_tab_id": closed_id,
                    "to_tab_id": active_id,
                    "cwd": cwd,
                })),
            });
        }

        self.layout_dirty = true;
        self.refresh_active_tab_title_from_focused_pane();
    }

    pub fn close_active_tab(&mut self) {
        self.close_tab(self.active_tab);
    }
}
