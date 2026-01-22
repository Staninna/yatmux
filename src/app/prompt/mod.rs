use serde_json::json;
use winit::event::KeyEvent;
use winit::keyboard::ModifiersState;

mod input;
mod select;
mod types;

pub use types::{PromptKind, PromptState, PromptUpdate};

use input::handle_input_prompt;
use select::{handle_confirm_prompt, handle_pick_prompt};

use super::App;

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
