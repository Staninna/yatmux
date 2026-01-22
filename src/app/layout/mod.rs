//! Layout management for terminal panes using a binary tree structure.

mod rects;
mod split;
mod tree;

pub type PaneId = u64;

pub use rects::{draw_border, fill_rect, overlap_1d, Rect};
pub use split::SplitDir;
pub use tree::{Divider, LayoutNode};
