use super::super::*;

impl App {
    pub(super) fn handle_shadow_prompt_input(
        pane: &mut Pane,
        key: &Key,
        modifiers: winit::keyboard::ModifiersState,
    ) -> bool {
        let ctrl = modifiers.control_key();
        let alt = modifiers.alt_key();

        match key {
            Key::Named(NamedKey::Backspace) => {
                pane.shadow_prompt.backspace();
                true
            }
            Key::Named(NamedKey::Delete) => {
                pane.shadow_prompt.delete();
                true
            }
            Key::Named(NamedKey::ArrowLeft) => {
                pane.shadow_prompt.move_left();
                true
            }
            Key::Named(NamedKey::ArrowRight) => {
                pane.shadow_prompt.move_right();
                true
            }
            Key::Named(NamedKey::Home) => {
                pane.shadow_prompt.move_home();
                true
            }
            Key::Named(NamedKey::End) => {
                pane.shadow_prompt.move_end();
                true
            }
            Key::Named(NamedKey::Escape) => {
                // Clear shadow prompt on Escape
                pane.shadow_prompt.clear();
                true
            }
            Key::Named(NamedKey::Enter) => {
                // Add newline to buffer (for multi-line commands)
                pane.shadow_prompt.insert('\n');
                true
            }
            Key::Named(NamedKey::Space) => {
                if !ctrl && !alt {
                    pane.shadow_prompt.insert(' ');
                    true
                } else {
                    false
                }
            }
            Key::Character(s) => {
                // Regular text input (when not ctrl/alt modified)
                if !ctrl && !alt {
                    pane.shadow_prompt.insert_str(s);
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
