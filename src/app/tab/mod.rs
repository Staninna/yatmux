//! Tab management for the terminal application.
//!
//! Each tab contains its own set of panes with an independent layout tree.

mod focus;
mod layout;
mod pane;
mod r#struct;

pub use r#struct::{Tab, TabId};
