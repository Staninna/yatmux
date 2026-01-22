use super::super::*;
use yatmux::config::{KeybindAction, PluginKeybind};

pub(super) struct ResolvedKeybind {
    pub(super) action: Option<Action>,
    pub(super) plugin_binding: Option<PluginKeybind>,
    pub(super) non_search_action: Option<Action>,
}

impl App {
    pub(super) fn resolve_keybinding(
        &self,
        key_str: Option<&str>,
        modifiers: winit::keyboard::ModifiersState,
    ) -> ResolvedKeybind {
        let ctrl = modifiers.control_key();
        let shift = modifiers.shift_key();
        let alt = modifiers.alt_key();

        // Get profile-aware keybind
        let profile_name = self
            .active_tab()
            .and_then(|t| t.focused_pane())
            .map(|p| p.profile.as_str())
            .unwrap_or("default");
        let empty_keybinds = std::collections::HashMap::new();
        let profile_keybinds = self
            .config
            .profiles
            .get_profile(profile_name)
            .map(|p| &p.keybinds)
            .unwrap_or(&empty_keybinds);

        let binding = if let Some(key) = key_str {
            let global_binding = self.config.keybinds.get_binding(key, ctrl, shift, alt);
            let global_action = global_binding.as_ref().and_then(|b| b.builtin_action());
            if global_action.is_some_and(is_global_only_action) {
                global_binding
            } else {
                self.config
                    .keybinds
                    .get_binding_with_profile(key, ctrl, shift, alt, profile_keybinds)
            }
        } else {
            None
        };
        let action = binding.as_ref().and_then(|b| b.builtin_action());
        let plugin_binding = binding.as_ref().and_then(|b| match b {
            KeybindAction::Plugin(plugin) => Some(plugin.clone()),
            _ => None,
        });
        let non_search_action = action.filter(|a| !a.is_search_mode_only());

        ResolvedKeybind {
            action,
            plugin_binding,
            non_search_action,
        }
    }
}

// Keybind resolution is profile-aware, but some core app actions are reserved to the
// global `[keybinds]` section to avoid trapping the user inside a profile.
fn is_global_only_action(action: Action) -> bool {
    matches!(
        action,
        Action::NewTab
            | Action::CloseTab
            | Action::NextTab
            | Action::PrevTab
            | Action::Tab1
            | Action::Tab2
            | Action::Tab3
            | Action::Tab4
            | Action::Tab5
            | Action::Tab6
            | Action::Tab7
            | Action::Tab8
            | Action::Tab9
            | Action::ToggleHelp
            | Action::ReloadConfig
            | Action::CycleProfile
            | Action::CycleProfileReverse
            | Action::SwitchToProfile1
            | Action::SwitchToProfile2
            | Action::SwitchToProfile3
            | Action::SwitchToProfile4
            | Action::SwitchToProfile5
            | Action::SwitchToProfile6
            | Action::SwitchToProfile7
            | Action::SwitchToProfile8
            | Action::SwitchToProfile9
    )
}
