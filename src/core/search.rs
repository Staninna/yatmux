//! Search functionality for the terminal scrollback buffer.
//!
//! Provides search within terminal history with match highlighting.
//! Supports both plain text and regex search modes.

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
    active: bool,
    /// Current search query.
    query: String,
    /// All matches found in the entire scrollback + live buffer.
    matches: Vec<SearchMatch>,
    /// Index of the currently selected match.
    current_match: usize,
    /// Whether search is case-sensitive.
    case_sensitive: bool,
    /// Search mode (plain text or regex).
    mode: SearchMode,
    /// Compiled regex (cached when in regex mode).
    compiled_regex: Option<Regex>,
    /// Whether the current regex is valid.
    regex_valid: bool,
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

    /// Updates the compiled regex cache.
    fn update_compiled_regex(&mut self) {
        if self.mode == SearchMode::Regex && !self.query.is_empty() {
            let pattern = if self.case_sensitive {
                self.query.clone()
            } else {
                format!("(?i){}", self.query)
            };
            match Regex::new(&pattern) {
                Ok(re) => {
                    self.compiled_regex = Some(re);
                    self.regex_valid = true;
                }
                Err(_) => {
                    self.compiled_regex = None;
                    self.regex_valid = false;
                }
            }
        } else {
            self.compiled_regex = None;
            self.regex_valid = true;
        }
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

    /// Finds all matches in a single row.
    fn find_matches_in_row(&mut self, row: &RowSnapshot, row_idx: usize) {
        let row_text: String = row.cells.iter().map(|(ch, _, _)| *ch).collect();

        match self.mode {
            SearchMode::Regex => {
                if let Some(ref re) = self.compiled_regex {
                    for m in re.find_iter(&row_text) {
                        // Convert byte offsets to character offsets
                        let start_col = row_text[..m.start()].chars().count();
                        let end_col = row_text[..m.end()].chars().count();

                        // Skip empty matches
                        if start_col < end_col {
                            self.matches.push(SearchMatch {
                                row: row_idx,
                                start_col,
                                end_col,
                            });
                        }
                    }
                }
            }
            SearchMode::Plain => {
                let search_text = if self.case_sensitive {
                    row_text.clone()
                } else {
                    row_text.to_lowercase()
                };

                let query = if self.case_sensitive {
                    self.query.clone()
                } else {
                    self.query.to_lowercase()
                };

                // `str::find` returns byte offsets. We must convert to character indices,
                // since terminal columns are in cells (chars), not bytes.
                let mut start_byte = 0;
                while let Some(pos) = search_text[start_byte..].find(&query) {
                    let match_start_byte = start_byte + pos;
                    let match_end_byte = match_start_byte + query.len();

                    let start_col = search_text[..match_start_byte].chars().count();
                    let end_col = search_text[..match_end_byte].chars().count();

                    self.matches.push(SearchMatch {
                        row: row_idx,
                        start_col,
                        end_col,
                    });

                    // Advance by one character to allow overlapping matches.
                    let advance = search_text[match_start_byte..]
                        .chars()
                        .next()
                        .map(|ch| ch.len_utf8())
                        .unwrap_or(1);
                    start_byte = (match_start_byte + advance).min(search_text.len());
                }
            }
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
