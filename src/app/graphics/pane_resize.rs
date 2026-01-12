use crate::app::App;

impl App {
    /// Resizes panes if the buffer size or layout has changed.
    pub fn resize_panes_if_needed(&mut self, buffer_width: u32, buffer_height: u32) {
        if (buffer_width, buffer_height) == self.last_buffer_size && !self.layout_dirty {
            return;
        }

        let tab_bar_height = self.tab_bar_height();
        let pane_height = (buffer_height as usize).saturating_sub(tab_bar_height);

        // Get padding config
        let padding_left = self.config.pane.padding_left();
        let padding_right = self.config.pane.padding_right();
        let padding_top = self.config.pane.padding_top();
        let padding_bottom = self.config.pane.padding_bottom();

        let Some(tab) = self.active_tab() else {
            return;
        };

        let (rects, _) = tab.pane_rects(buffer_width as usize, pane_height);

        // We need to iterate over the tab's panes, so we re-borrow
        let Some(tab) = self.active_tab_mut() else {
            return;
        };

        for (id, rect) in rects {
            if let Some(pane) = tab.panes.get(&id) {
                let (cell_w, cell_h) = Self::cell_size_for_scale(pane.scale);
                // Calculate content dimensions (after padding)
                let content_w = rect.w.saturating_sub(padding_left + padding_right) as u32;
                let content_h = rect.h.saturating_sub(padding_top + padding_bottom) as u32;
                pane.terminal.resize(content_w, content_h, cell_w, cell_h);
            }
        }

        self.last_buffer_size = (buffer_width, buffer_height);
        self.layout_dirty = false;
    }
}
