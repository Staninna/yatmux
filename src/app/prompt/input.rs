use winit::event::KeyEvent;
use winit::keyboard::ModifiersState;
use winit::keyboard::{Key, NamedKey};

use super::types::{PromptResolution, PromptState, PromptUpdate};

pub(super) fn handle_input_prompt(
    prompt: &mut PromptState,
    event: &KeyEvent,
    modifiers: ModifiersState,
) -> PromptUpdate {
    match &event.logical_key {
        Key::Named(NamedKey::Escape) => {
            return PromptUpdate {
                resolution: Some(PromptResolution::cancel()),
                needs_redraw: false,
            };
        }
        Key::Named(NamedKey::Enter) => {
            let value = if prompt.input.is_empty() {
                prompt.default_value.clone().unwrap_or_default()
            } else {
                prompt.input.clone()
            };
            return PromptUpdate {
                resolution: Some(PromptResolution::submit(true, Some(value), None)),
                needs_redraw: false,
            };
        }
        Key::Named(NamedKey::Backspace) => {
            prompt.input.pop();
            return PromptUpdate {
                resolution: None,
                needs_redraw: true,
            };
        }
        Key::Named(NamedKey::Tab) => {
            if let Some(default) = prompt.default_value.as_ref() {
                if prompt.input.is_empty() {
                    prompt.input = default.clone();
                    return PromptUpdate {
                        resolution: None,
                        needs_redraw: true,
                    };
                }
            }
        }
        Key::Character(s) => {
            if !modifiers.control_key() && !modifiers.alt_key() {
                for ch in s.chars() {
                    if !ch.is_control() {
                        prompt.input.push(ch);
                    }
                }
                return PromptUpdate {
                    resolution: None,
                    needs_redraw: true,
                };
            }
        }
        _ => {}
    }
    PromptUpdate::default()
}
