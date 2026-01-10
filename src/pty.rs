//! PTY (Pseudo-Terminal) handling for shell communication.

use std::io::{Read, Write};
use std::sync::Mutex;

use anyhow::{Context as _, Result};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};

use crate::constants::{DEFAULT_COLS, DEFAULT_ROWS};

/// Wrapper around a pseudo-terminal for shell communication.
pub struct Pty {
    master: Mutex<Box<dyn portable_pty::MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl Pty {
    /// Writes bytes to the PTY.
    ///
    /// Logs a warning if the write fails but does not propagate the error,
    /// as write failures during normal operation (e.g., shell exit) are expected.
    pub fn write(&self, bytes: &[u8]) {
        if let Ok(mut writer) = self.writer.lock() {
            if let Err(e) = writer.write_all(bytes) {
                eprintln!("pty write failed: {e}");
                return;
            }
            if let Err(e) = writer.flush() {
                eprintln!("pty flush failed: {e}");
            }
        }
    }

    /// Resizes the PTY to the given dimensions.
    pub fn resize(&self, size: PtySize) {
        if let Ok(master) = self.master.lock() {
            if let Err(e) = master.resize(size) {
                eprintln!("pty resize failed: {e}");
            }
        }
    }
}

/// Spawns a shell process and returns the PTY handle and reader.
pub fn spawn_shell() -> Result<(Pty, Box<dyn Read + Send>)> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: DEFAULT_ROWS,
            cols: DEFAULT_COLS,
            pixel_width: 0,
            pixel_height: 0,
        })
        .context("openpty")?;

    let mut cmd = CommandBuilder::new(default_shell());
    cmd.env("TERM", "xterm-256color");

    let child = pair.slave.spawn_command(cmd).context("spawn shell")?;

    let writer = pair.master.take_writer().context("take_writer")?;
    let reader = pair.master.try_clone_reader().context("clone_reader")?;

    Ok((
        Pty {
            master: Mutex::new(pair.master),
            writer: Mutex::new(writer),
            _child: child,
        },
        reader,
    ))
}

/// Returns the default shell for the current platform.
fn default_shell() -> String {
    #[cfg(windows)]
    {
        "powershell.exe".to_string()
    }
    #[cfg(not(windows))]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string())
    }
}
