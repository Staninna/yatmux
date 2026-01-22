use crate::config::FontConfig;
use crate::renderer::font;
use crate::renderer::UiStyle;
use crate::renderer::Renderer;

impl Renderer {
    /// Paints the shadow prompt at the bottom of the screen.
    /// Shows a prompt indicator (e.g. "$" ) and the buffered input with cursor.
    pub fn paint_shadow_prompt(
        &mut self,
        buffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        input: &str,
        cursor_pos: usize,
        font_scale: f32,
        style: &UiStyle,
        font_config: &FontConfig,
    ) {
        let (cell_w, cell_h) = self.font_renderer.cell_size(font_config);

        let prompt_indicator = "$ ";
        let prompt_len = prompt_indicator.len();

        let max_visible_chars = (buffer_width / cell_w).saturating_sub(prompt_len + 2);
        let input_chars: Vec<char> = input.chars().collect();

        let cursor_char_pos = input[..cursor_pos.min(input.len())].chars().count();

        let (visible_start, visible_input): (usize, String) = if input_chars.len()
            <= max_visible_chars
        {
            (0, input.to_string())
        } else {
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

        let text_len = prompt_len + visible_input.chars().count() + 1;
        let padding_x = cell_w / 2;
        let padding_y = cell_h / 4;

        let prompt_width = (text_len * cell_w + padding_x * 2).min(buffer_width);
        let prompt_height = cell_h + padding_y * 2;

        let prompt_x = padding_x;
        let prompt_y = buffer_height.saturating_sub(prompt_height + cell_h / 2);

        let bg_color = style.shadow_prompt_bg;
        let fg_color = style.shadow_prompt_text;
        let cursor_color = style.shadow_prompt_cursor;
        let prompt_color = style.shadow_prompt_indicator;
        let border_color = style.shadow_prompt_border;

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

        let cursor_x = input_start_x + visible_cursor_pos * cell_w;
        if cursor_x + cell_w <= buffer_width {
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
