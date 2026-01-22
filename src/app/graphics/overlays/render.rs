use yatmux::renderer::UiStyle;

use crate::app::layout::{fill_rect, Divider, Rect};
use crate::app::App;

use super::help::render_help_overlay;
use super::prompt::render_prompt_overlay;

impl App {
    pub(crate) fn render_overlays(
        &mut self,
        buffer_width: u32,
        buffer_height: u32,
        tab_bar_height: usize,
        dividers: &[Divider],
        accent_color: u32,
        font_scale: f32,
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

        let show_help = self.show_help;

        let App {
            renderer,
            graphics,
            help_filter,
            help_scroll,
            help_max_scroll,
            shell_warning_dismissed,
            config,
            prompt,
            ..
        } = self;

        // Re-acquire buffer for dividers and overlays
        let Some(graphics) = graphics.as_mut() else {
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
        if show_help {
            render_help_overlay(
                renderer,
                help_filter,
                help_scroll,
                help_max_scroll,
                config,
                *shell_warning_dismissed,
                &mut buffer,
                buffer_width as usize,
                buffer_height as usize,
                accent_color,
                font_scale,
                ui_style,
                font_config,
                shell_integration_detected,
            );
        }

        // Render shadow prompt if visible (during command execution)
        if let Some((ref input, cursor_pos)) = shadow_prompt_info {
            renderer.paint_shadow_prompt(
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
            renderer.paint_context_menu(
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

        render_prompt_overlay(
            renderer,
            prompt.as_ref(),
            &mut buffer,
            buffer_width as usize,
            buffer_height as usize,
            ui_style,
            font_config,
        );

        // Render toast notification if visible
        if let Some(ref message) = toast_message {
            renderer.paint_toast(
                &mut buffer,
                buffer_width as usize,
                buffer_height as usize,
                message,
                ui_style,
                font_config,
            );
        }

        if let Err(e) = buffer.present() {
            eprintln!("softbuffer present failed: {e:?}");
        }
    }
}
