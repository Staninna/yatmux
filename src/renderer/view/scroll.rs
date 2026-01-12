use super::TerminalView;

impl TerminalView {
    pub fn scrollback_scroll_by(&mut self, delta_lines: isize) {
        let max_offset = self.max_scroll_offset();
        let new_offset = (self.scroll_offset as isize + delta_lines).clamp(0, max_offset as isize);
        self.scroll_offset = new_offset as usize;
    }

    pub fn scrollback_scroll_to(&mut self, offset: usize) {
        let max_offset = self.max_scroll_offset();
        self.scroll_offset = offset.min(max_offset);
    }

    pub fn scrollback_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn scrollback_snap_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn is_scrolled_up(&self) -> bool {
        self.scroll_offset > 0
    }

    pub fn clear_scrollback(&mut self) {
        self.scroll_offset = 0;
    }
}
