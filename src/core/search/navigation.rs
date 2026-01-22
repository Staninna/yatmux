use super::state::{SearchMatch, SearchState};

impl SearchState {
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
