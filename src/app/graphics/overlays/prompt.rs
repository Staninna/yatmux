use yatmux::renderer::UiStyle;

use crate::app::{PromptKind, PromptState};
use yatmux::renderer::Renderer;

pub(super) fn render_prompt_overlay(
    renderer: &mut Renderer,
    prompt: Option<&PromptState>,
    buffer: &mut [u32],
    buffer_width: usize,
    buffer_height: usize,
    ui_style: &UiStyle,
    font_config: &yatmux::config::FontConfig,
) {
    if let Some(prompt) = prompt {
        let input = match prompt.kind {
            PromptKind::Input => Some(prompt.input.as_str()),
            _ => None,
        };
        renderer.paint_prompt(
            buffer,
            buffer_width,
            buffer_height,
            &prompt.title,
            prompt.message.as_deref(),
            input,
            &prompt.items,
            prompt.selected,
            &prompt.ok_label,
            &prompt.cancel_label,
            ui_style,
            font_config,
        );
    }
}
