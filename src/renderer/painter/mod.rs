//! Pixel painting for terminal rendering.
//!
//! `Renderer` is responsible for painting `RenderFrame`s to pixel buffers.
//! It is intentionally stateless - all interactive state lives in `TerminalView`.

mod frame;
mod glyph;
mod hud;
mod overlays;
mod primitives;

/// Pixel-paints a `RenderFrame` to the window surface.
///
/// All interactive state lives in `TerminalView`; this type is intentionally
/// stateless.
pub struct Renderer;

impl Default for Renderer {
    fn default() -> Self {
        Renderer::new()
    }
}

impl Renderer {
    pub fn new() -> Self {
        Renderer
    }
}
