use std::num::NonZeroU32;
use std::sync::Arc;

use softbuffer::{Context, Surface};
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

use yatmux::renderer::create_palette_with_ansi;

use crate::app::App;

use super::GraphicsState;

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

        let palette = Arc::new(create_palette_with_ansi(self.config.colors.palette));

        self.graphics = Some(GraphicsState {
            context,
            surface,
            palette,
        });

        self.sync_window_title();
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

    /// Requests a window redraw.
    pub fn request_redraw(&self) {
        if let Some(graphics) = &self.graphics {
            graphics.surface.window().request_redraw();
        }
    }
}
