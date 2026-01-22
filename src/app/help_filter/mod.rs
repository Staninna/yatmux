//! Help panel fuzzy filter state and matching logic.

mod matcher;
mod results;
mod state;

pub use state::HelpFilterState;

#[cfg(test)]
mod tests {
    use super::*;
    use yatmux::renderer::HelpSection;

    fn create_test_sections() -> Vec<HelpSection> {
        vec![
            HelpSection {
                title: "Config".to_string(),
                bindings: vec![("ctrl+shift+r".to_string(), "Reload config".to_string())],
            },
            HelpSection {
                title: "Navigation".to_string(),
                bindings: vec![
                    ("alt+right".to_string(), "Focus right pane".to_string()),
                    (
                        "ctrl+shift+right".to_string(),
                        "Resize: give right".to_string(),
                    ),
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
        let all_bindings: Vec<_> = results.iter().flat_map(|s| &s.bindings).collect();

        assert!(!all_bindings.is_empty(), "Should have matches for +r");

        let has_reload = all_bindings
            .iter()
            .any(|b| b.key == "ctrl+shift+r" && b.action == "Reload config");
        let has_right = all_bindings.iter().any(|b| b.key == "ctrl+shift+right");

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

        let all_bindings: Vec<_> = results.iter().flat_map(|s| &s.bindings).collect();

        let has_right = all_bindings.iter().any(|b| b.key == "ctrl+shift+right");

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

        let all_bindings: Vec<_> = results.iter().flat_map(|s| &s.bindings).collect();

        let has_reload = all_bindings.iter().any(|b| b.action == "Reload config");

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

        let all_bindings: Vec<_> = results.iter().flat_map(|s| &s.bindings).collect();

        let has_reload = all_bindings.iter().any(|b| b.action == "Reload config");

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

        let all_bindings: Vec<_> = results.iter().flat_map(|s| &s.bindings).collect();

        let has_reload = all_bindings.iter().any(|b| b.key == "ctrl+shift+r");

        assert!(has_reload, "Should fuzzy match ctrl+shift+r");
    }

    #[test]
    fn test_empty_query_returns_all() {
        let mut filter = HelpFilterState::new();
        filter.activate();

        let sections = create_test_sections();
        let results = filter.update_filter(&sections);

        // Count expected bindings
        let expected_count: usize = sections.iter().map(|s| s.bindings.len()).sum();

        let result_count: usize = results.iter().map(|s| s.bindings.len()).sum();

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
                    section.bindings[i - 1].score >= section.bindings[i].score,
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

        let all_bindings: Vec<_> = results.iter().flat_map(|s| &s.bindings).collect();

        let has_ctrl_t = all_bindings.iter().any(|b| b.key == "ctrl+t");

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
