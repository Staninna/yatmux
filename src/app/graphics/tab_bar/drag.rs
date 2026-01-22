use crate::app::input::TabDragState;
use crate::app::App;
use yatmux::renderer::UiStyle;

impl App {
    pub(super) fn draw_drop_indicator(
        buffer: &mut [u32],
        buffer_width: usize,
        tab_bar_height: usize,
        num_tabs: usize,
        tab_width: usize,
        tab_gap: usize,
        side_padding: usize,
        drag_state: Option<&TabDragState>,
        cursor_x: f64,
        accent_color: u32,
        style: &UiStyle,
    ) {
        if let Some(drag) = drag_state {
            if drag.committed {
                // Calculate drop position indicator x-coordinate
                let cursor_x_usize = cursor_x as usize;

                // Recalculate tab positions to find drop indicator position
                let mut drop_x = None;
                let mut x_offset = side_padding;

                for idx in 0..num_tabs {
                    let tab_x0 = x_offset;
                    let tab_x1 = if idx == num_tabs - 1 {
                        (x_offset + tab_width).min(buffer_width.saturating_sub(side_padding))
                    } else {
                        (x_offset + tab_width).min(buffer_width)
                    };

                    let tab_midpoint = (tab_x0 + tab_x1) / 2;

                    if cursor_x_usize < tab_midpoint {
                        drop_x = Some(tab_x0);
                        break;
                    }

                    x_offset = tab_x1 + tab_gap;
                }

                // If no drop position found, drop at end
                if drop_x.is_none() && num_tabs > 0 {
                    drop_x = Some(x_offset.saturating_sub(tab_gap));
                }

                // Draw vertical line at drop position
                if let Some(x) = drop_x {
                    if x > 0 && x < buffer_width {
                        let indicator_y0 = style.tab_vertical_padding_px / 2;
                        let indicator_y1 = tab_bar_height.saturating_sub(1);

                        // Draw 3px wide line for visibility
                        for y in indicator_y0..indicator_y1 {
                            let row = y * buffer_width;
                            if x > 0 {
                                buffer[row + x.saturating_sub(1)] = accent_color;
                            }
                            buffer[row + x] = accent_color;
                            if x + 1 < buffer_width {
                                buffer[row + x + 1] = accent_color;
                            }
                        }
                    }
                }
            }
        }
    }
}
