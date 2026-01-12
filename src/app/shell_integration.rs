use super::*;

impl App {
    pub(super) fn try_click_move_shell_cursor(
        &mut self,
        pane_id: PaneId,
        click_row: usize,
        click_col: usize,
    ) -> bool {
        if !self.config.shell_integration.semantic_zones_from_osc133 {
            return false;
        }

        let Some(tab) = self.active_tab() else {
            return false;
        };
        let Some(pane) = tab.panes.get(&pane_id) else {
            return false;
        };

        // Don't fight running apps/commands.
        if pane.command_running {
            return false;
        }

        let ((cursor_row, cursor_col), _cursor_visible) = pane.terminal.cursor();
        let cursor_row = cursor_row as usize;
        let cursor_col = cursor_col as usize;

        // For now, only support click-to-move on the cursor row.
        if click_row != cursor_row {
            return false;
        }

        let visible_start = pane.terminal.visible_start_row();
        let cursor_phys_y = visible_start + cursor_row;
        let click_phys_y = visible_start + click_row;

        let zones = match pane.terminal.semantic_zones() {
            Ok(z) => z,
            Err(_) => return false,
        };

        let input_zone = zones.iter().rev().find(|z| {
            z.semantic_type == tattoy_wezterm_term::SemanticType::Input
                && cursor_phys_y >= z.start_y as usize
                && cursor_phys_y <= z.end_y as usize
        });
        let Some(zone) = input_zone else {
            return false;
        };

        let in_zone = |phys_y: usize, col: usize| -> bool {
            let start_y = zone.start_y as usize;
            let end_y = zone.end_y as usize;
            if phys_y < start_y || phys_y > end_y {
                return false;
            }
            if start_y == end_y {
                return col >= zone.start_x && col < zone.end_x;
            }
            if phys_y == start_y {
                return col >= zone.start_x;
            }
            if phys_y == end_y {
                return col < zone.end_x;
            }
            true
        };

        if !in_zone(cursor_phys_y, cursor_col) {
            return false;
        }
        if !in_zone(click_phys_y, click_col) {
            return false;
        }

        let delta = click_col as isize - cursor_col as isize;
        if delta == 0 {
            return true;
        }

        let steps = delta
            .unsigned_abs()
            .min(self.config.interaction.click_move_max_steps);
        let seq: &[u8] = if delta > 0 { b"\x1b[C" } else { b"\x1b[D" };

        let mut bytes = Vec::with_capacity(steps * seq.len());
        for _ in 0..steps {
            bytes.extend_from_slice(seq);
        }
        pane.terminal.write(&bytes);
        true
    }

    pub(super) fn apply_shell_integration_updates(&mut self, tab_idx: usize, pane: PaneId) {
        let cfg = &self.config.shell_integration;
        let tab_id = self.tabs[tab_idx].id;
        let focused_pane = self.tabs[tab_idx].focused_pane;
        let is_active_tab = tab_idx == self.active_tab;

        let mut new_title: Option<String> = None;

        {
            let Some(pane_state) = self.tabs[tab_idx].panes.get_mut(&pane) else {
                return;
            };

            if cfg.cwd_from_osc7 {
                pane_state.shell_cwd = pane_state.terminal.shell_cwd();
            }

            // Only fetch semantic zones when debug logging is enabled (expensive operation)
            if cfg.semantic_zones_from_osc133 && cfg.debug_log {
                if let Ok(zones) = pane_state.terminal.semantic_zones() {
                    if !zones.is_empty() {
                        eprintln!("[shell] tab={} pane={} zones:", tab_id, pane);
                        for zone in &zones {
                            eprintln!(
                                "  {:?} rows {}..{}",
                                zone.semantic_type, zone.start_y, zone.end_y
                            );
                        }
                    }
                }
            }

            let status = pane_state.terminal.shell_integration_status();
            if cfg.debug_log && status != pane_state.shell_integration {
                eprintln!(
                    "[shell] tab={} pane={} any={} osc7={} osc133={} title={}",
                    tab_id,
                    pane,
                    status.any(),
                    status.osc7_cwd,
                    status.osc133_semantic,
                    status.osc_title
                );
            }
            pane_state.shell_integration = status;

            // Note: command_running state is now updated in PtyOutput handler
            // by detecting prompt markers in raw bytes (much cheaper than get_semantic_zones)

            if cfg.title_from_osc {
                pane_state.shell_title = pane_state.terminal.shell_title();
            }

            // Only update the tab title based on the focused pane.
            if focused_pane == pane {
                new_title = match cfg.tab_title_source {
                    yatmux::config::TabTitleSource::None => None,
                    yatmux::config::TabTitleSource::Cwd => pane_state
                        .shell_cwd
                        .as_deref()
                        .map(Self::cwd_url_to_tab_title)
                        .or_else(|| pane_state.shell_title.clone()),
                    yatmux::config::TabTitleSource::Title => pane_state.shell_title.clone(),
                };
            }
        }

        if let Some(title) = new_title {
            let title = Self::sanitize_title(&title);
            if !title.is_empty() {
                self.tabs[tab_idx].title = title;
                if is_active_tab {
                    self.sync_window_title();
                }
            }
        }
    }

    pub(super) fn refresh_active_tab_title_from_focused_pane(&mut self) {
        let tab_idx = self.active_tab;
        let cfg = &self.config.shell_integration;

        let Some(tab) = self.tabs.get_mut(tab_idx) else {
            return;
        };
        let focused = tab.focused_pane;
        let Some(pane) = tab.panes.get(&focused) else {
            self.sync_window_title();
            return;
        };

        let new_title = match cfg.tab_title_source {
            yatmux::config::TabTitleSource::None => None,
            yatmux::config::TabTitleSource::Cwd => pane
                .shell_cwd
                .as_deref()
                .map(Self::cwd_url_to_tab_title)
                .or_else(|| pane.shell_title.clone()),
            yatmux::config::TabTitleSource::Title => pane.shell_title.clone(),
        };

        if let Some(title) = new_title {
            let title = Self::sanitize_title(&title);
            if !title.is_empty() {
                tab.title = title;
            }
        }

        self.sync_window_title();
    }

    fn sanitize_title(s: &str) -> String {
        // Remove newlines/control chars; keep it single-line and readable.
        s.chars()
            .filter(|&ch| ch != '\n' && ch != '\r' && !ch.is_control())
            .collect::<String>()
            .trim()
            .to_string()
    }

    fn cwd_url_to_tab_title(cwd_url: &str) -> String {
        // Typical OSC 7 payload is a file:// URL.
        // Prefer showing a friendly basename in the tab bar.
        let mut s = cwd_url.trim();
        if let Some(stripped) = s.strip_prefix("file://") {
            s = stripped;
        }

        // Drop query/fragment if present.
        s = s.split(['?', '#']).next().unwrap_or(s);

        // Normalize multiple slashes (e.g. file:///home -> ///home -> /home).
        while s.starts_with("//") {
            s = &s[1..];
        }

        let trimmed = s.trim_end_matches('/');
        let base = trimmed.rsplit('/').next().unwrap_or(trimmed);
        if base.is_empty() {
            "/".to_string()
        } else {
            base.to_string()
        }
    }

    pub(super) fn sync_window_title(&mut self) {
        if !self
            .config
            .shell_integration
            .window_title_follows_active_tab
        {
            return;
        }
        let Some(graphics) = &self.graphics else {
            return;
        };

        let base = self.config.window.title.trim();
        let tab_title = self
            .tabs
            .get(self.active_tab)
            .map(|t| t.title.trim())
            .filter(|t| !t.is_empty())
            .unwrap_or("yatmux");

        let new_title = if base.is_empty() {
            tab_title.to_string()
        } else {
            format!("{tab_title} — {base}")
        };

        if self.last_window_title.as_deref() == Some(new_title.as_str()) {
            return;
        }

        graphics.surface.window().set_title(&new_title);
        self.last_window_title = Some(new_title);
    }
}
