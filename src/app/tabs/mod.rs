use super::*;
use yatmux::renderer::UiStyle;

mod lifecycle;
mod navigation;
mod reorder;

impl App {
    pub fn pane_rects(
        &self,
        buffer_width: usize,
        buffer_height: usize,
    ) -> (Vec<(PaneId, Rect)>, Vec<layout::Divider>) {
        // Reserve space for tab bar when there are multiple tabs
        let tab_bar_height = self.tab_bar_height();
        let pane_height = buffer_height.saturating_sub(tab_bar_height);

        if let Some(tab) = self.active_tab() {
            let (mut rects, dividers) = tab.pane_rects(buffer_width, pane_height);
            // Offset all rects by the tab bar height
            for (_, rect) in &mut rects {
                rect.y += tab_bar_height;
            }
            (rects, dividers)
        } else {
            (Vec::new(), Vec::new())
        }
    }

    pub fn tab_bar_height(&self) -> usize {
        if self.tabs.len() > 1 {
            let style = UiStyle::from_config(&self.config);

            // Create tab-specific font config with scaled font
            let mut tab_font_config = self.config.font.clone();
            tab_font_config.scale = style.tab_font_scale;

            let (_, cell_h) = self.renderer.font_renderer.cell_size(&tab_font_config);

            let height_from_padding = cell_h + style.tab_vertical_padding_px;
            let height_from_cells = style
                .tab_min_height_cells
                .map(|cells| cells * cell_h)
                .unwrap_or(0);

            height_from_padding.max(height_from_cells)
        } else {
            0
        }
    }

    pub fn pane_at_position(
        &self,
        rects: &[(PaneId, Rect)],
        pos: PhysicalPosition<f64>,
    ) -> Option<(PaneId, Rect)> {
        rects
            .iter()
            .find(|(_id, r)| r.contains(pos.x, pos.y))
            .copied()
    }

    pub fn close_tab_by_id(&mut self, tab_id: TabId) -> bool {
        let Some(index) = self.tabs.iter().position(|t| t.id == tab_id) else {
            return false;
        };
        self.close_tab(index);
        true
    }

    pub fn close_pane_by_id(&mut self, tab_id: TabId, pane_id: PaneId) -> bool {
        let Some(index) = self.tabs.iter().position(|t| t.id == tab_id) else {
            return false;
        };
        let should_close_tab = {
            let t = &mut self.tabs[index];
            if t.panes.contains_key(&pane_id) {
                t.close_pane(pane_id)
            } else {
                false
            }
        };
        if should_close_tab {
            self.close_tab(index);
        } else {
            self.layout_dirty = true;
            self.request_redraw();
        }
        true
    }

    pub(super) fn initialize_first_tab(&mut self) {
        if !self.tabs.is_empty() {
            return;
        }
        self.new_tab();
    }

    pub(super) fn localize_pos(rect: Rect, pos: PhysicalPosition<f64>) -> PhysicalPosition<f64> {
        PhysicalPosition::new(pos.x - rect.x as f64, pos.y - rect.y as f64)
    }
}
