//!
//! Terminal state management built on a terminal-core dependency.
//!
//! We use `tattoy-wezterm-term` as the terminal model because it supports
//! robust resize behavior (rewrapping logical lines instead of truncating
//! and losing data when the viewport shrinks).

mod adapters;
mod resize;
mod shell;
mod snapshot;

use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::constants::{DEFAULT_COLS, DEFAULT_ROWS, SCROLLBACK_CAPACITY};
use crate::pty::PtyWriter;

use adapters::{PtyWriteAdapter, TermConfig};
use shell::{ShellIntegrationAlertHandler, ShellIntegrationState};

use tattoy_wezterm_term::{Terminal as WezTerminal, TerminalConfiguration, TerminalSize};

pub use shell::{ShellIntegrationStatus, StickyPromptInfo};
pub use snapshot::ScreenSnapshot;

// Re-export mouse types for use by the app
pub use tattoy_wezterm_term::{KeyModifiers, MouseButton, MouseEventKind};

/// Core terminal state, independent of rendering.
///
/// Internally uses a robust terminal model that reflows on resize.
pub struct Terminal {
    term: Mutex<WezTerminal>,
    pty: Arc<dyn PtyWriter>,
    size: Mutex<(u16, u16)>,
    generation: AtomicU64,
    shell_integration: Arc<Mutex<ShellIntegrationState>>,
}

impl Terminal {
    /// Creates a new terminal with the given PTY.
    pub fn new(pty: Arc<dyn PtyWriter>) -> Self {
        Self::new_with_scrollback(pty, SCROLLBACK_CAPACITY)
    }

    pub fn new_with_scrollback(pty: Arc<dyn PtyWriter>, scrollback_lines: usize) -> Self {
        let config: Arc<dyn TerminalConfiguration + Send + Sync> = Arc::new(TermConfig {
            scrollback: scrollback_lines,
        });

        let size = TerminalSize {
            rows: DEFAULT_ROWS as usize,
            cols: DEFAULT_COLS as usize,
            pixel_width: 0,
            pixel_height: 0,
            dpi: 0,
        };

        let writer: Box<dyn Write + Send> = Box::new(PtyWriteAdapter { pty: pty.clone() });

        let shell_integration = Arc::new(Mutex::new(ShellIntegrationState::default()));

        let mut term = WezTerminal::new(size, config, "yatmux", env!("CARGO_PKG_VERSION"), writer);
        term.set_notification_handler(Box::new(ShellIntegrationAlertHandler {
            state: shell_integration.clone(),
        }));

        Terminal {
            term: Mutex::new(term),
            pty,
            size: Mutex::new((DEFAULT_ROWS, DEFAULT_COLS)),
            generation: AtomicU64::new(1),
            shell_integration,
        }
    }

    fn bump_generation(&self) {
        self.generation.fetch_add(1, Ordering::Relaxed);
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Relaxed)
    }
}
