use super::*;

impl App {
    pub fn pane_rects(
        &self,
        buffer_width: usize,
        buffer_height: usize,
    ) -> (Vec<(PaneId, Rect)>, Vec<layout::Divider>) {
        // Reserve space for tab bar when there are multiple tabs
        let tab_bar_height = self.tab_bar_height();
        let pane_height = buffer_height.saturating_sub(tab_bar_height);

        if let Some(tab) = self.active_tab() {
            let (mut rects, dividers) = tab.pane_rects(buffer_width, pane_height);
            // Offset all rects by the tab bar height
            for (_, rect) in &mut rects {
                rect.y += tab_bar_height;
            }
            (rects, dividers)
        } else {
            (Vec::new(), Vec::new())
        }
    }

    pub fn tab_bar_height(&self) -> usize {
        if self.tabs.len() > 1 {
            let scale = self.config.font.scale.clamp(1, 8);
            8 * scale + 4 // cell height + padding
        } else {
            0
        }
    }

    pub fn pane_at_position(
        &self,
        rects: &[(PaneId, Rect)],
        pos: PhysicalPosition<f64>,
    ) -> Option<(PaneId, Rect)> {
        rects
            .iter()
            .find(|(_id, r)| r.contains(pos.x, pos.y))
            .copied()
    }

    pub fn new_tab(&mut self) -> TabId {
        let id = self.next_tab_id;
        self.next_tab_id += 1;

        let mut tab = Tab::new(id);
        tab.spawn_initial_pane(
            self.config.font.scale,
            self.config.terminal.scrollback_lines as usize,
            self.event_proxy.as_ref(),
            self.config
                .shell_integration
                .shadow_prompt_enabled_by_default,
        );

        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
        self.layout_dirty = true;
        self.refresh_active_tab_title_from_focused_pane();
        id
    }

    pub fn close_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }

        self.tabs.remove(index);

        if self.tabs.is_empty() {
            self.should_exit = true;
            return;
        }

        // Adjust active tab index
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }

        self.layout_dirty = true;
        self.refresh_active_tab_title_from_focused_pane();
    }

    pub fn close_active_tab(&mut self) {
        self.close_tab(self.active_tab);
    }

    pub fn next_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        self.active_tab = (self.active_tab + 1) % self.tabs.len();
        self.layout_dirty = true;
        self.refresh_active_tab_title_from_focused_pane();
        self.request_redraw();
    }

    pub fn prev_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        self.active_tab = if self.active_tab == 0 {
            self.tabs.len() - 1
        } else {
            self.active_tab - 1
        };
        self.layout_dirty = true;
        self.refresh_active_tab_title_from_focused_pane();
        self.request_redraw();
    }

    pub fn goto_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_tab = index;
            self.layout_dirty = true;
            self.refresh_active_tab_title_from_focused_pane();
            self.request_redraw();
        }
    }

    pub(super) fn initialize_first_tab(&mut self) {
        if !self.tabs.is_empty() {
            return;
        }
        self.new_tab();
    }

    pub(super) fn localize_pos(rect: Rect, pos: PhysicalPosition<f64>) -> PhysicalPosition<f64> {
        PhysicalPosition::new(pos.x - rect.x as f64, pos.y - rect.y as f64)
    }
}
