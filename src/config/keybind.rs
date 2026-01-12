//! Keybind parsing and configuration.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::config::Action;

/// A keybind specification.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Keybind {
    pub key: String,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Keybind {
    /// Parses a keybind string like "ctrl+shift+c" or "f12".
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.to_lowercase();
        let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();

        let mut ctrl = false;
        let mut shift = false;
        let mut alt = false;
        let mut key = None;

        for part in parts {
            match part {
                "ctrl" | "control" => ctrl = true,
                "shift" => shift = true,
                "alt" | "meta" => alt = true,
                k => key = Some(k.to_string()),
            }
        }

        key.map(|k| Keybind {
            key: k,
            ctrl,
            shift,
            alt,
        })
    }

    /// Checks if this keybind matches the given key and modifiers.
    pub fn matches(&self, key: &str, ctrl: bool, shift: bool, alt: bool) -> bool {
        self.key.eq_ignore_ascii_case(key)
            && self.ctrl == ctrl
            && self.shift == shift
            && self.alt == alt
    }
}

/// Keybind configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeybindConfig {
    /// Map of keybind strings to actions.
    #[serde(flatten)]
    pub bindings: HashMap<String, Action>,
}

impl Default for KeybindConfig {
    fn default() -> Self {
        let mut bindings = HashMap::new();

        // General actions
        bindings.insert("ctrl+shift+c".to_string(), Action::Copy);
        bindings.insert("ctrl+shift+v".to_string(), Action::Paste);
        bindings.insert("ctrl+v".to_string(), Action::Paste);
        bindings.insert("shift+insert".to_string(), Action::Paste);

        // Pane management
        bindings.insert("ctrl+shift+\\".to_string(), Action::SplitVertical);
        bindings.insert("ctrl+shift+-".to_string(), Action::SplitHorizontal);
        bindings.insert("alt+left".to_string(), Action::FocusLeft);
        bindings.insert("alt+right".to_string(), Action::FocusRight);
        bindings.insert("alt+up".to_string(), Action::FocusUp);
        bindings.insert("alt+down".to_string(), Action::FocusDown);
        bindings.insert("ctrl+shift+left".to_string(), Action::ResizeLeft);
        bindings.insert("ctrl+shift+right".to_string(), Action::ResizeRight);
        bindings.insert("ctrl+shift+up".to_string(), Action::ResizeUp);
        bindings.insert("ctrl+shift+down".to_string(), Action::ResizeDown);
        bindings.insert("ctrl+shift+w".to_string(), Action::ClosePane);
        bindings.insert("ctrl+shift+/".to_string(), Action::ToggleHelp);

        // Zoom (pane-local)
        bindings.insert("ctrl+alt+=".to_string(), Action::ZoomIn);
        bindings.insert("ctrl+alt+-".to_string(), Action::ZoomOut);
        bindings.insert("ctrl+alt+0".to_string(), Action::ZoomReset);

        bindings.insert("shift+pageup".to_string(), Action::ScrollPageUp);
        bindings.insert("shift+pagedown".to_string(), Action::ScrollPageDown);
        bindings.insert("shift+up".to_string(), Action::ScrollLineUp);
        bindings.insert("shift+down".to_string(), Action::ScrollLineDown);
        bindings.insert("ctrl+shift+home".to_string(), Action::ScrollToTop);
        bindings.insert("ctrl+shift+end".to_string(), Action::ScrollToBottom);
        bindings.insert("ctrl+shift+f".to_string(), Action::SearchFind);
        bindings.insert("ctrl+shift+k".to_string(), Action::ClearScrollback);

        // Search mode actions
        bindings.insert("escape".to_string(), Action::SearchClose);
        bindings.insert("enter".to_string(), Action::SearchConfirm);
        bindings.insert("ctrl+n".to_string(), Action::SearchNext);
        bindings.insert("ctrl+p".to_string(), Action::SearchPrev);
        bindings.insert("ctrl+c".to_string(), Action::SearchToggleCase);
        bindings.insert("down".to_string(), Action::SearchNext);
        bindings.insert("up".to_string(), Action::SearchPrev);

        KeybindConfig { bindings }
    }
}

impl KeybindConfig {
    /// Merges any missing default bindings into this config.
    ///
    /// This keeps user-overrides intact while ensuring new actions show up
    /// in existing `config.toml` files.
    pub fn apply_defaults(&mut self) {
        let defaults = KeybindConfig::default();
        for (key, action) in defaults.bindings {
            self.bindings.entry(key).or_insert(action);
        }
    }

    /// Finds the action for a given key and modifiers.
    pub fn get_action(&self, key: &str, ctrl: bool, shift: bool, alt: bool) -> Option<Action> {
        for (bind_str, action) in &self.bindings {
            if let Some(keybind) = Keybind::parse(bind_str) {
                if keybind.matches(key, ctrl, shift, alt) {
                    return Some(*action);
                }
            }
        }
        None
    }
}
