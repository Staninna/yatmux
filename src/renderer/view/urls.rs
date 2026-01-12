use super::TerminalView;

impl TerminalView {
    pub fn update_url_hover(&mut self, row: usize, col: usize) -> bool {
        let was_hovered = self.urls.hovered_url().is_some();
        self.urls.update_hover(row, col);
        let is_hovered = self.urls.hovered_url().is_some();
        was_hovered != is_hovered || is_hovered
    }

    pub fn clear_url_hover(&mut self) {
        self.urls.clear_hover();
    }

    pub fn url_at(&self, row: usize, col: usize) -> Option<String> {
        self.urls.url_at(row, col).map(|span| span.full_url())
    }

    pub fn has_hovered_url(&self) -> bool {
        self.urls.hovered_url().is_some()
    }
}
