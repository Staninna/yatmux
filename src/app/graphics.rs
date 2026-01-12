//! Graphics state and rendering for the terminal application.

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Arc;

use softbuffer::{Context, Surface};
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

use term::renderer::{HelpSection, create_palette};

use crate::app::App;
use crate::app::layout::{Rect, draw_border, fill_rect};

/// Graphics state for rendering.
pub struct GraphicsState {
    #[allow(dead_code)]
    pub context: Context<winit::event_loop::OwnedDisplayHandle>,
    pub surface: Surface<winit::event_loop::OwnedDisplayHandle, Window>,
    pub palette: Arc<[u32; 256]>,
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

        let (rects, _) = self.pane_rects(buffer_width as usize, buffer_height as usize);

        for (id, rect) in rects {
            if let Some(pane) = self.panes.get(&id) {
                let (cell_w, cell_h) = Self::cell_size_for_scale(pane.scale);
                pane.terminal
                    .resize(rect.w as u32, rect.h as u32, cell_w, cell_h);
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

        let (rects, dividers) = self.pane_rects(buffer_width as usize, buffer_height as usize);

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

        buffer.fill(self.config.colors.background);

        for (id, rect) in &rects {
            let Some(pane) = self.panes.get_mut(id) else {
                continue;
            };

            let (cell_w, cell_h) = Self::cell_size_for_scale(pane.scale);
            if rect.w < cell_w || rect.h < cell_h {
                continue;
            }

            if let Err(e) = self.renderer.paint_terminal_region(
                &mut buffer,
                buffer_width as usize,
                buffer_height as usize,
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                cell_w,
                cell_h,
                pane.scale,
                &pane.terminal,
                &palette,
                &mut pane.view,
            ) {
                eprintln!("Render pane {id} error: {e:#}");
            }

            if *id == self.focused_pane {
                draw_border(
                    &mut buffer,
                    buffer_width as usize,
                    buffer_height as usize,
                    *rect,
                    self.config.colors.accent,
                );
            }
        }

        // Draw dividers between panes
        for d in &dividers {
            fill_rect(
                &mut buffer,
                buffer_width as usize,
                buffer_height as usize,
                Rect {
                    x: d.x,
                    y: d.y,
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
                by_category
                    .entry(action.category())
                    .or_default()
                    .push((key.clone(), action.label().to_string()));
            }

            for bindings in by_category.values_mut() {
                bindings.sort_by(|a, b| a.0.cmp(&b.0));
            }

            let order = ["General", "Panes", "Zoom", "Scrollback", "Search", "Help"];
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
                self.config.colors.accent,
                self.config.font.scale,
            );
            self.help_scroll = scroll;
            self.help_max_scroll = max_scroll;
        }

        if let Err(e) = buffer.present() {
            eprintln!("softbuffer present failed: {e:?}");
        }
    }

    /// Requests a window redraw.
    pub fn request_redraw(&self) {
        if let Some(graphics) = &self.graphics {
            graphics.surface.window().request_redraw();
        }
    }
}
