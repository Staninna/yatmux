use std::collections::HashMap;

use crate::config::Action;

use super::{KeybindAction, KeybindConfig};

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
        bindings.insert(
            "ctrl+shift+t".to_string(),
            KeybindAction::Builtin(Action::NewTab),
        );
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
        bindings.insert(
            "alt+right".to_string(),
            KeybindAction::Builtin(Action::FocusRight),
        );
        bindings.insert("alt+up".to_string(), KeybindAction::Builtin(Action::FocusUp));
        bindings.insert(
            "alt+down".to_string(),
            KeybindAction::Builtin(Action::FocusDown),
        );
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
        bindings.insert(
            "ctrl+alt+=".to_string(),
            KeybindAction::Builtin(Action::ZoomIn),
        );
        bindings.insert(
            "ctrl+alt+-".to_string(),
            KeybindAction::Builtin(Action::ZoomOut),
        );
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
        bindings.insert(
            "escape".to_string(),
            KeybindAction::Builtin(Action::SearchClose),
        );
        bindings.insert(
            "enter".to_string(),
            KeybindAction::Builtin(Action::SearchConfirm),
        );
        bindings.insert(
            "ctrl+n".to_string(),
            KeybindAction::Builtin(Action::SearchNext),
        );
        bindings.insert(
            "ctrl+p".to_string(),
            KeybindAction::Builtin(Action::SearchPrev),
        );
        bindings.insert(
            "ctrl+c".to_string(),
            KeybindAction::Builtin(Action::SearchToggleCase),
        );
        bindings.insert(
            "ctrl+r".to_string(),
            KeybindAction::Builtin(Action::SearchToggleRegex),
        );
        bindings.insert(
            "down".to_string(),
            KeybindAction::Builtin(Action::SearchNext),
        );
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

        // Profile management
        bindings.insert(
            "ctrl+shift+p".to_string(),
            KeybindAction::Builtin(Action::CycleProfile),
        );
        bindings.insert(
            "ctrl+alt+1".to_string(),
            KeybindAction::Builtin(Action::SwitchToProfile1),
        );
        bindings.insert(
            "ctrl+alt+2".to_string(),
            KeybindAction::Builtin(Action::SwitchToProfile2),
        );
        bindings.insert(
            "ctrl+alt+3".to_string(),
            KeybindAction::Builtin(Action::SwitchToProfile3),
        );
        bindings.insert(
            "ctrl+alt+4".to_string(),
            KeybindAction::Builtin(Action::SwitchToProfile4),
        );
        bindings.insert(
            "ctrl+alt+5".to_string(),
            KeybindAction::Builtin(Action::SwitchToProfile5),
        );
        bindings.insert(
            "ctrl+alt+6".to_string(),
            KeybindAction::Builtin(Action::SwitchToProfile6),
        );
        bindings.insert(
            "ctrl+alt+7".to_string(),
            KeybindAction::Builtin(Action::SwitchToProfile7),
        );
        bindings.insert(
            "ctrl+alt+8".to_string(),
            KeybindAction::Builtin(Action::SwitchToProfile8),
        );
        bindings.insert(
            "ctrl+alt+9".to_string(),
            KeybindAction::Builtin(Action::SwitchToProfile9),
        );

        KeybindConfig { bindings }
    }
}
