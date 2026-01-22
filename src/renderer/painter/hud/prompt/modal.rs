use crate::config::FontConfig;
use crate::renderer::font;
use crate::renderer::UiStyle;
use crate::renderer::Renderer;

impl Renderer {
    /// Paints a simple modal prompt overlay.
    pub fn paint_prompt(
        &mut self,
        buffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        title: &str,
        message: Option<&str>,
        input: Option<&str>,
        items: &[String],
        selected: usize,
        ok_label: &str,
        cancel_label: &str,
        style: &UiStyle,
        font_config: &FontConfig,
    ) {
        let (cell_w, cell_h) = self.font_renderer.cell_size(font_config);
        let cell_w = cell_w.max(1);
        let cell_h = cell_h.max(1);

        let mut lines: Vec<String> = Vec::new();
        let mut selected_line_offset = None;
        lines.push(title.to_string());
        if let Some(message) = message {
            for line in message.lines() {
                lines.push(line.to_string());
            }
        }
        if let Some(input) = input {
            lines.push(format!("> {input}"));
        }

        if !items.is_empty() {
            let max_items = 8usize.min(items.len());
            let start = if selected >= max_items {
                selected.saturating_sub(max_items - 1)
            } else {
                0
            };
            for (idx, item) in items.iter().enumerate().skip(start).take(max_items) {
                if idx == selected {
                    selected_line_offset = Some(lines.len());
                }
                let prefix = if idx == selected { "* " } else { "  " };
                lines.push(format!("{prefix}{item}"));
            }
            if items.len() > max_items {
                lines.push("...".to_string());
            }
        }

        let buttons = format!("[{ok_label}]  [{cancel_label}]");
        lines.push(buttons);

        let max_len = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        let padding_x = cell_w;
        let padding_y = cell_h / 2;
        let box_width = max_len * cell_w + padding_x * 2;
        let box_height = lines.len() * cell_h + padding_y * 2;

        let box_x = (buffer_width.saturating_sub(box_width)) / 2;
        let box_y = (buffer_height.saturating_sub(box_height)) / 2;

        let bg = style.help_bg;
        let fg = style.help_text;
        let border = style.context_menu_border;
        let highlight_bg = style.context_menu_hover_bg;
        let highlight_fg = style.context_menu_text;

        for y in box_y..box_y + box_height {
            if y >= buffer_height {
                continue;
            }
            for x in box_x..box_x + box_width {
                if x >= buffer_width {
                    continue;
                }
                buffer[y * buffer_width + x] = bg;
            }
        }

        for x in box_x..box_x + box_width {
            if x < buffer_width {
                if box_y < buffer_height {
                    buffer[box_y * buffer_width + x] = border;
                }
                if box_y + box_height - 1 < buffer_height {
                    buffer[(box_y + box_height - 1) * buffer_width + x] = border;
                }
            }
        }
        for y in box_y..box_y + box_height {
            if y < buffer_height {
                if box_x < buffer_width {
                    buffer[y * buffer_width + box_x] = border;
                }
                if box_x + box_width - 1 < buffer_width {
                    buffer[y * buffer_width + box_x + box_width - 1] = border;
                }
            }
        }

        let mut y = box_y + padding_y;
        for (i, line) in lines.iter().enumerate() {
            let mut line_x = box_x + padding_x;
            let mut line_fg = fg;
            let mut line_bg = None;

            if let Some(selected_line) = selected_line_offset {
                if i == selected_line {
                    line_bg = Some(highlight_bg);
                    line_fg = highlight_fg;
                }
            }

            if let Some(bg_color) = line_bg {
                let line_width = line.chars().count() * cell_w;
                for yy in y..y + cell_h {
                    if yy >= buffer_height {
                        continue;
                    }
                    for xx in line_x..line_x + line_width {
                        if xx >= buffer_width {
                            continue;
                        }
                        buffer[yy * buffer_width + xx] = bg_color;
                    }
                }
            }

            for ch in line.chars() {
                if line_x + cell_w > buffer_width {
                    break;
                }
                if let Ok(Some(tt_glyph)) = self.font_renderer.get_glyph(ch, font_config) {
                    let baseline_offset = self.font_renderer.baseline_offset(font_config);
                    let glyph_y = y
                        .saturating_add(baseline_offset as usize)
                        .saturating_sub(tt_glyph.bearing_y as usize);
                    self.draw_native_glyph(
                        buffer,
                        buffer_width,
                        buffer_height,
                        line_x,
                        glyph_y,
                        &tt_glyph.pixels,
                        tt_glyph.width,
                        tt_glyph.height,
                        line_fg,
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
                        font_config.scale,
                        line_x,
                        y,
                        glyph,
                        line_fg,
                    );
                }
                line_x += cell_w;
            }
            y += cell_h;
        }
    }
}
