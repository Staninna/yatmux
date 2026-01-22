use super::super::plugins::PluginEvent;
use super::super::*;
use serde_json::json;

impl App {
    pub fn next_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        let previous = self.active_tab().map(|t| t.id);
        self.active_tab = (self.active_tab + 1) % self.tabs.len();
        self.layout_dirty = true;
        self.refresh_active_tab_title_from_focused_pane();
        self.request_redraw();
        let current = self.active_tab().map(|t| t.id);
        if current != previous {
            let cwd = self.cwd_for_event(current, self.active_tab().map(|t| t.focused_pane));
            self.dispatch_plugin_event(PluginEvent {
                event: "tab_changed".to_string(),
                action: None,
                source: None,
                tab_id: current,
                pane_id: None,
                data: Some(json!({
                    "from_tab_id": previous,
                    "to_tab_id": current,
                    "cwd": cwd,
                })),
            });
        }
    }

    pub fn prev_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        let previous = self.active_tab().map(|t| t.id);
        self.active_tab = if self.active_tab == 0 {
            self.tabs.len() - 1
        } else {
            self.active_tab - 1
        };
        self.layout_dirty = true;
        self.refresh_active_tab_title_from_focused_pane();
        self.request_redraw();
        let current = self.active_tab().map(|t| t.id);
        if current != previous {
            let cwd = self.cwd_for_event(current, self.active_tab().map(|t| t.focused_pane));
            self.dispatch_plugin_event(PluginEvent {
                event: "tab_changed".to_string(),
                action: None,
                source: None,
                tab_id: current,
                pane_id: None,
                data: Some(json!({
                    "from_tab_id": previous,
                    "to_tab_id": current,
                    "cwd": cwd,
                })),
            });
        }
    }

    pub fn goto_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            let previous = self.active_tab().map(|t| t.id);
            self.active_tab = index;
            self.layout_dirty = true;
            self.refresh_active_tab_title_from_focused_pane();
            self.request_redraw();
            let current = self.active_tab().map(|t| t.id);
            if current != previous {
                let cwd = self.cwd_for_event(current, self.active_tab().map(|t| t.focused_pane));
                self.dispatch_plugin_event(PluginEvent {
                    event: "tab_changed".to_string(),
                    action: None,
                    source: None,
                    tab_id: current,
                    pane_id: None,
                    data: Some(json!({
                        "from_tab_id": previous,
                        "to_tab_id": current,
                        "cwd": cwd,
                    })),
                });
            }
        }
    }

    pub fn focus_tab_by_id(&mut self, tab_id: TabId) -> bool {
        let Some(index) = self.tabs.iter().position(|t| t.id == tab_id) else {
            return false;
        };
        let previous = self.active_tab().map(|t| t.id);
        self.active_tab = index;
        self.layout_dirty = true;
        self.refresh_active_tab_title_from_focused_pane();
        self.request_redraw();
        let current = self.active_tab().map(|t| t.id);
        if current != previous {
            let cwd = self.cwd_for_event(current, self.active_tab().map(|t| t.focused_pane));
            self.dispatch_plugin_event(PluginEvent {
                event: "tab_changed".to_string(),
                action: None,
                source: None,
                tab_id: current,
                pane_id: None,
                data: Some(json!({
                    "from_tab_id": previous,
                    "to_tab_id": current,
                    "cwd": cwd,
                })),
            });
        }
        true
    }
}
