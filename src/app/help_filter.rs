//! Help panel fuzzy filter state and matching logic.

use nucleo_matcher::{Matcher, Config, Utf32Str};
use nucleo_matcher::pattern::{Pattern, Normalization, CaseMatching};
use yatmux::renderer::HelpSection;

/// State for the help panel fuzzy filter.
#[derive(Debug)]
pub struct HelpFilterState {
    /// Whether filter mode is currently active.
    active: bool,
    /// Current filter query string.
    query: String,
    /// Nucleo fuzzy matcher instance.
    matcher: Matcher,
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

    /// Applies the filter to help sections, returning filtered results with match information.
    pub fn update_filter(&mut self, sections: &[HelpSection]) -> Vec<FilteredHelpSection> {
        if self.query.is_empty() {
            // Empty query - return all sections converted to filtered format
            return sections.iter().map(|section| FilteredHelpSection {
                title: section.title.clone(),
                bindings: section.bindings.iter().map(|(key, action)| FilteredBinding {
                    key: key.clone(),
                    action: action.clone(),
                    key_matches: Vec::new(),
                    action_matches: Vec::new(),
                    score: 0,
                }).collect(),
            }).collect();
        }

        let pattern = Pattern::parse(
            &self.query,
            CaseMatching::Ignore,
            Normalization::Smart,
        );

        let mut filtered_sections = Vec::new();

        for section in sections {
            let mut filtered_bindings = Vec::new();

            for (key, action) in &section.bindings {
                // Try to match against both key and action
                let mut key_indices = Vec::new();
                let mut action_indices = Vec::new();

                let key_score = pattern.indices(
                    Utf32Str::Ascii(key.as_bytes()),
                    &mut self.matcher,
                    &mut key_indices,
                ).unwrap_or(0);

                let action_score = pattern.indices(
                    Utf32Str::Ascii(action.as_bytes()),
                    &mut self.matcher,
                    &mut action_indices,
                ).unwrap_or(0);

                // Use the higher score
                let best_score = key_score.max(action_score);

                if best_score > 0 {
                    filtered_bindings.push(FilteredBinding {
                        key: key.clone(),
                        action: action.clone(),
                        key_matches: key_indices,
                        action_matches: action_indices,
                        score: best_score as i32,
                    });
                }
            }

            // Only include section if it has matches
            if !filtered_bindings.is_empty() {
                // Sort bindings by score (descending)
                filtered_bindings.sort_by(|a, b| b.score.cmp(&a.score));

                filtered_sections.push(FilteredHelpSection {
                    title: section.title.clone(),
                    bindings: filtered_bindings,
                });
            }
        }

        filtered_sections
    }
}

/// A filtered help section with match information.
#[derive(Debug, Clone)]
pub struct FilteredHelpSection {
    pub title: String,
    pub bindings: Vec<FilteredBinding>,
}

/// A filtered keybinding with match highlights.
#[derive(Debug, Clone)]
pub struct FilteredBinding {
    pub key: String,
    pub action: String,
    pub key_matches: Vec<u32>,      // Character indices that matched in key
    pub action_matches: Vec<u32>,   // Character indices that matched in action
    pub score: i32,                 // Relevance score for sorting
}

impl FilteredHelpSection {
    /// Converts back to a simple HelpSection (for rendering when filter is inactive).
    pub fn to_help_section(&self) -> HelpSection {
        HelpSection {
            title: self.title.clone(),
            bindings: self.bindings.iter()
                .map(|b| (b.key.clone(), b.action.clone()))
                .collect(),
        }
    }
}
