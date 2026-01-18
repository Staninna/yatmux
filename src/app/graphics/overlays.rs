use std::collections::HashMap;

use yatmux::config::Action;
use yatmux::renderer::{HelpSection, UiStyle};

use crate::app::App;
use crate::app::layout::{Divider, Rect, fill_rect};

impl App {
    pub(super) fn render_overlays(
        &mut self,
        buffer_width: u32,
        buffer_height: u32,
        tab_bar_height: usize,
        dividers: &[Divider],
        accent_color: u32,
        font_scale: usize,
        ui_style: &UiStyle,
        font_config: &yatmux::config::FontConfig,
    ) {
        // Calculate shell integration status before taking the graphics borrow
        let shell_integration_detected = self
            .active_tab()
            .and_then(|t| t.focused_pane())
            .map(|p| p.shell_integration.any())
            .unwrap_or(false);

        // Extract toast message before taking the graphics borrow
        let toast_message = self.current_toast().map(|s| s.to_string());

        // Extract context menu info before taking the graphics borrow
        let context_menu_info: Option<(usize, usize, Vec<(&str, usize)>)> =
            self.context_menu().map(|menu| {
                let items: Vec<(&str, usize)> = menu
                    .items
                    .iter()
                    .enumerate()
                    .map(|(i, (label, _action))| {
                        let is_hovered = if menu.hovered == Some(i) { 1 } else { 0 };
                        (*label, is_hovered)
                    })
                    .collect();
                (menu.rendered_x, menu.rendered_y, items)
            });

        // Extract shadow prompt state if visible and command is running (use cached state)
        let shadow_prompt_info: Option<(String, usize)> = {
            use yatmux::config::ShadowPromptMode;
            let mode = self.config.shell_integration.shadow_prompt;
            if mode == ShadowPromptMode::Off {
                None
            } else {
                self.active_tab()
                    .and_then(|t| t.focused_pane())
                    .and_then(|p| {
                        // Use cached command_running state instead of calling is_command_running()
                        let should_show =
                            if !p.shadow_prompt_enabled || p.terminal.is_alt_screen_active() {
                                false
                            } else {
                                match mode {
                                    ShadowPromptMode::Off => false,
                                    ShadowPromptMode::Always => p.command_running,
                                    ShadowPromptMode::OnTyping => {
                                        p.command_running && p.shadow_prompt.visible
                                    }
                                }
                            };
                        if should_show {
                            Some((p.shadow_prompt.buffer.clone(), p.shadow_prompt.cursor))
                        } else {
                            None
                        }
                    })
            }
        };

        // Re-acquire buffer for dividers and overlays
        let Some(graphics) = &mut self.graphics else {
            return;
        };
        let mut buffer = match graphics.surface.buffer_mut() {
            Ok(b) => b,
            Err(e) => {
                eprintln!("softbuffer buffer_mut failed: {e:?}");
                return;
            }
        };

        // Draw dividers between panes (offset by tab bar height)
        for d in dividers {
            fill_rect(
                &mut buffer,
                buffer_width as usize,
                buffer_height as usize,
                Rect {
                    x: d.x,
                    y: d.y + tab_bar_height,
                    w: d.w,
                    h: d.h,
                },
                ui_style.divider,
            );
        }

        // Render help overlay if visible
        if self.show_help {
            self.help_scroll = self.help_scroll.min(self.help_max_scroll);

            let mut by_category: HashMap<&'static str, Vec<(String, String)>> = HashMap::new();
            for (key, action) in &self.config.keybinds.bindings {
                // Skip disabled bindings (Action::None)
                if *action == Action::None {
                    continue;
                }
                by_category
                    .entry(action.category())
                    .or_default()
                    .push((key.clone(), action.label().to_string()));
            }

            // Consolidate numbered entries (e.g., "Go to tab 1" through "Go to tab 9")
            for bindings in by_category.values_mut() {
                Self::consolidate_numbered_bindings(bindings);
                bindings.sort_by(|a, b| a.0.cmp(&b.0));
            }

            let order = [
                "General",
                "Tabs",
                "Panes",
                "Zoom",
                "Scrollback",
                "Search",
                "Help",
            ];
            let mut sections: Vec<HelpSection> = Vec::new();

            for category in order {
                if let Some(bindings) = by_category.remove(category) {
                    sections.push(HelpSection {
                        title: category.to_string(),
                        bindings,
                    });
                }
            }

            let mut extra: Vec<(&'static str, Vec<(String, String)>)> =
                by_category.into_iter().collect();
            extra.sort_by(|a, b| a.0.cmp(b.0));
            for (category, bindings) in extra {
                sections.push(HelpSection {
                    title: category.to_string(),
                    bindings,
                });
            }

            let (scroll, max_scroll) = self.renderer.paint_help_overlay(
                &mut buffer,
                buffer_width as usize,
                buffer_height as usize,
                "Shortcuts",
                &sections,
                self.help_scroll,
                accent_color,
                font_scale,
                shell_integration_detected,
                ui_style,
                font_config,
                &self.config.ui.help,
            );
            self.help_scroll = scroll;
            self.help_max_scroll = max_scroll;
        }

        // Render shadow prompt if visible (during command execution)
        if let Some((ref input, cursor_pos)) = shadow_prompt_info {
            self.renderer.paint_shadow_prompt(
                &mut buffer,
                buffer_width as usize,
                buffer_height as usize,
                input,
                cursor_pos,
                font_scale,
                ui_style,
                font_config,
            );
        }

        // Render context menu if visible
        if let Some((menu_x, menu_y, ref items)) = context_menu_info {
            self.renderer.paint_context_menu(
                &mut buffer,
                buffer_width as usize,
                buffer_height as usize,
                menu_x,
                menu_y,
                items,
                font_scale,
                ui_style,
                font_config,
            );
        }

        // Render toast notification if visible
        if let Some(ref message) = toast_message {
            self.renderer.paint_toast(
                &mut buffer,
                buffer_width as usize,
                buffer_height as usize,
                message,
                font_scale,
                ui_style,
                font_config,
            );
        }

        if let Err(e) = buffer.present() {
            eprintln!("softbuffer present failed: {e:?}");
        }
    }

    /// Consolidates numbered keybindings like "alt+1" -> "Go to tab 1" through "alt+9" -> "Go to tab 9"
    /// into a single entry like "alt+1-9" -> "Go to tab 1-9".
    fn consolidate_numbered_bindings(bindings: &mut Vec<(String, String)>) {
        // Look for patterns like "Go to tab N" with keys like "alt+N"
        let patterns = [("Go to tab ", "alt+")];

        for (label_prefix, key_prefix) in patterns {
            // Find all matching entries
            let mut matches: Vec<(usize, char)> = Vec::new();
            for (i, (key, label)) in bindings.iter().enumerate() {
                if label.starts_with(label_prefix) && key.starts_with(key_prefix) {
                    let suffix = &label[label_prefix.len()..];
                    let key_suffix = &key[key_prefix.len()..];
                    if suffix.len() == 1 && key_suffix == suffix {
                        if let Some(digit) = suffix.chars().next() {
                            if digit.is_ascii_digit() {
                                matches.push((i, digit));
                            }
                        }
                    }
                }
            }

            // If we have multiple consecutive numbers, consolidate them
            if matches.len() >= 3 {
                matches.sort_by_key(|(_, d)| *d);
                let digits: Vec<char> = matches.iter().map(|(_, d)| *d).collect();
                let first = digits.first().unwrap();
                let last = digits.last().unwrap();

                // Remove original entries (in reverse order to preserve indices)
                let mut indices: Vec<usize> = matches.iter().map(|(i, _)| *i).collect();
                indices.sort();
                indices.reverse();
                for i in indices {
                    bindings.remove(i);
                }

                // Add consolidated entry
                bindings.push((
                    format!("{}{}-{}", key_prefix, first, last),
                    format!("{}{}-{}", label_prefix, first, last),
                ));
            }
        }
    }
}
