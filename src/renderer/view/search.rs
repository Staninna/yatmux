use super::TerminalView;

impl TerminalView {
    pub fn is_search_active(&self) -> bool {
        self.search.is_active()
    }

    pub fn activate_search(&mut self) {
        self.search.activate();
    }

    pub fn deactivate_search(&mut self) {
        self.search.deactivate();
    }

    pub fn search_query(&self) -> &str {
        self.search.query()
    }

    pub fn search_match_count(&self) -> usize {
        self.search.match_count()
    }

    pub fn search_current_index(&self) -> usize {
        self.search.current_match_index()
    }

    pub fn search_push_char(&mut self, ch: char) {
        self.search.push_char(ch);
    }

    pub fn search_pop_char(&mut self) {
        self.search.pop_char();
    }

    pub fn search_next(&mut self) {
        self.search.next_match();
        self.scroll_to_current_match();
    }

    pub fn search_prev(&mut self) {
        self.search.prev_match();
        self.scroll_to_current_match();
    }

    fn scroll_to_current_match(&mut self) {
        if let Some(match_row) = self.search.current_match_row() {
            self.scroll_to_row(match_row);
        }
    }

    pub fn search_toggle_case(&mut self) {
        self.search.toggle_case_sensitive();
    }

    pub fn is_search_case_sensitive(&self) -> bool {
        self.search.is_case_sensitive()
    }

    pub fn search_toggle_regex(&mut self) {
        self.search.toggle_mode();
    }

    pub fn is_search_regex(&self) -> bool {
        self.search.mode() == crate::core::search::SearchMode::Regex
    }

    pub fn is_search_regex_valid(&self) -> bool {
        self.search.is_regex_valid()
    }
}
