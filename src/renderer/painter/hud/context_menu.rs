use crate::config::FontConfig;

use crate::renderer::font;
use crate::renderer::UiStyle;
use crate::renderer::Renderer;

impl Renderer {
    /// Paints a context menu at the specified position.
    pub fn paint_context_menu(
        &mut self,
        buffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        menu_x: usize,
        menu_y: usize,
        items: &[(&str, usize)], // (label, is_hovered as 1 or 0)
        font_scale: f32,
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
}
