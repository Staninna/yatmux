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
                    score: 0,
                }).collect(),
            }).collect();
        }

        let pattern = Pattern::parse(
            &self.query,
            CaseMatching::Ignore,
            Normalization::Never,
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
    pub score: i32,                 // Relevance score for sorting
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_sections() -> Vec<HelpSection> {
        vec![
            HelpSection {
                title: "Config".to_string(),
                bindings: vec![
                    ("ctrl+shift+r".to_string(), "Reload config".to_string()),
                ],
            },
            HelpSection {
                title: "Navigation".to_string(),
                bindings: vec![
                    ("alt+right".to_string(), "Focus right pane".to_string()),
                    ("ctrl+shift+right".to_string(), "Resize: give right".to_string()),
                    ("shift+insert".to_string(), "Paste".to_string()),
                    ("ctrl+t".to_string(), "New tab".to_string()),
                ],
            },
        ]
    }

    #[test]
    fn test_single_char_after_plus() {
        // Bug case: +r should match ctrl+shift+r
        let mut filter = HelpFilterState::new();
        filter.activate();
        filter.push_char('+');
        filter.push_char('r');

        let sections = create_test_sections();
        let results = filter.update_filter(&sections);

        // Should match both ctrl+shift+r and ctrl+shift+right
        let all_bindings: Vec<_> = results.iter()
            .flat_map(|s| &s.bindings)
            .collect();

        assert!(!all_bindings.is_empty(), "Should have matches for +r");

        let has_reload = all_bindings.iter()
            .any(|b| b.key == "ctrl+shift+r" && b.action == "Reload config");
        let has_right = all_bindings.iter()
            .any(|b| b.key == "ctrl+shift+right");

        assert!(has_reload, "Should match ctrl+shift+r Reload config");
        assert!(has_right, "Should match ctrl+shift+right");
    }

    #[test]
    fn test_multi_char_after_plus() {
        // +ri should match ctrl+shift+right
        let mut filter = HelpFilterState::new();
        filter.activate();
        filter.push_char('+');
        filter.push_char('r');
        filter.push_char('i');

        let sections = create_test_sections();
        let results = filter.update_filter(&sections);

        let all_bindings: Vec<_> = results.iter()
            .flat_map(|s| &s.bindings)
            .collect();

        let has_right = all_bindings.iter()
            .any(|b| b.key == "ctrl+shift+right");

        assert!(has_right, "Should match ctrl+shift+right");
    }

    #[test]
    fn test_action_label_matching() {
        // "relo" should match "Reload config"
        let mut filter = HelpFilterState::new();
        filter.activate();
        for ch in "relo".chars() {
            filter.push_char(ch);
        }

        let sections = create_test_sections();
        let results = filter.update_filter(&sections);

        let all_bindings: Vec<_> = results.iter()
            .flat_map(|s| &s.bindings)
            .collect();

        let has_reload = all_bindings.iter()
            .any(|b| b.action == "Reload config");

        assert!(has_reload, "Should match 'Reload config' action");
    }

    #[test]
    fn test_case_insensitive() {
        // "REL" should match "Reload config" (case-insensitive)
        let mut filter = HelpFilterState::new();
        filter.activate();
        for ch in "REL".chars() {
            filter.push_char(ch);
        }

        let sections = create_test_sections();
        let results = filter.update_filter(&sections);

        let all_bindings: Vec<_> = results.iter()
            .flat_map(|s| &s.bindings)
            .collect();

        let has_reload = all_bindings.iter()
            .any(|b| b.action == "Reload config");

        assert!(has_reload, "Should match case-insensitively");
    }

    #[test]
    fn test_fuzzy_matching() {
        // "csr" should fuzzy match "ctrl+shift+r"
        let mut filter = HelpFilterState::new();
        filter.activate();
        for ch in "csr".chars() {
            filter.push_char(ch);
        }

        let sections = create_test_sections();
        let results = filter.update_filter(&sections);

        let all_bindings: Vec<_> = results.iter()
            .flat_map(|s| &s.bindings)
            .collect();

        let has_reload = all_bindings.iter()
            .any(|b| b.key == "ctrl+shift+r");

        assert!(has_reload, "Should fuzzy match ctrl+shift+r");
    }

    #[test]
    fn test_empty_query_returns_all() {
        let mut filter = HelpFilterState::new();
        filter.activate();

        let sections = create_test_sections();
        let results = filter.update_filter(&sections);

        // Count expected bindings
        let expected_count: usize = sections.iter()
            .map(|s| s.bindings.len())
            .sum();

        let result_count: usize = results.iter()
            .map(|s| s.bindings.len())
            .sum();

        assert_eq!(result_count, expected_count, "Empty query should return all bindings");
    }

    #[test]
    fn test_score_based_sorting() {
        let mut filter = HelpFilterState::new();
        filter.activate();
        filter.push_char('r');

        let sections = create_test_sections();
        let results = filter.update_filter(&sections);

        // Verify that bindings within each section are sorted by score descending
        for section in &results {
            for i in 1..section.bindings.len() {
                assert!(
                    section.bindings[i-1].score >= section.bindings[i].score,
                    "Bindings should be sorted by score (descending)"
                );
            }
        }
    }

    #[test]
    fn test_plus_t_matches() {
        // "+t" should match "ctrl+t"
        let mut filter = HelpFilterState::new();
        filter.activate();
        filter.push_char('+');
        filter.push_char('t');

        let sections = create_test_sections();
        let results = filter.update_filter(&sections);

        let all_bindings: Vec<_> = results.iter()
            .flat_map(|s| &s.bindings)
            .collect();

        let has_ctrl_t = all_bindings.iter()
            .any(|b| b.key == "ctrl+t");

        assert!(has_ctrl_t, "Should match ctrl+t");
    }

    #[test]
    fn test_deactivate_clears_query() {
        let mut filter = HelpFilterState::new();
        filter.activate();
        filter.push_char('t');
        filter.push_char('e');
        filter.push_char('s');
        filter.push_char('t');

        assert_eq!(filter.query(), "test");
        assert!(filter.is_active());

        filter.deactivate();

        assert_eq!(filter.query(), "");
        assert!(!filter.is_active());
    }

    #[test]
    fn test_pop_char_deactivates_on_empty() {
        let mut filter = HelpFilterState::new();
        filter.activate();
        filter.push_char('a');

        assert!(filter.is_active());

        filter.pop_char();

        assert!(!filter.is_active());
        assert_eq!(filter.query(), "");
    }
}
