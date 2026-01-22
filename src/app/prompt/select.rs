use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

use super::types::{PromptResolution, PromptState, PromptUpdate};

pub(super) fn handle_confirm_prompt(prompt: &mut PromptState, event: &KeyEvent) -> PromptUpdate {
    match &event.logical_key {
        Key::Named(NamedKey::Escape) => {
            return PromptUpdate {
                resolution: Some(PromptResolution::cancel()),
                needs_redraw: false,
            };
        }
        Key::Named(NamedKey::Enter) => {
            let ok = prompt.selected == 0;
            return PromptUpdate {
                resolution: Some(PromptResolution::submit(ok, None, None)),
                needs_redraw: false,
            };
        }
        Key::Named(NamedKey::ArrowLeft) | Key::Named(NamedKey::ArrowRight) => {
            prompt.selected = if prompt.selected == 0 { 1 } else { 0 };
            return PromptUpdate {
                resolution: None,
                needs_redraw: true,
            };
        }
        Key::Named(NamedKey::Tab) => {
            prompt.selected = if prompt.selected == 0 { 1 } else { 0 };
            return PromptUpdate {
                resolution: None,
                needs_redraw: true,
            };
        }
        _ => {}
    }
    PromptUpdate::default()
}

pub(super) fn handle_pick_prompt(prompt: &mut PromptState, event: &KeyEvent) -> PromptUpdate {
    match &event.logical_key {
        Key::Named(NamedKey::Escape) => {
            return PromptUpdate {
                resolution: Some(PromptResolution::cancel()),
                needs_redraw: false,
            };
        }
        Key::Named(NamedKey::Enter) => {
            let value = prompt.items.get(prompt.selected).cloned();
            return PromptUpdate {
                resolution: Some(PromptResolution::submit(true, value, Some(prompt.selected))),
                needs_redraw: false,
            };
        }
        Key::Named(NamedKey::ArrowUp) => {
            if prompt.selected > 0 {
                prompt.selected -= 1;
                return PromptUpdate {
                    resolution: None,
                    needs_redraw: true,
                };
            }
        }
        Key::Named(NamedKey::ArrowDown) => {
            if prompt.selected + 1 < prompt.items.len() {
                prompt.selected += 1;
                return PromptUpdate {
                    resolution: None,
                    needs_redraw: true,
                };
            }
        }
        Key::Named(NamedKey::PageUp) => {
            prompt.selected = prompt.selected.saturating_sub(5);
            return PromptUpdate {
                resolution: None,
                needs_redraw: true,
            };
        }
        Key::Named(NamedKey::PageDown) => {
            prompt.selected = (prompt.selected + 5).min(prompt.items.len().saturating_sub(1));
            return PromptUpdate {
                resolution: None,
                needs_redraw: true,
            };
        }
        _ => {}
    }
    PromptUpdate::default()
}
