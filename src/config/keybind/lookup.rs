use std::collections::HashMap;

use crate::config::Action;

use super::{Keybind, KeybindAction, KeybindConfig};

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

    /// Finds the binding for a given key and modifiers with profile support.
    ///
    /// First checks profile keybinds, then falls back to global keybinds.
    /// Returns `None` if the key is not bound, or if explicitly disabled with `"none"`.
    pub fn get_binding_with_profile(
        &self,
        key: &str,
        ctrl: bool,
        shift: bool,
        alt: bool,
        profile_keybinds: &HashMap<String, KeybindAction>,
    ) -> Option<KeybindAction> {
        // First check profile keybinds
        for (bind_str, action) in profile_keybinds {
            if let Some(keybind) = Keybind::parse(bind_str) {
                if keybind.matches(key, ctrl, shift, alt) {
                    if action.is_disabled() {
                        return None;
                    }
                    return Some(action.clone());
                }
            }
        }
        // Fall back to global keybinds
        self.get_binding(key, ctrl, shift, alt)
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
