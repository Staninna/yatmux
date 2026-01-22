use regex::Regex;

use crate::core::grid::RowSnapshot;

/// Search mode type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchMode {
    /// Plain text search (default).
    #[default]
    Plain,
    /// Regular expression search.
    Regex,
}

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
    pub(super) active: bool,
    /// Current search query.
    pub(super) query: String,
    /// All matches found in the entire scrollback + live buffer.
    pub(super) matches: Vec<SearchMatch>,
    /// Index of the currently selected match.
    pub(super) current_match: usize,
    /// Whether search is case-sensitive.
    pub(super) case_sensitive: bool,
    /// Search mode (plain text or regex).
    pub(super) mode: SearchMode,
    /// Compiled regex (cached when in regex mode).
    pub(super) compiled_regex: Option<Regex>,
    /// Whether the current regex is valid.
    pub(super) regex_valid: bool,
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
            mode: SearchMode::Plain,
            compiled_regex: None,
            regex_valid: true,
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
        self.compiled_regex = None;
        self.regex_valid = true;
    }

    /// Deactivates search mode.
    pub fn deactivate(&mut self) {
        self.active = false;
        self.query.clear();
        self.matches.clear();
        self.current_match = 0;
        self.compiled_regex = None;
        self.regex_valid = true;
    }

    /// Returns the current search query.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Appends a character to the query.
    pub fn push_char(&mut self, ch: char) {
        self.query.push(ch);
        self.update_compiled_regex();
    }

    /// Removes the last character from the query.
    pub fn pop_char(&mut self) {
        self.query.pop();
        self.update_compiled_regex();
    }

    /// Toggles case sensitivity.
    pub fn toggle_case_sensitive(&mut self) {
        self.case_sensitive = !self.case_sensitive;
        self.update_compiled_regex();
    }

    /// Returns whether search is case-sensitive.
    pub fn is_case_sensitive(&self) -> bool {
        self.case_sensitive
    }

    /// Returns the current search mode.
    pub fn mode(&self) -> SearchMode {
        self.mode
    }

    /// Toggles between plain text and regex search mode.
    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            SearchMode::Plain => SearchMode::Regex,
            SearchMode::Regex => SearchMode::Plain,
        };
        self.update_compiled_regex();
    }

    /// Sets the search mode.
    pub fn set_mode(&mut self, mode: SearchMode) {
        if self.mode != mode {
            self.mode = mode;
            self.update_compiled_regex();
        }
    }

    /// Returns whether the current regex pattern is valid.
    pub fn is_regex_valid(&self) -> bool {
        self.regex_valid
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

    /// Updates the matches based on current query and ALL rows (history + live).
    /// Call this when the query changes or when significant content changes.
    pub fn update_matches(&mut self, all_rows: &[RowSnapshot]) {
        self.matches.clear();

        if self.query.is_empty() {
            return;
        }

        // In regex mode with invalid pattern, don't search
        if self.mode == SearchMode::Regex && !self.regex_valid {
            return;
        }

        for (row_idx, row) in all_rows.iter().enumerate() {
            self.find_matches_in_row(row, row_idx);
        }

        // Clamp current_match to valid range
        if !self.matches.is_empty() && self.current_match >= self.matches.len() {
            self.current_match = self.matches.len() - 1;
        }
    }
}
