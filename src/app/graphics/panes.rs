use std::sync::Arc;

use yatmux::renderer::UiStyle;

use crate::app::App;
use crate::app::layout::{PaneId, Rect, draw_border};

use crate::app::layout::Divider;

/// Data needed to render a single pane.
pub(super) struct PaneRenderData {
    pub(super) id: PaneId,
    pub(super) rect: Rect,
    pub(super) content_rect: Rect, // Inner rect after padding
    pub(super) scale: usize,
    pub(super) is_focused: bool,
}

impl App {
    pub(super) fn collect_pane_render_data(
        &self,
        buffer_width: u32,
        buffer_height: u32,
    ) -> Option<(Vec<PaneRenderData>, Vec<Divider>)> {
        let tab_bar_height = self.tab_bar_height();
        let pane_area_height = (buffer_height as usize).saturating_sub(tab_bar_height);

        // Get padding config
        let padding_left = self.config.pane.padding_left();
        let padding_right = self.config.pane.padding_right();
        let padding_top = self.config.pane.padding_top();
        let padding_bottom = self.config.pane.padding_bottom();

        let tab = self.active_tab()?;
        let (rects, dividers) = tab.pane_rects(buffer_width as usize, pane_area_height);
        let data: Vec<PaneRenderData> = rects
            .into_iter()
            .filter_map(|(id, rect)| {
                tab.panes.get(&id).map(|pane| {
                    let outer_rect = Rect {
                        x: rect.x,
                        y: rect.y + tab_bar_height,
                        w: rect.w,
                        h: rect.h,
                    };
                    // Calculate content rect with padding
                    let content_rect = Rect {
                        x: outer_rect.x + padding_left,
                        y: outer_rect.y + padding_top,
                        w: outer_rect.w.saturating_sub(padding_left + padding_right),
                        h: outer_rect.h.saturating_sub(padding_top + padding_bottom),
                    };
                    PaneRenderData {
                        id,
                        rect: outer_rect,
                        content_rect,
                        scale: pane.scale,
                        is_focused: id == tab.focused_pane,
                    }
                })
            })
            .collect();

        Some((data, dividers))
    }

    pub(super) fn render_panes(
        &mut self,
        pane_data: &[PaneRenderData],
        buffer_width: u32,
        buffer_height: u32,
        palette: &Arc<[u32; 256]>,
        ui_style: &UiStyle,
        accent_color: u32,
    ) {
        for pane_render in pane_data {
            let (cell_w, cell_h) = Self::cell_size_for_scale(pane_render.scale);
            if pane_render.content_rect.w < cell_w || pane_render.content_rect.h < cell_h {
                continue;
            }

            // Get mutable access to the pane
            let Some(tab) = self.tabs.get_mut(self.active_tab) else {
                continue;
            };
            let Some(pane) = tab.panes.get_mut(&pane_render.id) else {
                continue;
            };

            // Re-acquire the buffer for this pane
            let graphics = self.graphics.as_mut().expect("graphics exists");
            let mut buffer = match graphics.surface.buffer_mut() {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("softbuffer buffer_mut failed: {e:?}");
                    return;
                }
            };

            // Render terminal content in the padded content area
            if let Err(e) = self.renderer.paint_terminal_region(
                &mut buffer,
                buffer_width as usize,
                buffer_height as usize,
                pane_render.content_rect.x,
                pane_render.content_rect.y,
                pane_render.content_rect.w,
                pane_render.content_rect.h,
                cell_w,
                cell_h,
                pane_render.scale,
                &pane.terminal,
                palette,
                &mut pane.view,
                ui_style,
            ) {
                eprintln!("Render pane {} error: {e:#}", pane_render.id);
            }

            // Render sticky prompt if scrolled up and enabled (not during command execution)
            if self.config.shell_integration.sticky_prompt
                && pane.view.is_scrolled_up()
                && !pane.command_running
            {
                if let Some(prompt_info) = pane.terminal.current_prompt_rows() {
                    self.renderer.paint_sticky_prompt(
                        &mut buffer,
                        buffer_width as usize,
                        buffer_height as usize,
                        pane_render.content_rect.x,
                        pane_render.content_rect.y,
                        pane_render.content_rect.w,
                        pane_render.content_rect.h,
                        cell_w,
                        cell_h,
                        pane_render.scale,
                        &prompt_info.rows,
                        prompt_info.cursor,
                        palette,
                        ui_style,
                    );
                }
            }

            // Draw borders around panes.
            // - Inactive panes: subtle divider border
            // - Active pane: accent border on top
            draw_border(
                &mut buffer,
                buffer_width as usize,
                buffer_height as usize,
                pane_render.rect,
                ui_style.divider,
            );

            if pane_render.is_focused {
                draw_border(
                    &mut buffer,
                    buffer_width as usize,
                    buffer_height as usize,
                    pane_render.rect,
                    accent_color,
                );
            }
        }
    }
}
