use super::super::events::PluginEvent;
use crate::app::layout::PaneId;
use crate::app::tab::TabId;
use crate::app::App;

impl App {
    pub(crate) fn cwd_for_event(
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
            .and_then(super::super::utils::cwd_url_to_path)
            .map(|p| p.to_string_lossy().to_string())
    }

    pub(crate) fn profile_for_event(
        &self,
        tab_id: Option<TabId>,
        pane_id: Option<PaneId>,
    ) -> Option<String> {
        let tab_id = tab_id?;
        let pane_id = pane_id?;
        let tab = self.tabs.iter().find(|t| t.id == tab_id)?;
        let pane = tab.panes.get(&pane_id)?;
        Some(pane.profile.clone())
    }

    pub(crate) fn active_pane_cwd_path(&self) -> Option<std::path::PathBuf> {
        let tab = self.active_tab()?;
        let pane = tab.panes.get(&tab.focused_pane)?;
        let shell_cwd = pane
            .shell_cwd
            .as_deref()
            .map(|s| s.to_string())
            .or_else(|| pane.terminal.shell_cwd());
        shell_cwd.as_deref().and_then(super::super::utils::cwd_url_to_path)
    }

    pub(crate) fn set_tab_cwd(&mut self, tab_id: TabId, cwd: &str) {
        let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) else {
            return;
        };
        let command = format!(
            "cd -- '{}'\n",
            super::super::utils::escape_shell_single_quotes(cwd)
        );
        for pane in tab.panes.values() {
            pane.terminal.write(command.as_bytes());
        }
    }

    pub(crate) fn set_pane_cwd(&mut self, tab_id: TabId, pane_id: Option<u64>, cwd: &str) {
        let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) else {
            return;
        };
        let target = pane_id
            .map(|id| id as PaneId)
            .unwrap_or(tab.focused_pane);
        if let Some(pane) = tab.panes.get(&target) {
            let command = format!(
                "cd -- '{}'\n",
                super::super::utils::escape_shell_single_quotes(cwd)
            );
            pane.terminal.write(command.as_bytes());
        }
    }

    pub(crate) fn set_pane_profile(&mut self, tab_id: TabId, pane_id: Option<u64>, profile: &str) {
        let (old_profile, new_profile, target_tab_id, target_pane_id) = {
            let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) else {
                return;
            };
            let target = pane_id
                .map(|id| id as PaneId)
                .unwrap_or(tab.focused_pane);

            if let Some(pane) = tab.panes.get_mut(&target) {
                let old_profile = pane.profile.clone();
                pane.profile = profile.to_string();
                let new_profile = pane.profile.clone();
                (Some(old_profile), new_profile, tab.id, target)
            } else {
                return;
            }
        };

        if let Some(old) = old_profile {
            if old != new_profile {
                let cwd = self.cwd_for_event(Some(target_tab_id), Some(target_pane_id));
                self.dispatch_plugin_event(PluginEvent {
                    event: "profile_changed".to_string(),
                    action: None,
                    source: None,
                    tab_id: Some(target_tab_id),
                    pane_id: Some(target_pane_id),
                    data: Some(serde_json::json!({
                        "profile": new_profile,
                        "old_profile": old,
                        "cwd": cwd
                    })),
                });
            }
        }

        self.request_redraw();
    }

    pub(crate) fn send_text(&mut self, tab_id: TabId, pane_id: Option<u64>, text: &str) {
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
}
