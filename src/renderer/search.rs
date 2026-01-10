//! Search functionality for the terminal scrollback buffer.
//!
//! Provides search within terminal history with match highlighting.

use crate::renderer::scrollback::{RowSnapshot, ScrollbackBuffer};

/// A match location in the scrollback buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchMatch {
    /// Row index in the scrollback buffer.
    pub row: usize,
    /// Starting column of the match.
    pub start_col: usize,
    /// Ending column of the match (exclusive).
    pub end_col: usize,
}

/// Search state for the terminal.
pub struct SearchState {
    /// Whether search mode is active.
    active: bool,
    /// Current search query.
    query: String,
    /// All matches found.
    matches: Vec<SearchMatch>,
    /// Index of the currently selected match.
    current_match: usize,
    /// Whether search is case-sensitive.
    case_sensitive: bool,
}

impl Default for SearchState {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchState {
    /// Creates a new search state.
    pub fn new() -> Self {
        SearchState {
            active: false,
            query: String::new(),
            matches: Vec::new(),
            current_match: 0,
            case_sensitive: false,
        }
    }

    /// Returns whether search mode is active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Activates search mode.
    pub fn activate(&mut self) {
        self.active = true;
        self.query.clear();
        self.matches.clear();
        self.current_match = 0;
    }

    /// Deactivates search mode.
    pub fn deactivate(&mut self) {
        self.active = false;
        self.query.clear();
        self.matches.clear();
        self.current_match = 0;
    }

    /// Returns the current search query.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Sets the search query and updates matches.
    pub fn set_query(
        &mut self,
        query: &str,
        scrollback: &ScrollbackBuffer,
        live_rows: &[RowSnapshot],
    ) {
        self.query = query.to_string();
        self.update_matches(scrollback, live_rows);
    }

    /// Appends a character to the query.
    pub fn push_char(
        &mut self,
        ch: char,
        scrollback: &ScrollbackBuffer,
        live_rows: &[RowSnapshot],
    ) {
        self.query.push(ch);
        self.update_matches(scrollback, live_rows);
    }

    /// Removes the last character from the query.
    pub fn pop_char(&mut self, scrollback: &ScrollbackBuffer, live_rows: &[RowSnapshot]) {
        self.query.pop();
        self.update_matches(scrollback, live_rows);
    }

    /// Toggles case sensitivity.
    pub fn toggle_case_sensitive(
        &mut self,
        scrollback: &ScrollbackBuffer,
        live_rows: &[RowSnapshot],
    ) {
        self.case_sensitive = !self.case_sensitive;
        self.update_matches(scrollback, live_rows);
    }

    /// Returns whether search is case-sensitive.
    pub fn is_case_sensitive(&self) -> bool {
        self.case_sensitive
    }

    /// Returns all matches.
    pub fn matches(&self) -> &[SearchMatch] {
        &self.matches
    }

    /// Returns the current match index.
    pub fn current_match_index(&self) -> usize {
        self.current_match
    }

    /// Returns the total number of matches.
    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    /// Returns the current match, if any.
    pub fn current_match(&self) -> Option<&SearchMatch> {
        self.matches.get(self.current_match)
    }

    /// Moves to the next match.
    pub fn next_match(&mut self) {
        if !self.matches.is_empty() {
            self.current_match = (self.current_match + 1) % self.matches.len();
        }
    }

    /// Moves to the previous match.
    pub fn prev_match(&mut self) {
        if !self.matches.is_empty() {
            self.current_match = if self.current_match == 0 {
                self.matches.len() - 1
            } else {
                self.current_match - 1
            };
        }
    }

    /// Updates the matches based on current query.
    fn update_matches(&mut self, scrollback: &ScrollbackBuffer, live_rows: &[RowSnapshot]) {
        self.matches.clear();
        self.current_match = 0;

        if self.query.is_empty() {
            return;
        }

        let query = if self.case_sensitive {
            self.query.clone()
        } else {
            self.query.to_lowercase()
        };

        // Search in scrollback buffer
        let scrollback_len = scrollback.len();
        for row_idx in 0..scrollback_len {
            if let Some(row) = scrollback.get_row(row_idx) {
                self.find_matches_in_row(&row, row_idx, &query);
            }
        }

        // Search in live rows (after scrollback)
        for (i, row) in live_rows.iter().enumerate() {
            let row_idx = scrollback_len + i;
            self.find_matches_in_row(row, row_idx, &query);
        }
    }

    /// Finds all matches in a single row.
    fn find_matches_in_row(&mut self, row: &RowSnapshot, row_idx: usize, query: &str) {
        let row_text: String = row.cells.iter().map(|(ch, _, _)| *ch).collect();
        let search_text = if self.case_sensitive {
            row_text.clone()
        } else {
            row_text.to_lowercase()
        };

        let mut start = 0;
        while let Some(pos) = search_text[start..].find(query) {
            let match_start = start + pos;
            let match_end = match_start + query.len();

            self.matches.push(SearchMatch {
                row: row_idx,
                start_col: match_start,
                end_col: match_end,
            });

            start = match_start + 1;
        }
    }

    /// Checks if a cell is part of a match.
    /// Returns Some(true) if it's the current match, Some(false) if it's another match, None if not a match.
    pub fn is_match(&self, row: usize, col: usize) -> Option<bool> {
        for (i, m) in self.matches.iter().enumerate() {
            if m.row == row && col >= m.start_col && col < m.end_col {
                return Some(i == self.current_match);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vt100::Color;

    fn make_row(text: &str) -> RowSnapshot {
        let cells: Vec<_> = text
            .chars()
            .map(|ch| (ch, Color::Default, Color::Default))
            .collect();
        let tabs = vec![None; cells.len()];
        RowSnapshot::new(cells, tabs)
    }

    #[test]
    fn test_search_state_new() {
        let state = SearchState::new();
        assert!(!state.is_active());
        assert!(state.query().is_empty());
        assert_eq!(state.match_count(), 0);
    }

    #[test]
    fn test_search_activate_deactivate() {
        let mut state = SearchState::new();

        state.activate();
        assert!(state.is_active());

        state.deactivate();
        assert!(!state.is_active());
    }

    #[test]
    fn test_search_find_matches() {
        let mut state = SearchState::new();
        state.activate();

        let scrollback = ScrollbackBuffer::new();
        let live_rows = vec![
            make_row("hello world"),
            make_row("hello again"),
            make_row("goodbye"),
        ];

        state.set_query("hello", &scrollback, &live_rows);

        assert_eq!(state.match_count(), 2);
        assert_eq!(state.matches()[0].row, 0);
        assert_eq!(state.matches()[0].start_col, 0);
        assert_eq!(state.matches()[0].end_col, 5);
        assert_eq!(state.matches()[1].row, 1);
    }

    #[test]
    fn test_search_case_insensitive() {
        let mut state = SearchState::new();
        state.activate();

        let scrollback = ScrollbackBuffer::new();
        let live_rows = vec![make_row("Hello World"), make_row("HELLO WORLD")];

        state.set_query("hello", &scrollback, &live_rows);

        assert_eq!(state.match_count(), 2);
    }

    #[test]
    fn test_search_case_sensitive() {
        let mut state = SearchState::new();
        state.activate();

        let scrollback = ScrollbackBuffer::new();
        let live_rows = vec![
            make_row("Hello World"),
            make_row("HELLO WORLD"),
            make_row("hello world"),
        ];

        state.toggle_case_sensitive(&scrollback, &live_rows);
        state.set_query("hello", &scrollback, &live_rows);

        assert_eq!(state.match_count(), 1);
        assert_eq!(state.matches()[0].row, 2);
    }

    #[test]
    fn test_search_navigation() {
        let mut state = SearchState::new();
        state.activate();

        let scrollback = ScrollbackBuffer::new();
        let live_rows = vec![make_row("test one test two test three")];

        state.set_query("test", &scrollback, &live_rows);

        assert_eq!(state.match_count(), 3);
        assert_eq!(state.current_match_index(), 0);

        state.next_match();
        assert_eq!(state.current_match_index(), 1);

        state.next_match();
        assert_eq!(state.current_match_index(), 2);

        state.next_match();
        assert_eq!(state.current_match_index(), 0); // Wraps around

        state.prev_match();
        assert_eq!(state.current_match_index(), 2); // Wraps backward
    }

    #[test]
    fn test_search_is_match() {
        let mut state = SearchState::new();
        state.activate();

        let scrollback = ScrollbackBuffer::new();
        let live_rows = vec![make_row("hello world")];

        state.set_query("world", &scrollback, &live_rows);

        // "world" starts at col 6
        assert!(state.is_match(0, 5).is_none()); // 'o' before world
        assert_eq!(state.is_match(0, 6), Some(true)); // 'w' - current match
        assert_eq!(state.is_match(0, 10), Some(true)); // 'd' - still in match
        assert!(state.is_match(0, 11).is_none()); // Past match
    }

    #[test]
    fn test_search_multiple_matches_same_line() {
        let mut state = SearchState::new();
        state.activate();

        let scrollback = ScrollbackBuffer::new();
        let live_rows = vec![make_row("abcabcabc")];

        state.set_query("abc", &scrollback, &live_rows);

        assert_eq!(state.match_count(), 3);
        assert_eq!(state.matches()[0].start_col, 0);
        assert_eq!(state.matches()[1].start_col, 3);
        assert_eq!(state.matches()[2].start_col, 6);
    }

    #[test]
    fn test_search_push_pop_char() {
        let mut state = SearchState::new();
        state.activate();

        let scrollback = ScrollbackBuffer::new();
        let live_rows = vec![make_row("testing")];

        state.push_char('t', &scrollback, &live_rows);
        state.push_char('e', &scrollback, &live_rows);
        state.push_char('s', &scrollback, &live_rows);

        assert_eq!(state.query(), "tes");
        assert_eq!(state.match_count(), 1);

        state.pop_char(&scrollback, &live_rows);
        assert_eq!(state.query(), "te");
        assert_eq!(state.match_count(), 1);
    }
}
