//! Keybind parsing and configuration.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::config::Action;

/// A keybinding action (built-in or plugin command).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KeybindAction {
    Builtin(Action),
    Plugin(PluginKeybind),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginKeybind {
    pub plugin: String,
    pub command: String,
    #[serde(default)]
    pub args: Option<toml::Value>,
}

impl KeybindAction {
    pub fn is_disabled(&self) -> bool {
        matches!(self, KeybindAction::Builtin(Action::None))
    }

    pub fn category(&self) -> &'static str {
        match self {
            KeybindAction::Builtin(action) => action.category(),
            KeybindAction::Plugin(_) => "Plugins",
        }
    }

    pub fn label(&self) -> String {
        match self {
            KeybindAction::Builtin(action) => action.label().to_string(),
            KeybindAction::Plugin(plugin) => {
                format!("Plugin: {} {}", plugin.plugin, plugin.command)
            }
        }
    }

    pub fn builtin_action(&self) -> Option<Action> {
        match self {
            KeybindAction::Builtin(action) => Some(*action),
            KeybindAction::Plugin(_) => None,
        }
    }
}

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
///
/// Bindings map key combinations to actions. Use `"none"` to disable a default binding:
///
/// ```toml
/// [keybinds]
/// "ctrl+shift+-" = "none"  # Disable horizontal split
/// "ctrl+shift+\\" = "none"  # Disable vertical split
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeybindConfig {
    /// Map of keybind strings to actions.
    #[serde(flatten)]
    pub bindings: HashMap<String, KeybindAction>,
}

impl Default for KeybindConfig {
    fn default() -> Self {
        let mut bindings = HashMap::new();

        // General actions
        bindings.insert(
            "ctrl+shift+c".to_string(),
            KeybindAction::Builtin(Action::Copy),
        );
        bindings.insert(
            "ctrl+shift+v".to_string(),
            KeybindAction::Builtin(Action::Paste),
        );
        bindings.insert("ctrl+v".to_string(), KeybindAction::Builtin(Action::Paste));
        bindings.insert(
            "shift+insert".to_string(),
            KeybindAction::Builtin(Action::Paste),
        );

        // Tab management
        bindings.insert("ctrl+shift+t".to_string(), KeybindAction::Builtin(Action::NewTab));
        bindings.insert(
            "ctrl+shift+q".to_string(),
            KeybindAction::Builtin(Action::CloseTab),
        );
        bindings.insert("ctrl+tab".to_string(), KeybindAction::Builtin(Action::NextTab));
        bindings.insert(
            "ctrl+shift+tab".to_string(),
            KeybindAction::Builtin(Action::PrevTab),
        );
        bindings.insert("alt+1".to_string(), KeybindAction::Builtin(Action::Tab1));
        bindings.insert("alt+2".to_string(), KeybindAction::Builtin(Action::Tab2));
        bindings.insert("alt+3".to_string(), KeybindAction::Builtin(Action::Tab3));
        bindings.insert("alt+4".to_string(), KeybindAction::Builtin(Action::Tab4));
        bindings.insert("alt+5".to_string(), KeybindAction::Builtin(Action::Tab5));
        bindings.insert("alt+6".to_string(), KeybindAction::Builtin(Action::Tab6));
        bindings.insert("alt+7".to_string(), KeybindAction::Builtin(Action::Tab7));
        bindings.insert("alt+8".to_string(), KeybindAction::Builtin(Action::Tab8));
        bindings.insert("alt+9".to_string(), KeybindAction::Builtin(Action::Tab9));

        // Pane management
        bindings.insert(
            "ctrl+shift+\\".to_string(),
            KeybindAction::Builtin(Action::SplitVertical),
        );
        bindings.insert(
            "ctrl+shift+-".to_string(),
            KeybindAction::Builtin(Action::SplitHorizontal),
        );
        bindings.insert("alt+left".to_string(), KeybindAction::Builtin(Action::FocusLeft));
        bindings.insert("alt+right".to_string(), KeybindAction::Builtin(Action::FocusRight));
        bindings.insert("alt+up".to_string(), KeybindAction::Builtin(Action::FocusUp));
        bindings.insert("alt+down".to_string(), KeybindAction::Builtin(Action::FocusDown));
        bindings.insert(
            "ctrl+shift+left".to_string(),
            KeybindAction::Builtin(Action::ResizeLeft),
        );
        bindings.insert(
            "ctrl+shift+right".to_string(),
            KeybindAction::Builtin(Action::ResizeRight),
        );
        bindings.insert(
            "ctrl+shift+up".to_string(),
            KeybindAction::Builtin(Action::ResizeUp),
        );
        bindings.insert(
            "ctrl+shift+down".to_string(),
            KeybindAction::Builtin(Action::ResizeDown),
        );
        bindings.insert(
            "ctrl+shift+w".to_string(),
            KeybindAction::Builtin(Action::ClosePane),
        );
        bindings.insert(
            "ctrl+shift+/".to_string(),
            KeybindAction::Builtin(Action::ToggleHelp),
        );

        // Zoom (pane-local)
        bindings.insert("ctrl+alt+=".to_string(), KeybindAction::Builtin(Action::ZoomIn));
        bindings.insert("ctrl+alt+-".to_string(), KeybindAction::Builtin(Action::ZoomOut));
        bindings.insert(
            "ctrl+alt+0".to_string(),
            KeybindAction::Builtin(Action::ZoomReset),
        );

        bindings.insert(
            "shift+pageup".to_string(),
            KeybindAction::Builtin(Action::ScrollPageUp),
        );
        bindings.insert(
            "shift+pagedown".to_string(),
            KeybindAction::Builtin(Action::ScrollPageDown),
        );
        bindings.insert(
            "shift+up".to_string(),
            KeybindAction::Builtin(Action::ScrollLineUp),
        );
        bindings.insert(
            "shift+down".to_string(),
            KeybindAction::Builtin(Action::ScrollLineDown),
        );
        bindings.insert(
            "ctrl+shift+home".to_string(),
            KeybindAction::Builtin(Action::ScrollToTop),
        );
        bindings.insert(
            "ctrl+shift+end".to_string(),
            KeybindAction::Builtin(Action::ScrollToBottom),
        );
        bindings.insert(
            "ctrl+shift+f".to_string(),
            KeybindAction::Builtin(Action::SearchFind),
        );
        bindings.insert(
            "ctrl+shift+k".to_string(),
            KeybindAction::Builtin(Action::ClearScrollback),
        );

        // Search mode actions
        bindings.insert("escape".to_string(), KeybindAction::Builtin(Action::SearchClose));
        bindings.insert("enter".to_string(), KeybindAction::Builtin(Action::SearchConfirm));
        bindings.insert("ctrl+n".to_string(), KeybindAction::Builtin(Action::SearchNext));
        bindings.insert("ctrl+p".to_string(), KeybindAction::Builtin(Action::SearchPrev));
        bindings.insert(
            "ctrl+c".to_string(),
            KeybindAction::Builtin(Action::SearchToggleCase),
        );
        bindings.insert(
            "ctrl+r".to_string(),
            KeybindAction::Builtin(Action::SearchToggleRegex),
        );
        bindings.insert("down".to_string(), KeybindAction::Builtin(Action::SearchNext));
        bindings.insert("up".to_string(), KeybindAction::Builtin(Action::SearchPrev));

        // Config
        bindings.insert(
            "ctrl+shift+r".to_string(),
            KeybindAction::Builtin(Action::ReloadConfig),
        );

        // Shell integration actions
        bindings.insert(
            "ctrl+shift+o".to_string(),
            KeybindAction::Builtin(Action::CopyLastOutput),
        );
        bindings.insert(
            "ctrl+shift+pageup".to_string(),
            KeybindAction::Builtin(Action::JumpToPrevPrompt),
        );
        bindings.insert(
            "ctrl+shift+pagedown".to_string(),
            KeybindAction::Builtin(Action::JumpToNextPrompt),
        );
        bindings.insert(
            "ctrl+shift+y".to_string(),
            KeybindAction::Builtin(Action::ToggleShadowPrompt),
        );

        KeybindConfig { bindings }
    }
}

impl KeybindConfig {
    /// Merges any missing default bindings into this config.
    ///
    /// This keeps user-overrides intact (including `"none"` to disable)
    /// while ensuring new actions show up in existing `config.toml` files.
    pub fn apply_defaults(&mut self) {
        let defaults = KeybindConfig::default();
        for (key, action) in defaults.bindings {
            self.bindings.entry(key).or_insert(action);
        }
    }

    /// Finds the binding for a given key and modifiers.
    ///
    /// Returns `None` if the key is not bound, or if explicitly disabled with `"none"`.
    pub fn get_binding(
        &self,
        key: &str,
        ctrl: bool,
        shift: bool,
        alt: bool,
    ) -> Option<KeybindAction> {
        for (bind_str, action) in &self.bindings {
            if let Some(keybind) = Keybind::parse(bind_str) {
                if keybind.matches(key, ctrl, shift, alt) {
                    if action.is_disabled() {
                        return None;
                    }
                    return Some(action.clone());
                }
            }
        }
        None
    }

    /// Finds the built-in action for a given key and modifiers.
    ///
    /// Returns `None` if the key is unbound, disabled, or mapped to a plugin action.
    pub fn get_action(&self, key: &str, ctrl: bool, shift: bool, alt: bool) -> Option<Action> {
        self.get_binding(key, ctrl, shift, alt)
            .and_then(|action| action.builtin_action())
    }

    /// Checks if a keybind is explicitly disabled (set to "none").
    pub fn is_disabled(&self, key: &str, ctrl: bool, shift: bool, alt: bool) -> bool {
        for (bind_str, action) in &self.bindings {
            if let Some(keybind) = Keybind::parse(bind_str) {
                if keybind.matches(key, ctrl, shift, alt) {
                    return action.is_disabled();
                }
            }
        }
        false
    }
}
