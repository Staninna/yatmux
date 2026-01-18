use crate::config::FontConfig;

use super::Renderer;

use super::super::UiStyle;
use super::super::font;

impl Renderer {
    /// Paints a small toast notification at the bottom-center of the screen.
    pub fn paint_toast(
        &mut self,
        buffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        message: &str,
        font_scale: usize,
        style: &UiStyle,
        font_config: &FontConfig,
    ) {
        let (cell_w, cell_h) = self.font_renderer.cell_size(font_config);

        let text_len = message.chars().count();
        let padding_x = cell_w;
        let padding_y = cell_h / 2;

        let toast_width = text_len * cell_w + padding_x * 2;
        let toast_height = cell_h + padding_y * 2;

        // Position: bottom-center, slightly above the bottom edge
        let toast_x = (buffer_width.saturating_sub(toast_width)) / 2;
        let toast_y =
            buffer_height.saturating_sub(toast_height + cell_h * style.toast_bottom_margin_cells);

        let bg_color = style.toast_bg;
        let fg_color = style.toast_text;
        let border_color = style.toast_border;

        // Draw background
        for y in toast_y..toast_y + toast_height {
            if y >= buffer_height {
                continue;
            }
            for x in toast_x..toast_x + toast_width {
                if x >= buffer_width {
                    continue;
                }
                buffer[y * buffer_width + x] = bg_color;
            }
        }

        // Draw border (single pixel)
        for x in toast_x..toast_x + toast_width {
            if x < buffer_width {
                if toast_y > 0 && toast_y - 1 < buffer_height {
                    buffer[(toast_y) * buffer_width + x] = border_color;
                }
                if toast_y + toast_height - 1 < buffer_height {
                    buffer[(toast_y + toast_height - 1) * buffer_width + x] = border_color;
                }
            }
        }
        for y in toast_y..toast_y + toast_height {
            if y < buffer_height {
                if toast_x < buffer_width {
                    buffer[y * buffer_width + toast_x] = border_color;
                }
                if toast_x + toast_width - 1 < buffer_width {
                    buffer[y * buffer_width + toast_x + toast_width - 1] = border_color;
                }
            }
        }

        // Draw text
        let text_x = toast_x + padding_x;
        let text_y = toast_y + padding_y;

        for (i, ch) in message.chars().enumerate() {
            let x = text_x + i * cell_w;
            if x + cell_w > buffer_width {
                break;
            }
            if let Ok(Some(tt_glyph)) = self.font_renderer.get_glyph(ch, font_config) {
                let baseline_offset = self.font_renderer.baseline_offset(font_config);
                let glyph_y = text_y
                    .saturating_add(baseline_offset as usize)
                    .saturating_sub(tt_glyph.bearing_y as usize);
                self.draw_native_glyph(
                    buffer,
                    buffer_width,
                    buffer_height,
                    x,
                    glyph_y,
                    &tt_glyph.pixels,
                    tt_glyph.width,
                    tt_glyph.height,
                    fg_color,
                );
            } else {
                let glyph = font::get_bitmap_glyph(ch);
                self.draw_glyph(
                    buffer,
                    buffer_width,
                    buffer_height,
                    0,
                    0,
                    buffer_width,
                    buffer_height,
                    font_scale,
                    x,
                    text_y,
                    glyph,
                    fg_color,
                );
            }
        }
    }

    /// Paints a context menu at the specified position.
    pub fn paint_context_menu(
        &mut self,
        buffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        menu_x: usize,
        menu_y: usize,
        items: &[(&str, usize)], // (label, is_hovered as 1 or 0)
        font_scale: usize,
        style: &UiStyle,
        font_config: &FontConfig,
    ) {
        if items.is_empty() {
            return;
        }

        let (cell_w, cell_h) = self.font_renderer.cell_size(font_config);
        let padding_x = cell_w;
        let padding_y = cell_h / 4;
        let item_height = cell_h + padding_y * 2;

        // Find widest item
        let max_label_len = items
            .iter()
            .map(|(label, _)| label.len())
            .max()
            .unwrap_or(8);
        let menu_width = max_label_len * cell_w + padding_x * 2;
        let menu_height = items.len() * item_height;

        // Position is already adjusted for screen boundaries during menu creation
        // No need to adjust here

        let bg_color = style.context_menu_bg;
        let hover_bg = style.context_menu_hover_bg;
        let fg_color = style.context_menu_text;
        let border_color = style.context_menu_border;

        // Draw background
        for y in menu_y..menu_y + menu_height {
            if y >= buffer_height {
                continue;
            }
            for x in menu_x..menu_x + menu_width {
                if x >= buffer_width {
                    continue;
                }
                buffer[y * buffer_width + x] = bg_color;
            }
        }

        // Draw border
        for x in menu_x..menu_x + menu_width {
            if x < buffer_width {
                if menu_y < buffer_height {
                    buffer[menu_y * buffer_width + x] = border_color;
                }
                if menu_y + menu_height > 0 && menu_y + menu_height - 1 < buffer_height {
                    buffer[(menu_y + menu_height - 1) * buffer_width + x] = border_color;
                }
            }
        }
        for y in menu_y..menu_y + menu_height {
            if y < buffer_height {
                if menu_x < buffer_width {
                    buffer[y * buffer_width + menu_x] = border_color;
                }
                if menu_x + menu_width > 0 && menu_x + menu_width - 1 < buffer_width {
                    buffer[y * buffer_width + menu_x + menu_width - 1] = border_color;
                }
            }
        }

        // Draw each menu item
        for (i, (label, is_hovered)) in items.iter().enumerate() {
            let item_y = menu_y + i * item_height;

            // Draw hover highlight
            if *is_hovered == 1 {
                for y in item_y..item_y + item_height {
                    if y >= buffer_height {
                        continue;
                    }
                    for x in (menu_x + 1)..(menu_x + menu_width - 1) {
                        if x >= buffer_width {
                            continue;
                        }
                        buffer[y * buffer_width + x] = hover_bg;
                    }
                }
            }

            // Draw label
            let text_x = menu_x + padding_x;
            let text_y = item_y + padding_y;

            for (j, ch) in label.chars().enumerate() {
                let x = text_x + j * cell_w;
                if x + cell_w > buffer_width {
                    break;
                }
                if let Ok(Some(tt_glyph)) = self.font_renderer.get_glyph(ch, font_config) {
                    let baseline_offset = self.font_renderer.baseline_offset(font_config);
                    let glyph_y = text_y
                        .saturating_add(baseline_offset as usize)
                        .saturating_sub(tt_glyph.bearing_y as usize);
                    self.draw_native_glyph(
                        buffer,
                        buffer_width,
                        buffer_height,
                        x,
                        glyph_y,
                        &tt_glyph.pixels,
                        tt_glyph.width,
                        tt_glyph.height,
                        fg_color,
                    );
                } else {
                    let glyph = font::get_bitmap_glyph(ch);
                    self.draw_glyph(
                        buffer,
                        buffer_width,
                        buffer_height,
                        0,
                        0,
                        buffer_width,
                        buffer_height,
                        font_scale,
                        x,
                        text_y,
                        glyph,
                        fg_color,
                    );
                }
            }
        }
    }

    /// Paints the shadow prompt at the bottom of the screen.
    /// Shows a prompt indicator (e.g. "$") and the buffered input with cursor.
    pub fn paint_shadow_prompt(
        &mut self,
        buffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        input: &str,
        cursor_pos: usize,
        font_scale: usize,
        style: &UiStyle,
        font_config: &FontConfig,
    ) {
        let (cell_w, cell_h) = self.font_renderer.cell_size(font_config);

        // Prompt indicator
        let prompt_indicator = "$ ";
        let prompt_len = prompt_indicator.len();

        // Calculate visible portion of input (handle long input)
        let max_visible_chars = (buffer_width / cell_w).saturating_sub(prompt_len + 2);
        let input_chars: Vec<char> = input.chars().collect();

        // Calculate cursor position in chars
        let cursor_char_pos = input[..cursor_pos.min(input.len())].chars().count();

        // Calculate visible window around cursor
        let (visible_start, visible_input): (usize, String) =
            if input_chars.len() <= max_visible_chars {
                (0, input.to_string())
            } else {
                // Keep cursor visible, preferably in the middle
                let half_visible = max_visible_chars / 2;
                let start = if cursor_char_pos < half_visible {
                    0
                } else if cursor_char_pos > input_chars.len().saturating_sub(half_visible) {
                    input_chars.len().saturating_sub(max_visible_chars)
                } else {
                    cursor_char_pos.saturating_sub(half_visible)
                };
                let end = (start + max_visible_chars).min(input_chars.len());
                (start, input_chars[start..end].iter().collect())
            };

        let visible_cursor_pos = cursor_char_pos.saturating_sub(visible_start);

        let text_len = prompt_len + visible_input.chars().count() + 1; // +1 for cursor
        let padding_x = cell_w / 2;
        let padding_y = cell_h / 4;

        let prompt_width = (text_len * cell_w + padding_x * 2).min(buffer_width);
        let prompt_height = cell_h + padding_y * 2;

        // Position: bottom-left, at the bottom edge
        let prompt_x = padding_x;
        let prompt_y = buffer_height.saturating_sub(prompt_height + cell_h / 2);

        let bg_color = style.shadow_prompt_bg;
        let fg_color = style.shadow_prompt_text;
        let cursor_color = style.shadow_prompt_cursor;
        let prompt_color = style.shadow_prompt_indicator;
        let border_color = style.shadow_prompt_border;

        // Draw background
        for y in prompt_y..prompt_y + prompt_height {
            if y >= buffer_height {
                continue;
            }
            for x in prompt_x..prompt_x + prompt_width {
                if x >= buffer_width {
                    continue;
                }
                buffer[y * buffer_width + x] = bg_color;
            }
        }

        // Draw border
        for x in prompt_x..prompt_x + prompt_width {
            if x < buffer_width {
                if prompt_y < buffer_height {
                    buffer[prompt_y * buffer_width + x] = border_color;
                }
                if prompt_y + prompt_height - 1 < buffer_height {
                    buffer[(prompt_y + prompt_height - 1) * buffer_width + x] = border_color;
                }
            }
        }
        for y in prompt_y..prompt_y + prompt_height {
            if y < buffer_height {
                if prompt_x < buffer_width {
                    buffer[y * buffer_width + prompt_x] = border_color;
                }
                if prompt_x + prompt_width - 1 < buffer_width {
                    buffer[y * buffer_width + prompt_x + prompt_width - 1] = border_color;
                }
            }
        }

        // Draw prompt indicator
        let text_x = prompt_x + padding_x;
        let text_y = prompt_y + padding_y;

        for (i, ch) in prompt_indicator.chars().enumerate() {
            let x = text_x + i * cell_w;
            if x + cell_w > buffer_width {
                break;
            }
            if let Ok(Some(tt_glyph)) = self.font_renderer.get_glyph(ch, font_config) {
                let baseline_offset = self.font_renderer.baseline_offset(font_config);
                let glyph_y = text_y
                    .saturating_add(baseline_offset as usize)
                    .saturating_sub(tt_glyph.bearing_y as usize);
                self.draw_native_glyph(
                    buffer,
                    buffer_width,
                    buffer_height,
                    x,
                    glyph_y,
                    &tt_glyph.pixels,
                    tt_glyph.width,
                    tt_glyph.height,
                    prompt_color,
                );
            } else {
                let glyph = font::get_bitmap_glyph(ch);
                self.draw_glyph(
                    buffer,
                    buffer_width,
                    buffer_height,
                    0,
                    0,
                    buffer_width,
                    buffer_height,
                    font_scale,
                    x,
                    text_y,
                    glyph,
                    prompt_color,
                );
            }
        }

        // Draw input text
        let input_start_x = text_x + prompt_len * cell_w;
        for (i, ch) in visible_input.chars().enumerate() {
            let x = input_start_x + i * cell_w;
            if x + cell_w > buffer_width {
                break;
            }
            if let Ok(Some(tt_glyph)) = self.font_renderer.get_glyph(ch, font_config) {
                let baseline_offset = self.font_renderer.baseline_offset(font_config);
                let glyph_y = text_y
                    .saturating_add(baseline_offset as usize)
                    .saturating_sub(tt_glyph.bearing_y as usize);
                self.draw_native_glyph(
                    buffer,
                    buffer_width,
                    buffer_height,
                    x,
                    glyph_y,
                    &tt_glyph.pixels,
                    tt_glyph.width,
                    tt_glyph.height,
                    fg_color,
                );
            } else {
                let glyph = font::get_bitmap_glyph(ch);
                self.draw_glyph(
                    buffer,
                    buffer_width,
                    buffer_height,
                    0,
                    0,
                    buffer_width,
                    buffer_height,
                    font_scale,
                    x,
                    text_y,
                    glyph,
                    fg_color,
                );
            }
        }

        // Draw cursor
        let cursor_x = input_start_x + visible_cursor_pos * cell_w;
        if cursor_x + cell_w <= buffer_width {
            // Draw a block cursor
            for y in text_y..text_y + cell_h {
                if y >= buffer_height {
                    continue;
                }
                for x in cursor_x..cursor_x + cell_w {
                    if x >= buffer_width {
                        continue;
                    }
                    buffer[y * buffer_width + x] = cursor_color;
                }
            }

            // Draw the character at cursor position in contrasting color (if any)
            if visible_cursor_pos < visible_input.chars().count() {
                let cursor_char = visible_input.chars().nth(visible_cursor_pos).unwrap_or(' ');
                if let Ok(Some(tt_glyph)) = self.font_renderer.get_glyph(cursor_char, font_config) {
                    let baseline_offset = self.font_renderer.baseline_offset(font_config);
                    let glyph_y = text_y
                        .saturating_add(baseline_offset as usize)
                        .saturating_sub(tt_glyph.bearing_y as usize);
                    self.draw_native_glyph(
                        buffer,
                        buffer_width,
                        buffer_height,
                        cursor_x,
                        glyph_y,
                        &tt_glyph.pixels,
                        tt_glyph.width,
                        tt_glyph.height,
                        bg_color,
                    );
                } else {
                    let glyph = font::get_bitmap_glyph(cursor_char);
                    self.draw_glyph(
                        buffer,
                        buffer_width,
                        buffer_height,
                        0,
                        0,
                        buffer_width,
                        buffer_height,
                        font_scale,
                        cursor_x,
                        text_y,
                        glyph,
                        bg_color,
                    );
                }
            }
        }
    }
}
