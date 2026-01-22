//! Search functionality for the terminal scrollback buffer.
//!
//! Provides search within terminal history with match highlighting.
//! Supports both plain text and regex search modes.

mod navigation;
mod pattern;
mod state;

pub use state::{SearchMatch, SearchMode, SearchState};
