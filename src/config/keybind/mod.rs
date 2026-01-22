//! Keybind parsing and configuration.

mod defaults;
mod lookup;
mod parse;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::config::Action;

pub use parse::Keybind;

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
