//! Search functionality for the terminal scrollback buffer.
//!
//! Provides search within terminal history with match highlighting.

use crate::renderer::scrollback::RowSnapshot;

/// A match location in the scrollback buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchMatch {
    /// Absolute row index in the combined buffer (history + live).
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
    /// All matches found in the entire scrollback + live buffer.
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

    /// Appends a character to the query.
    pub fn push_char(&mut self, ch: char) {
        self.query.push(ch);
    }

    /// Removes the last character from the query.
    pub fn pop_char(&mut self) {
        self.query.pop();
    }

    /// Toggles case sensitivity.
    pub fn toggle_case_sensitive(&mut self) {
        self.case_sensitive = !self.case_sensitive;
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

    /// Returns the absolute row of the current match (for scrolling to it).
    pub fn current_match_row(&self) -> Option<usize> {
        self.current_match().map(|m| m.row)
    }

    /// Updates the matches based on current query and ALL rows (history + live).
    /// Call this when the query changes or when significant content changes.
    pub fn update_matches(&mut self, all_rows: &[RowSnapshot]) {
        self.matches.clear();

        if self.query.is_empty() {
            return;
        }

        let query = if self.case_sensitive {
            self.query.clone()
        } else {
            self.query.to_lowercase()
        };

        for (row_idx, row) in all_rows.iter().enumerate() {
            self.find_matches_in_row(row, row_idx, &query);
        }

        // Clamp current_match to valid range
        if !self.matches.is_empty() && self.current_match >= self.matches.len() {
            self.current_match = self.matches.len() - 1;
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
    /// `display_row` is the row index in the current display (0 = top of visible area).
    /// `view_start` is the absolute index of the first visible row.
    /// Returns Some(true) if it's the current match, Some(false) if it's another match, None if not a match.
    pub fn is_match(&self, display_row: usize, col: usize, view_start: usize) -> Option<bool> {
        let absolute_row = view_start + display_row;
        for (i, m) in self.matches.iter().enumerate() {
            if m.row == absolute_row && col >= m.start_col && col < m.end_col {
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
        state.push_char('h');
        state.push_char('e');
        state.push_char('l');
        state.push_char('l');
        state.push_char('o');

        let display_rows = vec![
            make_row("hello world"),
            make_row("hello again"),
            make_row("goodbye"),
        ];

        state.update_matches(&display_rows);

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
        state.push_char('h');
        state.push_char('e');
        state.push_char('l');
        state.push_char('l');
        state.push_char('o');

        let display_rows = vec![make_row("Hello World"), make_row("HELLO WORLD")];

        state.update_matches(&display_rows);

        assert_eq!(state.match_count(), 2);
    }

    #[test]
    fn test_search_case_sensitive() {
        let mut state = SearchState::new();
        state.activate();
        state.toggle_case_sensitive();
        state.push_char('h');
        state.push_char('e');
        state.push_char('l');
        state.push_char('l');
        state.push_char('o');

        let display_rows = vec![
            make_row("Hello World"),
            make_row("HELLO WORLD"),
            make_row("hello world"),
        ];

        state.update_matches(&display_rows);

        assert_eq!(state.match_count(), 1);
        assert_eq!(state.matches()[0].row, 2);
    }

    #[test]
    fn test_search_navigation() {
        let mut state = SearchState::new();
        state.activate();
        state.push_char('t');
        state.push_char('e');
        state.push_char('s');
        state.push_char('t');

        let display_rows = vec![make_row("test one test two test three")];

        state.update_matches(&display_rows);

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
        state.push_char('w');
        state.push_char('o');
        state.push_char('r');
        state.push_char('l');
        state.push_char('d');

        let display_rows = vec![make_row("hello world")];

        state.update_matches(&display_rows);

        // "world" starts at col 6
        // view_start=0 means display row 0 = absolute row 0
        assert!(state.is_match(0, 5, 0).is_none()); // 'o' before world
        assert_eq!(state.is_match(0, 6, 0), Some(true)); // 'w' - current match
        assert_eq!(state.is_match(0, 10, 0), Some(true)); // 'd' - still in match
        assert!(state.is_match(0, 11, 0).is_none()); // Past match
    }

    #[test]
    fn test_search_multiple_matches_same_line() {
        let mut state = SearchState::new();
        state.activate();
        state.push_char('a');
        state.push_char('b');
        state.push_char('c');

        let display_rows = vec![make_row("abcabcabc")];

        state.update_matches(&display_rows);

        assert_eq!(state.match_count(), 3);
        assert_eq!(state.matches()[0].start_col, 0);
        assert_eq!(state.matches()[1].start_col, 3);
        assert_eq!(state.matches()[2].start_col, 6);
    }

    #[test]
    fn test_search_push_pop_char() {
        let mut state = SearchState::new();
        state.activate();

        state.push_char('t');
        state.push_char('e');
        state.push_char('s');

        assert_eq!(state.query(), "tes");

        state.pop_char();
        assert_eq!(state.query(), "te");
    }

    #[test]
    fn test_search_with_history() {
        // Simulate searching through history + live rows
        let mut state = SearchState::new();
        state.activate();
        state.push_char('t');
        state.push_char('e');
        state.push_char('s');
        state.push_char('t');

        // Simulate: 2 history rows + 2 live rows
        let all_rows = vec![
            make_row("history test 1"), // row 0 (history)
            make_row("history row 2"),  // row 1 (history)
            make_row("live test 2"),    // row 2 (live)
            make_row("live row"),       // row 3 (live)
        ];

        state.update_matches(&all_rows);

        // Should find "test" in rows 0 and 2
        assert_eq!(state.match_count(), 2);
        assert_eq!(state.matches()[0].row, 0); // history row
        assert_eq!(state.matches()[1].row, 2); // live row

        // Test is_match with view_start
        // If view shows rows 1-3 (view_start=1), display row 0 = absolute row 1
        assert!(state.is_match(0, 8, 1).is_none()); // display row 0 = abs row 1, no match
        assert_eq!(state.is_match(1, 5, 1), Some(false)); // display row 1 = abs row 2, has "test"

        // Navigate to next match
        state.next_match();
        assert_eq!(state.current_match_index(), 1);
        assert_eq!(state.current_match_row(), Some(2));
    }

    #[test]
    fn test_search_current_match_row() {
        let mut state = SearchState::new();
        state.activate();
        state.push_char('a');

        let all_rows = vec![
            make_row("row a"), // row 0
            make_row("row b"), // row 1
            make_row("row a"), // row 2
        ];

        state.update_matches(&all_rows);
        assert_eq!(state.current_match_row(), Some(0));

        state.next_match();
        assert_eq!(state.current_match_row(), Some(2));
    }
}
