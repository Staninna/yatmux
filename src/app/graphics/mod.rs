//! Graphics state and rendering for the terminal application.

mod overlays;
mod pane_resize;
mod panes;
mod render;
mod tab_bar;
mod window;

use std::sync::Arc;

use softbuffer::{Context, Surface};
use winit::window::Window;

/// Graphics state for rendering.
pub struct GraphicsState {
    #[allow(dead_code)]
    pub context: Context<winit::event_loop::OwnedDisplayHandle>,
    pub surface: Surface<winit::event_loop::OwnedDisplayHandle, Window>,
    pub palette: Arc<[u32; 256]>,
}
