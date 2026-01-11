//! Terminal emulator library.
//!
//! This module exposes the core terminal components for use in integration tests.

pub mod clipboard;
pub mod config;
pub mod constants;
pub mod core;
pub mod keys;
pub mod pty;
pub mod renderer;
pub mod terminal;

// Re-export commonly used types
pub use clipboard::ClipboardProvider;
pub use config::Config;
pub use pty::PtyWriter;
pub use terminal::Terminal;
