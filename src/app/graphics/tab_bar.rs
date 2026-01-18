use yatmux::config::FontConfig;
use yatmux::renderer::{UiStyle, font::get_bitmap_glyph};

use crate::app::App;

use yatmux::renderer::font::FontRenderer;

impl App {
    /// Renders the tab bar at the top of the window (static method to avoid borrow issues).
    pub(super) fn render_tab_bar_static(
        buffer: &mut [u32],
        buffer_width: usize,
        tab_bar_height: usize,
        tabs: &[(String, bool)],
        bg_color: u32,
        accent_color: u32,
        style: &UiStyle,
        font_renderer: &mut FontRenderer,
        font_config: &FontConfig,
    ) {
        // Create tab-specific font config
        let mut tab_font_config = font_config.clone();
        tab_font_config.scale = style.tab_font_scale;

        let (cell_w, cell_h) = font_renderer.cell_size(&tab_font_config);

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
        let max_title_chars =
            (tab_width.saturating_sub(style.tab_internal_padding_px * 2)) / cell_w;

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
            let tab_y0 = style.tab_vertical_padding_px / 2;
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

            // Draw text with proper alpha blending
            let baseline_offset = font_renderer.baseline_offset(&tab_font_config);
            let mut char_x = text_x;

            for ch in display_title.chars() {
                if let Ok(Some(tt_glyph)) = font_renderer.get_glyph(ch, &tab_font_config) {
                    let glyph_y = text_y
                        .saturating_add(baseline_offset as usize)
                        .saturating_sub(tt_glyph.bearing_y as usize);

                    // Use proper alpha blending
                    Self::draw_glyph_with_alpha(
                        buffer,
                        buffer_width,
                        tab_bar_height,
                        char_x,
                        glyph_y,
                        &tt_glyph.pixels,
                        tt_glyph.width,
                        tt_glyph.height,
                        text_color,
                    );
                } else {
                    // Bitmap fallback
                    let glyph = get_bitmap_glyph(ch);
                    let bitmap_scale = (cell_h / 8).max(1);
                    for gy in 0..8 {
                        let bits = glyph[gy];
                        for gx in 0..8 {
                            if (bits >> gx) & 1 == 1 {
                                for sy in 0..bitmap_scale {
                                    for sx in 0..bitmap_scale {
                                        let px = char_x + gx * bitmap_scale + sx;
                                        let py = text_y + gy * bitmap_scale + sy;
                                        if px < buffer_width && py < text_y + tab_bar_height {
                                            buffer[py * buffer_width + px] = text_color;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                char_x += cell_w;
            }

            x_offset = tab_x1 + tab_gap;

            if x_offset >= buffer_width {
                break;
            }
        }
    }

    /// Helper to draw a glyph with proper alpha blending
    fn draw_glyph_with_alpha(
        buffer: &mut [u32],
        buffer_width: usize,
        max_height: usize,
        x0: usize,
        y0: usize,
        glyph_data: &[u8],
        glyph_width: usize,
        glyph_height: usize,
        color: u32,
    ) {
        let (r, g, b) = (
            ((color >> 16) & 0xFF) as u8,
            ((color >> 8) & 0xFF) as u8,
            (color & 0xFF) as u8,
        );

        for gy in 0..glyph_height {
            let y = y0 + gy;
            if y >= max_height {
                break;
            }

            for gx in 0..glyph_width {
                let x = x0 + gx;
                if x >= buffer_width {
                    break;
                }

                let alpha = glyph_data[gy * glyph_width + gx];
                if alpha > 0 {
                    let buffer_idx = y * buffer_width + x;

                    if alpha == 255 {
                        buffer[buffer_idx] = color;
                    } else {
                        // Proper alpha blending
                        let existing = buffer[buffer_idx];
                        let (er, eg, eb) = (
                            ((existing >> 16) & 0xFF) as u8,
                            ((existing >> 8) & 0xFF) as u8,
                            (existing & 0xFF) as u8,
                        );

                        let t = alpha as f32 / 255.0;
                        let nr = (er as f32 + (r as f32 - er as f32) * t) as u8;
                        let ng = (eg as f32 + (g as f32 - eg as f32) * t) as u8;
                        let nb = (eb as f32 + (b as f32 - eb as f32) * t) as u8;

                        buffer[buffer_idx] =
                            ((nr as u32) << 16) | ((ng as u32) << 8) | (nb as u32);
                    }
                }
            }
        }
    }
}
