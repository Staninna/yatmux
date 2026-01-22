use super::super::*;

impl App {
    pub(super) fn handle_help_overlay_input(
        &mut self,
        key_str: Option<&str>,
        event: &winit::event::KeyEvent,
        modifiers: winit::keyboard::ModifiersState,
    ) -> bool {
        if !self.show_help {
            return false;
        }

        let ctrl = modifiers.control_key();
        let alt = modifiers.alt_key();

        if let Some(key) = key_str {
            match key {
                "escape" => {
                    if self.help_filter.is_active() {
                        self.help_filter.deactivate();
                        self.help_scroll = 0;
                        self.request_redraw();
                        return true;
                    }
                }
                "backspace" => {
                    if self.help_filter.is_active() {
                        self.help_filter.pop_char();
                        self.help_scroll = 0;
                        self.request_redraw();
                        return true;
                    }
                }
                "up" => {
                    self.help_scroll = self.help_scroll.saturating_sub(1);
                    self.request_redraw();
                    return true;
                }
                "down" => {
                    self.help_scroll = (self.help_scroll + 1).min(self.help_max_scroll);
                    self.request_redraw();
                    return true;
                }
                "pageup" => {
                    self.help_scroll = self.help_scroll.saturating_sub(10);
                    self.request_redraw();
                    return true;
                }
                "pagedown" => {
                    self.help_scroll = (self.help_scroll + 10).min(self.help_max_scroll);
                    self.request_redraw();
                    return true;
                }
                "home" => {
                    self.help_scroll = 0;
                    self.request_redraw();
                    return true;
                }
                "end" => {
                    self.help_scroll = self.help_max_scroll;
                    self.request_redraw();
                    return true;
                }
                "d" if !ctrl && !alt && !self.help_filter.is_active() => {
                    self.shell_warning_dismissed = true;
                    self.request_redraw();
                    return true;
                }
                _ => {}
            }
        }

        // Any other character input activates filter and adds to query
        if let Some(text) = &event.text {
            if !text.is_empty() && !ctrl && !alt {
                for ch in text.chars() {
                    if !ch.is_control() {
                        self.help_filter.activate();
                        self.help_filter.push_char(ch);
                    }
                }
                self.help_scroll = 0;
                self.request_redraw();
                return true;
            }
        }

        false
    }
}
