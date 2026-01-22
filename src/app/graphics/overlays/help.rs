use std::collections::HashMap;

use yatmux::renderer::{HelpSection, UiStyle};

use crate::app::HelpFilterState;
use yatmux::config::Config;
use yatmux::renderer::Renderer;

pub(super) fn render_help_overlay(
    renderer: &mut Renderer,
    help_filter: &mut HelpFilterState,
    help_scroll: &mut usize,
    help_max_scroll: &mut usize,
    config: &Config,
    shell_warning_dismissed: bool,
    buffer: &mut [u32],
    buffer_width: usize,
    buffer_height: usize,
    accent_color: u32,
    font_scale: f32,
    ui_style: &UiStyle,
    font_config: &yatmux::config::FontConfig,
    shell_integration_detected: bool,
) {
    *help_scroll = (*help_scroll).min(*help_max_scroll);

    let mut by_category: HashMap<&'static str, Vec<(String, String)>> = HashMap::new();
    for (key, action) in &config.keybinds.bindings {
        if action.is_disabled() {
            continue;
        }
        by_category
            .entry(action.category())
            .or_default()
            .push((key.clone(), action.label()));
    }

    // Consolidate numbered entries (e.g., "Go to tab 1" through "Go to tab 9")
    for bindings in by_category.values_mut() {
        consolidate_numbered_bindings(bindings);
        bindings.sort_by(|a, b| a.0.cmp(&b.0));
    }

    let order = ["General", "Tabs", "Panes", "Zoom", "Scrollback", "Search", "Help"];
    let mut sections: Vec<HelpSection> = Vec::new();

    for category in order {
        if let Some(bindings) = by_category.remove(category) {
            sections.push(HelpSection {
                title: category.to_string(),
                bindings,
            });
        }
    }

    let mut extra: Vec<(&'static str, Vec<(String, String)>)> = by_category.into_iter().collect();
    extra.sort_by(|a, b| a.0.cmp(b.0));
    for (category, bindings) in extra {
        sections.push(HelpSection {
            title: category.to_string(),
            bindings,
        });
    }

    // Apply filter if active
    let (sections_to_render, filter_query, match_count) = if help_filter.is_active() {
        let filtered = help_filter.update_filter(&sections);
        let count = filtered.iter().map(|s| s.bindings.len()).sum();
        // Convert filtered sections back to HelpSection for rendering
        let converted: Vec<HelpSection> = filtered
            .iter()
            .map(|fs| HelpSection {
                title: fs.title.clone(),
                bindings: fs
                    .bindings
                    .iter()
                    .map(|b| (b.key.clone(), b.action.clone()))
                    .collect(),
            })
            .collect();
        (converted, Some(help_filter.query().to_string()), Some(count))
    } else {
        (sections, None, None)
    };

    let (scroll, max_scroll) = renderer.paint_help_overlay(
        buffer,
        buffer_width,
        buffer_height,
        "Shortcuts",
        &sections_to_render,
        filter_query,
        match_count,
        *help_scroll,
        accent_color,
        font_scale,
        shell_integration_detected,
        shell_warning_dismissed,
        ui_style,
        font_config,
    );
    *help_scroll = scroll;
    *help_max_scroll = max_scroll;
}

/// Consolidates numbered keybindings like "alt+1" -> "Go to tab 1" through "alt+9" -> "Go to tab 9"
/// into a single entry like "alt+1-9" -> "Go to tab 1-9".
fn consolidate_numbered_bindings(bindings: &mut Vec<(String, String)>) {
    // Look for patterns like "Go to tab N" with keys like "alt+N"
    let patterns = [("Go to tab ", "alt+")];

    for (label_prefix, key_prefix) in patterns {
        // Find all matching entries
        let mut matches: Vec<(usize, char)> = Vec::new();
        for (i, (key, label)) in bindings.iter().enumerate() {
            if label.starts_with(label_prefix) && key.starts_with(key_prefix) {
                let suffix = &label[label_prefix.len()..];
                let key_suffix = &key[key_prefix.len()..];
                if suffix.len() == 1 && key_suffix == suffix {
                    if let Some(digit) = suffix.chars().next() {
                        if digit.is_ascii_digit() {
                            matches.push((i, digit));
                        }
                    }
                }
            }
        }

        // If we have multiple consecutive numbers, consolidate them
        if matches.len() >= 3 {
            matches.sort_by_key(|(_, d)| *d);
            let digits: Vec<char> = matches.iter().map(|(_, d)| *d).collect();
            let first = digits.first().unwrap();
            let last = digits.last().unwrap();

            // Remove original entries (in reverse order to preserve indices)
            let mut indices: Vec<usize> = matches.iter().map(|(i, _)| *i).collect();
            indices.sort();
            indices.reverse();
            for i in indices {
                bindings.remove(i);
            }

            // Add consolidated entry
            bindings.push((
                format!("{}{}-{}", key_prefix, first, last),
                format!("{}{}-{}", label_prefix, first, last),
            ));
        }
    }
}
