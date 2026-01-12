use yatmux::renderer::UiStyle;

use crate::app::App;

impl App {
    /// Renders the tab bar at the top of the window (static method to avoid borrow issues).
    pub(super) fn render_tab_bar_static(
        buffer: &mut [u32],
        buffer_width: usize,
        tab_bar_height: usize,
        tabs: &[(String, bool)],
        bg_color: u32,
        accent_color: u32,
        font_scale: usize,
        style: &UiStyle,
    ) {
        let scale = font_scale.clamp(1, 8);
        let cell_w = 8 * scale;
        let cell_h = 8 * scale;

        // Background
        let tab_bar_bg = style.tab_bar_bg;
        for y in 0..tab_bar_height {
            let row = y * buffer_width;
            for x in 0..buffer_width {
                buffer[row + x] = tab_bar_bg;
            }
        }

        // Bottom border
        let border_y = tab_bar_height.saturating_sub(1);
        for x in 0..buffer_width {
            buffer[border_y * buffer_width + x] = style.tab_bar_border;
        }

        // Calculate tab dimensions to share total space
        let num_tabs = tabs.len();
        if num_tabs == 0 {
            return;
        }

        let tab_gap = style.tab_gap_px;
        let side_padding = style.tab_side_padding_px;
        let total_gap_width = tab_gap * (num_tabs.saturating_sub(1)) + side_padding * 2;
        let available_width = buffer_width.saturating_sub(total_gap_width);
        let tab_width = (available_width / num_tabs)
            .min(cell_w * style.tab_max_width_cells + style.tab_max_width_px_extra);
        let max_title_chars = (tab_width.saturating_sub(16)) / cell_w; // Account for padding

        let mut x_offset = side_padding;

        for (idx, (title, is_active)) in tabs.iter().enumerate() {
            // Tab background
            let tab_bg = if *is_active {
                bg_color
            } else {
                style.tab_inactive_bg
            };

            let tab_x0 = x_offset;
            let tab_x1 = if idx == num_tabs - 1 {
                // Last tab extends to fill remaining space (minus padding)
                (x_offset + tab_width).min(buffer_width.saturating_sub(side_padding))
            } else {
                (x_offset + tab_width).min(buffer_width)
            };
            let tab_y0 = 2;
            let tab_y1 = tab_bar_height.saturating_sub(1);

            for y in tab_y0..tab_y1 {
                let row = y * buffer_width;
                for x in tab_x0..tab_x1 {
                    buffer[row + x] = tab_bg;
                }
            }

            // Active tab indicator (accent color on top and both sides)
            if *is_active {
                // Top accent line
                for x in tab_x0..tab_x1 {
                    buffer[tab_y0 * buffer_width + x] = accent_color;
                }
                // Left side accent line
                for y in tab_y0..tab_y1 {
                    buffer[y * buffer_width + tab_x0] = accent_color;
                }
                // Right side accent line
                let right_x = tab_x1.saturating_sub(1);
                for y in tab_y0..tab_y1 {
                    buffer[y * buffer_width + right_x] = accent_color;
                }
            }

            // Tab title (centered in tab)
            let display_title: String = title.chars().take(max_title_chars).collect();
            let text_color = if *is_active {
                style.base_fg
            } else {
                style.tab_inactive_text
            };
            let title_pixel_width = display_title.chars().count() * cell_w;
            let tab_content_width = tab_x1.saturating_sub(tab_x0);
            let text_x = tab_x0 + (tab_content_width.saturating_sub(title_pixel_width)) / 2;
            let text_y = (tab_bar_height - cell_h) / 2;

            Self::draw_text_static(
                buffer,
                buffer_width,
                text_x,
                text_y,
                &display_title,
                text_color,
                scale,
            );

            x_offset = tab_x1 + tab_gap;

            if x_offset >= buffer_width {
                break;
            }
        }
    }

    /// Draws text at the given position using the bitmap font.
    fn draw_text_static(
        buffer: &mut [u32],
        buffer_width: usize,
        x: usize,
        y: usize,
        text: &str,
        color: u32,
        scale: usize,
    ) {
        let cell_w = 8 * scale;

        let mut char_x = x;
        for ch in text.chars() {
            let glyph = yatmux::renderer::font::get_glyph(ch);
            for gy in 0..8 {
                let bits = glyph[gy];
                for gx in 0..8 {
                    if (bits >> gx) & 1 == 1 {
                        for sy in 0..scale {
                            for sx in 0..scale {
                                let px = char_x + gx * scale + sx;
                                let py = y + gy * scale + sy;
                                if px < buffer_width {
                                    buffer[py * buffer_width + px] = color;
                                }
                            }
                        }
                    }
                }
            }
            char_x += cell_w;
        }
    }
}
