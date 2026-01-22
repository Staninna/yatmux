use super::super::*;

impl App {
    pub(super) fn update_tab_drag_on_move(
        &mut self,
        position: PhysicalPosition<f64>,
        tab_bar_height: usize,
    ) -> bool {
        let mut drag_committed = false;
        let dragging_info = if let Some(ref mut drag) = self.input.tab_dragging {
            let distance = (position.x - drag.start_x).abs();
            let was_committed = drag.committed;

            // Commit drag if moved beyond threshold (5px)
            if !drag.committed && distance > 5.0 {
                drag.committed = true;
            }

            drag_committed = drag.committed;

            Some((was_committed, drag.committed))
        } else {
            None
        };

        if drag_committed && tab_bar_height > 0 {
            let drop_idx = self.calculate_tab_drop_position(position.x);
            if let Some(ref mut drag) = self.input.tab_dragging {
                drag.last_drop_idx = Some(drop_idx);
            }
        }

        if let Some((was_committed, now_committed)) = dragging_info {
            if !was_committed && now_committed {
                self.update_cursor(); // Change to grabbing cursor
            }
            if now_committed {
                self.request_redraw(); // Redraw for visual feedback
            }
            return true; // Don't process other hover logic during drag
        }

        false
    }

    /// Calculates the drop position (insertion index) for a tab being dragged to x-coordinate
    fn calculate_tab_drop_position(&self, x: f64) -> usize {
        let buffer_width = self.last_buffer_size.0 as usize;
        let num_tabs = self.tabs.len();

        if num_tabs == 0 {
            return 0;
        }

        let style = &yatmux::renderer::UiStyle::from_config(&self.config);
        let mut tab_font_config = self.config.font.clone();
        tab_font_config.scale = style.tab_font_scale;

        let (cell_w, _) = self.renderer.font_renderer.cell_size(&tab_font_config);
        let tab_gap = style.tab_gap_px;
        let side_padding = style.tab_side_padding_px;
        let total_gap_width = tab_gap * (num_tabs.saturating_sub(1)) + side_padding * 2;
        let available_width = buffer_width.saturating_sub(total_gap_width);
        let tab_width = (available_width / num_tabs)
            .min(cell_w * style.tab_max_width_cells + style.tab_max_width_px_extra);

        let cursor_x = x as usize;

        // If before first tab, drop at position 0
        if cursor_x < side_padding {
            return 0;
        }

        // Calculate midpoints of each tab boundary
        let mut x_offset = side_padding;
        for idx in 0..num_tabs {
            let tab_x0 = x_offset;
            let tab_x1 = if idx == num_tabs - 1 {
                (x_offset + tab_width).min(buffer_width.saturating_sub(side_padding))
            } else {
                (x_offset + tab_width).min(buffer_width)
            };

            let tab_midpoint = (tab_x0 + tab_x1) / 2;

            if cursor_x < tab_midpoint {
                return idx;
            }

            x_offset = tab_x1 + tab_gap;
        }

        // After all tabs, drop at end
        num_tabs
    }
}
