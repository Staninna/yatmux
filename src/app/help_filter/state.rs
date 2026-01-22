use nucleo_matcher::{Config, Matcher};

/// State for the help panel fuzzy filter.
#[derive(Debug)]
pub struct HelpFilterState {
    /// Whether filter mode is currently active.
    active: bool,
    /// Current filter query string.
    pub(super) query: String,
    /// Nucleo fuzzy matcher instance.
    pub(super) matcher: Matcher,
}

impl Default for HelpFilterState {
    fn default() -> Self {
        Self::new()
    }
}

impl HelpFilterState {
    /// Creates a new help filter state.
    pub fn new() -> Self {
        Self {
            active: false,
            query: String::new(),
            matcher: Matcher::new(Config::DEFAULT),
        }
    }

    /// Returns whether the filter is currently active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Returns the current query string.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Activates the filter mode.
    pub fn activate(&mut self) {
        self.active = true;
    }

    /// Deactivates the filter and clears the query.
    pub fn deactivate(&mut self) {
        self.active = false;
        self.query.clear();
    }

    /// Adds a character to the query.
    pub fn push_char(&mut self, ch: char) {
        self.query.push(ch);
    }

    /// Removes the last character from the query.
    pub fn pop_char(&mut self) {
        self.query.pop();
        // If query becomes empty, deactivate
        if self.query.is_empty() {
            self.active = false;
        }
    }
}
