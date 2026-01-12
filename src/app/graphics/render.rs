use yatmux::renderer::UiStyle;

use crate::app::App;

impl App {
    /// Renders all panes.
    pub fn render(&mut self) {
        // Probe buffer dimensions and snapshot palette without holding borrows.
        let (buffer_width, buffer_height, palette) = {
            let Some(graphics) = &mut self.graphics else {
                return;
            };
            let (bw, bh) = match graphics.surface.buffer_mut() {
                Ok(buffer) => (buffer.width().get(), buffer.height().get()),
                Err(e) => {
                    eprintln!("softbuffer buffer_mut failed: {e:?}");
                    return;
                }
            };
            (bw, bh, graphics.palette.clone())
        };

        // Update PTY sizes if layout/buffer changed.
        self.resize_panes_if_needed(buffer_width, buffer_height);

        let tab_bar_height = self.tab_bar_height();

        // Gather all config data we need
        let bg_color = self.config.colors.background;
        let accent_color = self.config.colors.accent;
        let font_scale = self.config.font.scale;
        let ui_style = UiStyle::from_config(&self.config);
        let num_tabs = self.tabs.len();

        // Get pane render data and dividers from active tab
        let Some((pane_data, dividers)) =
            self.collect_pane_render_data(buffer_width, buffer_height)
        else {
            return;
        };

        // Collect tab info for rendering tab bar
        let tab_info: Vec<(String, bool)> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(idx, tab)| {
                let is_active = idx == self.active_tab;
                (tab.title.clone(), is_active)
            })
            .collect();

        // Now get the buffer and do base rendering
        let Some(graphics) = &mut self.graphics else {
            return;
        };

        let mut buffer = match graphics.surface.buffer_mut() {
            Ok(buffer) => buffer,
            Err(e) => {
                eprintln!("softbuffer buffer_mut failed: {e:?}");
                return;
            }
        };

        buffer.fill(bg_color);

        // Render tab bar if there are multiple tabs
        if num_tabs > 1 && tab_bar_height > 0 {
            Self::render_tab_bar_static(
                &mut buffer,
                buffer_width as usize,
                tab_bar_height,
                &tab_info,
                bg_color,
                accent_color,
                font_scale,
                &ui_style,
            );
        }

        // Render each pane - we need to drop the buffer borrow temporarily to access panes.
        drop(buffer);

        self.render_panes(
            &pane_data,
            buffer_width,
            buffer_height,
            &palette,
            &ui_style,
            accent_color,
        );

        self.render_overlays(
            buffer_width,
            buffer_height,
            tab_bar_height,
            &dividers,
            accent_color,
            font_scale,
            &ui_style,
        );
    }
}
