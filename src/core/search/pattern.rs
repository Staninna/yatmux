use regex::Regex;

use crate::core::grid::RowSnapshot;

use super::state::{SearchMode, SearchMatch, SearchState};

impl SearchState {
    /// Updates the compiled regex cache.
    pub(super) fn update_compiled_regex(&mut self) {
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

    /// Finds all matches in a single row.
    pub(super) fn find_matches_in_row(&mut self, row: &RowSnapshot, row_idx: usize) {
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
}
