//! Graphics state and rendering for the terminal application.

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Arc;

use softbuffer::{Context, Surface};
use term::config::Action;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

use term::renderer::{HelpSection, create_palette};

use crate::app::App;
use crate::app::layout::{PaneId, Rect, draw_border, fill_rect};

/// Graphics state for rendering.
pub struct GraphicsState {
    #[allow(dead_code)]
    pub context: Context<winit::event_loop::OwnedDisplayHandle>,
    pub surface: Surface<winit::event_loop::OwnedDisplayHandle, Window>,
    pub palette: Arc<[u32; 256]>,
}

/// Data needed to render a single pane.
struct PaneRenderData {
    id: PaneId,
    rect: Rect,
    content_rect: Rect, // Inner rect after padding
    scale: usize,
    is_focused: bool,
}

impl App {
    /// Creates the window and graphics context.
    pub fn create_window(&mut self, event_loop: &ActiveEventLoop) {
        let window_attrs = Window::default_attributes().with_title(&self.config.window.title);

        let window = match event_loop.create_window(window_attrs) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("Failed to create window: {e}");
                return;
            }
        };

        let display = event_loop.owned_display_handle();
        let context = match Context::new(display) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to create graphics context: {e:?}");
                return;
            }
        };

        let surface = match Surface::new(&context, window) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to create surface: {e:?}");
                return;
            }
        };

        let palette = Arc::new(create_palette());

        self.graphics = Some(GraphicsState {
            context,
            surface,
            palette,
        });

        self.handle_resize();
        self.request_redraw();
    }

    /// Handles window resize events.
    pub fn handle_resize(&mut self) {
        let Some(graphics) = &mut self.graphics else {
            return;
        };

        let size = graphics.surface.window().inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        if let Err(e) = graphics.surface.resize(
            NonZeroU32::new(width).expect("width >= 1"),
            NonZeroU32::new(height).expect("height >= 1"),
        ) {
            eprintln!("Surface resize failed: {e:?}");
            return;
        }

        self.layout_dirty = true;
    }

    /// Resizes panes if the buffer size or layout has changed.
    pub fn resize_panes_if_needed(&mut self, buffer_width: u32, buffer_height: u32) {
        if (buffer_width, buffer_height) == self.last_buffer_size && !self.layout_dirty {
            return;
        }

        let tab_bar_height = self.tab_bar_height();
        let pane_height = (buffer_height as usize).saturating_sub(tab_bar_height);

        // Get padding config
        let padding_left = self.config.pane.padding_left();
        let padding_right = self.config.pane.padding_right();
        let padding_top = self.config.pane.padding_top();
        let padding_bottom = self.config.pane.padding_bottom();

        let Some(tab) = self.active_tab() else {
            return;
        };

        let (rects, _) = tab.pane_rects(buffer_width as usize, pane_height);

        // We need to iterate over the tab's panes, so we re-borrow
        let Some(tab) = self.active_tab_mut() else {
            return;
        };

        for (id, rect) in rects {
            if let Some(pane) = tab.panes.get(&id) {
                let (cell_w, cell_h) = Self::cell_size_for_scale(pane.scale);
                // Calculate content dimensions (after padding)
                let content_w = rect.w.saturating_sub(padding_left + padding_right) as u32;
                let content_h = rect.h.saturating_sub(padding_top + padding_bottom) as u32;
                pane.terminal.resize(content_w, content_h, cell_w, cell_h);
            }
        }

        self.last_buffer_size = (buffer_width, buffer_height);
        self.layout_dirty = false;
    }

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
        let pane_area_height = (buffer_height as usize).saturating_sub(tab_bar_height);

        // Gather all config data we need
        let bg_color = self.config.colors.background;
        let accent_color = self.config.colors.accent;
        let font_scale = self.config.font.scale;
        let num_tabs = self.tabs.len();
        let padding_left = self.config.pane.padding_left();
        let padding_right = self.config.pane.padding_right();
        let padding_top = self.config.pane.padding_top();
        let padding_bottom = self.config.pane.padding_bottom();

        // Get pane render data and dividers from active tab
        let (pane_data, dividers): (Vec<PaneRenderData>, _) = {
            let Some(tab) = self.active_tab() else {
                return;
            };
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
            (data, dividers)
        };

        // Collect tab info for rendering tab bar
        let tab_info: Vec<(String, bool)> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(idx, tab)| (tab.title.clone(), idx == self.active_tab))
            .collect();

        // Now get the buffer and do all rendering
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
            );
        }

        // Render each pane - we need to drop the buffer borrow temporarily to access panes
        // because self.graphics and self.tabs are separate fields, but Rust can't see that
        // through the method calls.
        drop(buffer);

        for pane_render in &pane_data {
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
                &palette,
                &mut pane.view,
            ) {
                eprintln!("Render pane {} error: {e:#}", pane_render.id);
            }

            // Draw border around the outer pane rect (not the content rect)
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

        // Re-acquire buffer for dividers and help overlay
        let Some(graphics) = &mut self.graphics else {
            return;
        };
        let mut buffer = match graphics.surface.buffer_mut() {
            Ok(b) => b,
            Err(e) => {
                eprintln!("softbuffer buffer_mut failed: {e:?}");
                return;
            }
        };

        // Draw dividers between panes (offset by tab bar height)
        for d in &dividers {
            fill_rect(
                &mut buffer,
                buffer_width as usize,
                buffer_height as usize,
                Rect {
                    x: d.x,
                    y: d.y + tab_bar_height,
                    w: d.w,
                    h: d.h,
                },
                0x2A2A2A,
            );
        }

        // Render help overlay if visible
        if self.show_help {
            self.help_scroll = self.help_scroll.min(self.help_max_scroll);

            let mut by_category: HashMap<&'static str, Vec<(String, String)>> = HashMap::new();
            for (key, action) in &self.config.keybinds.bindings {
                // Skip disabled bindings (Action::None)
                if *action == Action::None {
                    continue;
                }
                by_category
                    .entry(action.category())
                    .or_default()
                    .push((key.clone(), action.label().to_string()));
            }

            // Consolidate numbered entries (e.g., "Go to tab 1" through "Go to tab 9")
            for bindings in by_category.values_mut() {
                Self::consolidate_numbered_bindings(bindings);
                bindings.sort_by(|a, b| a.0.cmp(&b.0));
            }

            let order = [
                "General",
                "Tabs",
                "Panes",
                "Zoom",
                "Scrollback",
                "Search",
                "Help",
            ];
            let mut sections: Vec<HelpSection> = Vec::new();

            for category in order {
                if let Some(bindings) = by_category.remove(category) {
                    sections.push(HelpSection {
                        title: category.to_string(),
                        bindings,
                    });
                }
            }

            let mut extra: Vec<(&'static str, Vec<(String, String)>)> =
                by_category.into_iter().collect();
            extra.sort_by(|a, b| a.0.cmp(b.0));
            for (category, bindings) in extra {
                sections.push(HelpSection {
                    title: category.to_string(),
                    bindings,
                });
            }

            let (scroll, max_scroll) = self.renderer.paint_help_overlay(
                &mut buffer,
                buffer_width as usize,
                buffer_height as usize,
                "Shortcuts",
                &sections,
                self.help_scroll,
                accent_color,
                font_scale,
            );
            self.help_scroll = scroll;
            self.help_max_scroll = max_scroll;
        }

        if let Err(e) = buffer.present() {
            eprintln!("softbuffer present failed: {e:?}");
        }
    }

    /// Renders the tab bar at the top of the window (static method to avoid borrow issues).
    fn render_tab_bar_static(
        buffer: &mut [u32],
        buffer_width: usize,
        tab_bar_height: usize,
        tabs: &[(String, bool)],
        bg_color: u32,
        accent_color: u32,
        font_scale: usize,
    ) {
        let scale = font_scale.clamp(1, 8);
        let cell_w = 8 * scale;
        let cell_h = 8 * scale;

        // Background
        let tab_bar_bg = 0x1A1A1A;
        for y in 0..tab_bar_height {
            let row = y * buffer_width;
            for x in 0..buffer_width {
                buffer[row + x] = tab_bar_bg;
            }
        }

        // Bottom border
        let border_y = tab_bar_height.saturating_sub(1);
        for x in 0..buffer_width {
            buffer[border_y * buffer_width + x] = 0x333333;
        }

        // Calculate tab dimensions to share total space
        let num_tabs = tabs.len();
        if num_tabs == 0 {
            return;
        }

        let tab_gap = 4;
        let side_padding = 8;
        let total_gap_width = tab_gap * (num_tabs.saturating_sub(1)) + side_padding * 2;
        let available_width = buffer_width.saturating_sub(total_gap_width);
        let tab_width = (available_width / num_tabs).min(cell_w * 12 + 16); // Cap max width
        let max_title_chars = (tab_width.saturating_sub(16)) / cell_w; // Account for padding

        let mut x_offset = side_padding;

        for (idx, (title, is_active)) in tabs.iter().enumerate() {
            // Tab background
            let tab_bg = if *is_active { bg_color } else { 0x252525 };

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
            let text_color = if *is_active { 0xFFFFFF } else { 0x888888 };
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
            let glyph = term::renderer::font::get_glyph(ch);
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

    /// Requests a window redraw.
    pub fn request_redraw(&self) {
        if let Some(graphics) = &self.graphics {
            graphics.surface.window().request_redraw();
        }
    }

    /// Consolidates numbered keybindings like "alt+1" -> "Go to tab 1" through "alt+9" -> "Go to tab 9"
    /// into a single entry like "alt+1-9" -> "Go to tab 1-9".
    fn consolidate_numbered_bindings(bindings: &mut Vec<(String, String)>) {
        // Look for patterns like "Go to tab N" with keys like "alt+N"
        let patterns = [("Go to tab ", "alt+")];

        for (label_prefix, key_prefix) in patterns {
            // Find all matching entries
            let mut matches: Vec<(usize, char)> = Vec::new();
            for (i, (key, label)) in bindings.iter().enumerate() {
                if label.starts_with(label_prefix) && key.starts_with(key_prefix) {
                    let suffix = &label[label_prefix.len()..];
                    let key_suffix = &key[key_prefix.len()..];
                    if suffix.len() == 1 && key_suffix == suffix {
                        if let Some(digit) = suffix.chars().next() {
                            if digit.is_ascii_digit() {
                                matches.push((i, digit));
                            }
                        }
                    }
                }
            }

            // If we have multiple consecutive numbers, consolidate them
            if matches.len() >= 3 {
                matches.sort_by_key(|(_, d)| *d);
                let digits: Vec<char> = matches.iter().map(|(_, d)| *d).collect();
                let first = digits.first().unwrap();
                let last = digits.last().unwrap();

                // Remove original entries (in reverse order to preserve indices)
                let mut indices: Vec<usize> = matches.iter().map(|(i, _)| *i).collect();
                indices.sort();
                indices.reverse();
                for i in indices {
                    bindings.remove(i);
                }

                // Add consolidated entry
                bindings.push((
                    format!("{}{}-{}", key_prefix, first, last),
                    format!("{}{}-{}", label_prefix, first, last),
                ));
            }
        }
    }
}
