use serde_json::json;

use super::super::plugins::PluginEvent;
use super::super::*;
use crate::app::layout::SplitDir;

impl App {
    /// Moves focus in the given direction within the active tab.
    pub(super) fn focus_move(&mut self, dir: SplitDir, positive: bool) {
        let (buffer_width, buffer_height) = self.last_buffer_size;
        if buffer_width == 0 || buffer_height == 0 {
            return;
        }

        let tab_bar_height = self.tab_bar_height();
        let pane_height = (buffer_height as usize).saturating_sub(tab_bar_height);
        let overlap_weight = self.config.interaction.focus_move_overlap_weight;

        let (moved, tab_id, pane_id) = {
            let Some(tab) = self.active_tab_mut() else {
                return;
            };
            let (rects, _) = tab.pane_rects(buffer_width as usize, pane_height);
            let moved = tab.focus_move(dir, positive, &rects, overlap_weight);
            (moved, tab.id, tab.focused_pane)
        };

        if moved {
            self.refresh_active_tab_title_from_focused_pane();
            self.update_cursor();
            self.request_redraw();
            let direction = match (dir, positive) {
                (SplitDir::Vertical, false) => "left",
                (SplitDir::Vertical, true) => "right",
                (SplitDir::Horizontal, false) => "up",
                (SplitDir::Horizontal, true) => "down",
            };
            let cwd = self.cwd_for_event(Some(tab_id), Some(pane_id));
            let profile = self.profile_for_event(Some(tab_id), Some(pane_id));
            self.dispatch_plugin_event(PluginEvent {
                event: "pane_focus_changed".to_string(),
                action: None,
                source: None,
                tab_id: Some(tab_id),
                pane_id: Some(pane_id),
                data: Some(json!({ "direction": direction, "cwd": cwd, "profile": profile })),
            });
        }
    }

    /// Resizes the focused pane in the given direction.
    pub(super) fn resize_focused(&mut self, dir: SplitDir, negative: bool) {
        let step = self.config.interaction.pane_resize_step;
        if let Some(tab) = self.active_tab_mut() {
            if tab.resize_focused(dir, negative, step) {
                self.layout_dirty = true;
                self.request_redraw();
            }
        }
    }

    /// Closes the focused pane in the active tab.
    pub(super) fn close_focused_pane(&mut self) {
        let tab_id = self.active_tab().map(|t| t.id);
        let previous_focus = self.active_tab().map(|t| t.focused_pane);
        let should_close_tab = self
            .active_tab_mut()
            .map(|t| t.close_focused_pane())
            .unwrap_or(false);

        if should_close_tab {
            self.close_active_tab();
        }

        self.layout_dirty = true;
        self.refresh_active_tab_title_from_focused_pane();
        self.update_cursor();
        self.request_redraw();

        if let (Some(tab_id), Some(pane_id)) = (tab_id, previous_focus) {
            let cwd = self.cwd_for_event(Some(tab_id), Some(pane_id));
            let profile = self.profile_for_event(Some(tab_id), Some(pane_id));
            self.dispatch_plugin_event(PluginEvent {
                event: "pane_closed".to_string(),
                action: None,
                source: None,
                tab_id: Some(tab_id),
                pane_id: Some(pane_id),
                data: Some(json!({ "cwd": cwd, "profile": profile })),
            });
        }

        let current_focus = self.active_tab().map(|t| t.focused_pane);
        let current_tab_id = self.active_tab().map(|t| t.id);
        if current_focus != previous_focus {
            if let (Some(tab_id), Some(pane_id)) = (current_tab_id, current_focus) {
                let cwd = self.cwd_for_event(Some(tab_id), Some(pane_id));
                let profile = self.profile_for_event(Some(tab_id), Some(pane_id));
                self.dispatch_plugin_event(PluginEvent {
                    event: "pane_focus_changed".to_string(),
                    action: None,
                    source: None,
                    tab_id: Some(tab_id),
                    pane_id: Some(pane_id),
                    data: Some(json!({ "reason": "close", "cwd": cwd, "profile": profile })),
                });
            }
        }
    }

    /// Splits the focused pane in the given direction.
    pub(super) fn split_pane(&mut self, dir: SplitDir) {
        let scale = self.config.font.scale;
        let scrollback = self.config.terminal.scrollback_lines;
        let min_size = self.config.pane.min_size();
        let inherit_cwd = self.config.pane.inherit_cwd_on_split();
        let shadow_default = self
            .config
            .shell_integration
            .shadow_prompt_enabled_by_default;
        let proxy = self.event_proxy.clone();
        let cwd = if inherit_cwd {
            self.active_pane_cwd_path()
        } else {
            None
        };
        let parent_shell_cwd = if inherit_cwd {
            self.active_tab()
                .and_then(|t| t.panes.get(&t.focused_pane))
                .and_then(|p| p.shell_cwd.clone())
        } else {
            None
        };

        // Get the current focused pane's rect
        let focused_rect = self.focused_pane_rect();

        let (new_pane, tab_id) = {
            let Some(tab) = self.active_tab_mut() else {
                return;
            };
            let new_pane = tab.split_focused(
                dir,
                scale,
                scrollback,
                proxy.as_ref(),
                focused_rect,
                min_size,
                shadow_default,
                cwd.as_deref(),
            );
            if inherit_cwd {
                if let (Some(new_id), Some(cwd_url)) =
                    (new_pane, parent_shell_cwd.as_deref())
                {
                    if let Some(pane) = tab.panes.get_mut(&new_id) {
                        pane.shell_cwd = Some(cwd_url.to_string());
                    }
                }
            }
            (new_pane, tab.id)
        };

        if let Some(new_pane) = new_pane {
            self.layout_dirty = true;
            self.refresh_active_tab_title_from_focused_pane();
            self.request_redraw();
            let direction = match dir {
                SplitDir::Vertical => "vertical",
                SplitDir::Horizontal => "horizontal",
            };
            let cwd = self.cwd_for_event(Some(tab_id), Some(new_pane));
            let profile = self.profile_for_event(Some(tab_id), Some(new_pane));
            self.dispatch_plugin_event(PluginEvent {
                event: "pane_split".to_string(),
                action: None,
                source: None,
                tab_id: Some(tab_id),
                pane_id: Some(new_pane),
                data: Some(json!({ "direction": direction, "cwd": cwd, "profile": profile })),
            });
        }
    }

    /// Returns the rectangle of the currently focused pane, if any.
    pub(super) fn focused_pane_rect(&self) -> Option<crate::app::layout::Rect> {
        let Some(graphics) = &self.graphics else {
            return None;
        };
        let size = graphics.surface.window().inner_size();
        let tab_bar_height = self.tab_bar_height();
        let pane_height = (size.height as usize).saturating_sub(tab_bar_height);

        let tab = self.active_tab()?;
        let (rects, _) = tab.pane_rects(size.width as usize, pane_height);
        rects
            .into_iter()
            .find(|(id, _)| *id == tab.focused_pane)
            .map(|(_, rect)| rect)
    }

    /// Zooms the focused pane by the given delta.
    pub(super) fn zoom_focused(&mut self, delta: isize) {
        let (scale_min, scale_max) = self.config.font_scale_clamp();

        let Some(pane) = self.focused_pane_mut() else {
            return;
        };

        // Use fixed 0.25 increments
        let new_scale = if delta > 0 {
            (pane.scale + 0.25).clamp(scale_min, scale_max)
        } else {
            (pane.scale - 0.25).clamp(scale_min, scale_max)
        };
        if (new_scale - pane.scale).abs() < 0.01 {
            return;
        }

        pane.scale = new_scale;
        self.layout_dirty = true;
        self.update_cursor();
        self.request_redraw();
    }

    /// Resets the focused pane's zoom to the default scale.
    pub(super) fn zoom_reset_focused(&mut self) {
        let (scale_min, scale_max) = self.config.font_scale_clamp();
        let new_scale = self.config.font.scale.clamp(scale_min, scale_max);

        let Some(pane) = self.focused_pane_mut() else {
            return;
        };

        if pane.scale == new_scale {
            return;
        }

        pane.scale = new_scale;
        self.layout_dirty = true;
        self.update_cursor();
        self.request_redraw();
    }
}
