//! Terminal rendering and terminal view state.
//!
//! This module provides:
//! - `TerminalView`: UI state management (scrolling, selection, URLs, search)
//! - `Renderer`: Pixel painting for terminal frames
//!
//! Scrollback lives in the terminal model (`wezterm-term`). The view asks the
//! terminal for a snapshot of the *visible* rows. Only when search needs to
//! (re)index matches do we build a full snapshot of the scrollback.

mod color;
pub mod font;
mod help;
mod painter;
mod view;

pub use crate::core::search::{SearchMatch, SearchState};
pub use color::create_palette;
pub use painter::Renderer;
pub use view::TerminalView;

use crate::core::grid::RowSnapshot;

/// Search highlight colors.
pub(crate) const SEARCH_MATCH_BG: u32 = 0x4A4A00; // Dark yellow for regular matches
pub(crate) const SEARCH_CURRENT_BG: u32 = 0x806000; // Brighter yellow for current match

/// A frame of terminal content ready for rendering.
pub(crate) struct RenderFrame {
    pub cursor: (u16, u16),
    pub display_rows: Vec<RowSnapshot>,
    pub rows: usize,
    pub cols: usize,
    pub view_start: usize,
    pub show_cursor: bool,
}

/// A categorized list of key bindings for the help overlay.
#[derive(Clone, Debug)]
pub struct HelpSection {
    pub title: String,
    pub bindings: Vec<(String, String)>,
}
