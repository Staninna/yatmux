//! Input state and handling for keyboard and mouse events.

use winit::dpi::PhysicalPosition;
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};

use yatmux::config::Action;
use yatmux::renderer::TerminalView;

/// Input state for mouse and keyboard.
pub struct InputState {
    pub cursor_position: PhysicalPosition<f64>,
    pub mouse_selecting: bool,
    pub modifiers: ModifiersState,
}

impl Default for InputState {
    fn default() -> Self {
        InputState {
            cursor_position: PhysicalPosition::new(0.0, 0.0),
            mouse_selecting: false,
            modifiers: ModifiersState::empty(),
        }
    }
}

/// Converts a key event into a stable key string for keybind matching.
///
/// Important: this is based on the *physical* key code where possible, so
/// `ctrl+shift+-` matches "-" (not "_") and `ctrl+shift+\\` matches "\\" (not "|").
pub fn key_event_to_string(event: &winit::event::KeyEvent) -> Option<String> {
    if let PhysicalKey::Code(code) = event.physical_key {
        let s = match code {
            // Letters
            KeyCode::KeyA => "a",
            KeyCode::KeyB => "b",
            KeyCode::KeyC => "c",
            KeyCode::KeyD => "d",
            KeyCode::KeyE => "e",
            KeyCode::KeyF => "f",
            KeyCode::KeyG => "g",
            KeyCode::KeyH => "h",
            KeyCode::KeyI => "i",
            KeyCode::KeyJ => "j",
            KeyCode::KeyK => "k",
            KeyCode::KeyL => "l",
            KeyCode::KeyM => "m",
            KeyCode::KeyN => "n",
            KeyCode::KeyO => "o",
            KeyCode::KeyP => "p",
            KeyCode::KeyQ => "q",
            KeyCode::KeyR => "r",
            KeyCode::KeyS => "s",
            KeyCode::KeyT => "t",
            KeyCode::KeyU => "u",
            KeyCode::KeyV => "v",
            KeyCode::KeyW => "w",
            KeyCode::KeyX => "x",
            KeyCode::KeyY => "y",
            KeyCode::KeyZ => "z",

            // Digits
            KeyCode::Digit0 => "0",
            KeyCode::Digit1 => "1",
            KeyCode::Digit2 => "2",
            KeyCode::Digit3 => "3",
            KeyCode::Digit4 => "4",
            KeyCode::Digit5 => "5",
            KeyCode::Digit6 => "6",
            KeyCode::Digit7 => "7",
            KeyCode::Digit8 => "8",
            KeyCode::Digit9 => "9",

            // Punctuation
            KeyCode::Minus => "-",
            KeyCode::Equal => "=",
            KeyCode::Backquote => "`",
            KeyCode::Backslash | KeyCode::IntlBackslash => "\\",
            KeyCode::Slash => "/",
            KeyCode::Comma => ",",
            KeyCode::Period => ".",
            KeyCode::Semicolon => ";",
            KeyCode::Quote => "'",
            KeyCode::BracketLeft => "[",
            KeyCode::BracketRight => "]",

            // Navigation
            KeyCode::Enter => "enter",
            KeyCode::Tab => "tab",
            KeyCode::Space => "space",
            KeyCode::Backspace => "backspace",
            KeyCode::Escape => "escape",
            KeyCode::Insert => "insert",
            KeyCode::Delete => "delete",
            KeyCode::Home => "home",
            KeyCode::End => "end",
            KeyCode::PageUp => "pageup",
            KeyCode::PageDown => "pagedown",
            KeyCode::ArrowUp => "up",
            KeyCode::ArrowDown => "down",
            KeyCode::ArrowLeft => "left",
            KeyCode::ArrowRight => "right",

            // Function keys
            KeyCode::F1 => "f1",
            KeyCode::F2 => "f2",
            KeyCode::F3 => "f3",
            KeyCode::F4 => "f4",
            KeyCode::F5 => "f5",
            KeyCode::F6 => "f6",
            KeyCode::F7 => "f7",
            KeyCode::F8 => "f8",
            KeyCode::F9 => "f9",
            KeyCode::F10 => "f10",
            KeyCode::F11 => "f11",
            KeyCode::F12 => "f12",

            _ => "",
        };

        if !s.is_empty() {
            return Some(s.to_string());
        }
    }

    // Fallback for platforms/keys without a physical keycode.
    match &event.logical_key {
        Key::Character(c) => Some(c.to_lowercase()),
        Key::Named(named) => {
            let name = match named {
                NamedKey::Enter => "enter",
                NamedKey::Tab => "tab",
                NamedKey::Space => "space",
                NamedKey::Backspace => "backspace",
                NamedKey::Escape => "escape",
                NamedKey::Insert => "insert",
                NamedKey::Delete => "delete",
                NamedKey::Home => "home",
                NamedKey::End => "end",
                NamedKey::PageUp => "pageup",
                NamedKey::PageDown => "pagedown",
                NamedKey::ArrowUp => "up",
                NamedKey::ArrowDown => "down",
                NamedKey::ArrowLeft => "left",
                NamedKey::ArrowRight => "right",
                NamedKey::F1 => "f1",
                NamedKey::F2 => "f2",
                NamedKey::F3 => "f3",
                NamedKey::F4 => "f4",
                NamedKey::F5 => "f5",
                NamedKey::F6 => "f6",
                NamedKey::F7 => "f7",
                NamedKey::F8 => "f8",
                NamedKey::F9 => "f9",
                NamedKey::F10 => "f10",
                NamedKey::F11 => "f11",
                NamedKey::F12 => "f12",
                _ => return None,
            };
            Some(name.to_string())
        }
        _ => None,
    }
}

/// Applies search input to the terminal view. Returns true if a redraw is needed.
pub fn apply_search_input(
    view: &mut TerminalView,
    modifiers: ModifiersState,
    action: Option<Action>,
    event: &winit::event::KeyEvent,
) -> bool {
    if let Some(action) = action {
        match action {
            Action::SearchClose => {
                view.deactivate_search();
                return true;
            }
            Action::SearchConfirm => {
                if view.search_match_count() > 0 {
                    view.search_next();
                }
                return true;
            }
            Action::SearchNext => {
                view.search_next();
                return true;
            }
            Action::SearchPrev => {
                view.search_prev();
                return true;
            }
            Action::SearchToggleCase => {
                view.search_toggle_case();
                return true;
            }
            Action::SearchToggleRegex => {
                view.search_toggle_regex();
                return true;
            }
            _ => {}
        }
    }

    match &event.logical_key {
        Key::Named(NamedKey::Backspace) => {
            view.search_pop_char();
            true
        }
        Key::Character(s) => {
            if !modifiers.control_key() && !modifiers.alt_key() {
                for ch in s.chars() {
                    view.search_push_char(ch);
                }
                true
            } else {
                false
            }
        }
        _ => false,
    }
}
