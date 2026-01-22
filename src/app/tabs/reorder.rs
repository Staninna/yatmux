use super::super::*;

impl App {
    /// Reorders a tab from one position to another
    pub fn reorder_tab(&mut self, from: usize, to: usize) {
        if from == to || from >= self.tabs.len() || to > self.tabs.len() {
            return;
        }

        let tab = self.tabs.remove(from);

        // Adjust insertion index: after remove, indices shift
        let insert_pos = if to > from { to - 1 } else { to };
        self.tabs.insert(insert_pos, tab);

        // Update active_tab index to track the correct tab after reorder
        if self.active_tab == from {
            // The dragged tab was active - follow it to new position
            self.active_tab = insert_pos;
        } else if from < self.active_tab && insert_pos >= self.active_tab {
            // Moved tab from before active to after active
            self.active_tab -= 1;
        } else if from > self.active_tab && insert_pos <= self.active_tab {
            // Moved tab from after active to before active
            self.active_tab += 1;
        }

        self.refresh_active_tab_title_from_focused_pane();
    }

    pub fn move_active_tab_left(&mut self) {
        if self.active_tab == 0 {
            return;
        }
        let from = self.active_tab;
        self.reorder_tab(from, from - 1);
        self.layout_dirty = true;
        self.request_redraw();
    }

    pub fn move_active_tab_right(&mut self) {
        if self.active_tab + 1 >= self.tabs.len() {
            return;
        }
        let from = self.active_tab;
        self.reorder_tab(from, from + 2);
        self.layout_dirty = true;
        self.request_redraw();
    }

    pub fn move_active_tab_to_start(&mut self) {
        if self.active_tab == 0 {
            return;
        }
        let from = self.active_tab;
        self.reorder_tab(from, 0);
        self.layout_dirty = true;
        self.request_redraw();
    }

    pub fn move_active_tab_to_end(&mut self) {
        if self.active_tab + 1 >= self.tabs.len() {
            return;
        }
        let from = self.active_tab;
        self.reorder_tab(from, self.tabs.len());
        self.layout_dirty = true;
        self.request_redraw();
    }

    pub fn close_other_tabs(&mut self) {
        if self.tabs.len() <= 1 {
            return;
        }
        let active = self.active_tab;
        let active_tab = self.tabs.remove(active);
        self.tabs.clear();
        self.tabs.push(active_tab);
        self.active_tab = 0;
        self.layout_dirty = true;
        self.refresh_active_tab_title_from_focused_pane();
        self.request_redraw();
    }

    pub fn close_tabs_to_right(&mut self) {
        if self.active_tab + 1 >= self.tabs.len() {
            return;
        }
        self.tabs.truncate(self.active_tab + 1);
        self.layout_dirty = true;
        self.refresh_active_tab_title_from_focused_pane();
        self.request_redraw();
    }
}
