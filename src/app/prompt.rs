use serde_json::json;
use winit::event::KeyEvent;
use winit::keyboard::ModifiersState;
use winit::keyboard::{Key, NamedKey};

use super::App;

#[derive(Debug, Clone)]
pub enum PromptKind {
    Input,
    Confirm,
    Pick,
}

#[derive(Debug, Clone)]
pub struct PromptState {
    pub id: String,
    pub title: String,
    pub message: Option<String>,
    pub kind: PromptKind,
    pub input: String,
    pub default_value: Option<String>,
    pub items: Vec<String>,
    pub selected: usize,
    pub ok_label: String,
    pub cancel_label: String,
}

pub struct PromptResolution {
    pub ok: bool,
    pub value: Option<String>,
    pub index: Option<usize>,
    pub reason: Option<String>,
}

#[derive(Default)]
pub struct PromptUpdate {
    pub resolution: Option<PromptResolution>,
    pub needs_redraw: bool,
}

impl PromptResolution {
    pub fn submit(ok: bool, value: Option<String>, index: Option<usize>) -> Self {
        Self {
            ok,
            value,
            index,
            reason: None,
        }
    }

    pub fn cancel() -> Self {
        Self {
            ok: false,
            value: None,
            index: None,
            reason: None,
        }
    }
}

impl PromptState {
    pub fn input(
        id: String,
        title: String,
        message: Option<String>,
        default_value: Option<String>,
    ) -> Self {
        Self {
            id,
            title,
            message,
            kind: PromptKind::Input,
            input: String::new(),
            default_value,
            items: Vec::new(),
            selected: 0,
            ok_label: "OK".to_string(),
            cancel_label: "Cancel".to_string(),
        }
    }

    pub fn confirm(
        id: String,
        title: String,
        message: Option<String>,
        ok_label: Option<String>,
        cancel_label: Option<String>,
    ) -> Self {
        Self {
            id,
            title,
            message,
            kind: PromptKind::Confirm,
            input: String::new(),
            default_value: None,
            items: Vec::new(),
            selected: 0,
            ok_label: ok_label.unwrap_or_else(|| "OK".to_string()),
            cancel_label: cancel_label.unwrap_or_else(|| "Cancel".to_string()),
        }
    }

    pub fn pick(
        id: String,
        title: String,
        message: Option<String>,
        items: Vec<String>,
        selected: Option<usize>,
    ) -> Self {
        let selected = selected.unwrap_or(0).min(items.len().saturating_sub(1));
        Self {
            id,
            title,
            message,
            kind: PromptKind::Pick,
            input: String::new(),
            default_value: None,
            items,
            selected,
            ok_label: "Select".to_string(),
            cancel_label: "Cancel".to_string(),
        }
    }

    pub fn kind_label(&self) -> &'static str {
        match self.kind {
            PromptKind::Input => "input",
            PromptKind::Confirm => "confirm",
            PromptKind::Pick => "pick",
        }
    }
}

impl App {
    pub(super) fn open_prompt(&mut self, prompt: PromptState) {
        if self.prompt.is_some() {
            self.finish_prompt(false, None, None, Some("replaced".to_string()));
        }
        self.prompt = Some(prompt);
        self.request_redraw();
    }

    pub(super) fn handle_prompt_input(
        prompt: &mut PromptState,
        event: &KeyEvent,
        modifiers: ModifiersState,
    ) -> PromptUpdate {
        if event.state != winit::event::ElementState::Pressed {
            return PromptUpdate::default();
        }

        match prompt.kind {
            PromptKind::Input => handle_input_prompt(prompt, event, modifiers),
            PromptKind::Confirm => handle_confirm_prompt(prompt, event),
            PromptKind::Pick => handle_pick_prompt(prompt, event),
        }
    }

    pub(super) fn finish_prompt(
        &mut self,
        ok: bool,
        value: Option<String>,
        index: Option<usize>,
        reason: Option<String>,
    ) {
        let prompt = self.prompt.take();
        let Some(prompt) = prompt else {
            return;
        };

        let mut data = json!({
            "id": prompt.id,
            "ok": ok,
            "value": value,
            "index": index,
            "kind": prompt.kind_label(),
        });
        if let Some(reason) = reason {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("reason".to_string(), json!(reason));
            }
        }

        self.dispatch_prompt_response(data);
        self.request_redraw();
    }
}

fn handle_input_prompt(
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

fn handle_confirm_prompt(prompt: &mut PromptState, event: &KeyEvent) -> PromptUpdate {
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

fn handle_pick_prompt(prompt: &mut PromptState, event: &KeyEvent) -> PromptUpdate {
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
