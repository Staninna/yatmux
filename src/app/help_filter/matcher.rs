use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::Utf32Str;
use yatmux::renderer::HelpSection;

use super::results::{FilteredBinding, FilteredHelpSection};
use super::state::HelpFilterState;

impl HelpFilterState {
    /// Applies the filter to help sections, returning filtered results with match information.
    pub fn update_filter(&mut self, sections: &[HelpSection]) -> Vec<FilteredHelpSection> {
        if self.query().is_empty() {
            // Empty query - return all sections converted to filtered format
            return sections
                .iter()
                .map(|section| FilteredHelpSection {
                    title: section.title.clone(),
                    bindings: section
                        .bindings
                        .iter()
                        .map(|(key, action)| FilteredBinding {
                            key: key.clone(),
                            action: action.clone(),
                            score: 0,
                        })
                        .collect(),
                })
                .collect();
        }

        let pattern = Pattern::parse(self.query(), CaseMatching::Ignore, Normalization::Never);

        let mut filtered_sections = Vec::new();

        for section in sections {
            let mut filtered_bindings = Vec::new();

            for (key, action) in &section.bindings {
                // Try to match against both key and action
                let mut key_indices = Vec::new();
                let mut action_indices = Vec::new();

                let key_score = pattern
                    .indices(
                        Utf32Str::Ascii(key.as_bytes()),
                        &mut self.matcher,
                        &mut key_indices,
                    )
                    .unwrap_or(0);

                let action_score = pattern
                    .indices(
                        Utf32Str::Ascii(action.as_bytes()),
                        &mut self.matcher,
                        &mut action_indices,
                    )
                    .unwrap_or(0);

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
