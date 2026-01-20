//! PTY (Pseudo-Terminal) handling for shell communication.

use std::io::{Read, Write};
use std::path::Path;
use std::sync::Mutex;

use anyhow::{Context as _, Result};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};

use crate::constants::{DEFAULT_COLS, DEFAULT_ROWS};

/// Trait for writing to a PTY.
///
/// This abstraction allows for mocking the PTY in tests.
pub trait PtyWriter: Send + Sync {
    /// Writes bytes to the PTY.
    fn write(&self, bytes: &[u8]);

    /// Resizes the PTY to the given dimensions.
    fn resize(&self, rows: u16, cols: u16, pixel_width: u16, pixel_height: u16);
}

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
    fn write_impl(&self, bytes: &[u8]) {
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
    fn resize_impl(&self, size: PtySize) {
        if let Ok(master) = self.master.lock() {
            if let Err(e) = master.resize(size) {
                eprintln!("pty resize failed: {e}");
            }
        }
    }
}

impl PtyWriter for Pty {
    fn write(&self, bytes: &[u8]) {
        self.write_impl(bytes);
    }

    fn resize(&self, rows: u16, cols: u16, pixel_width: u16, pixel_height: u16) {
        self.resize_impl(PtySize {
            rows,
            cols,
            pixel_width,
            pixel_height,
        });
    }
}

/// Spawns a shell process and returns the PTY handle and reader.
pub fn spawn_shell() -> Result<(Pty, Box<dyn Read + Send>)> {
    spawn_shell_with_cwd(None)
}

/// Spawns a shell process with an optional working directory.
pub fn spawn_shell_with_cwd(cwd: Option<&Path>) -> Result<(Pty, Box<dyn Read + Send>)> {
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
    cmd.env("TERM_PROGRAM", "yatmux");
    cmd.env("TERM_PROGRAM_VERSION", env!("CARGO_PKG_VERSION"));
    cmd.env("YATMUX", "1");
    if let Some(cwd) = cwd {
        cmd.cwd(cwd);
    }

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

/// Mock PTY writer for testing.
///
/// Records all writes and resize calls for verification in tests.
pub mod mock {
    use super::PtyWriter;
    use std::sync::Mutex;

    /// A mock PTY that records all operations for testing.
    #[derive(Default)]
    pub struct MockPty {
        /// All bytes written to the PTY.
        pub writes: Mutex<Vec<Vec<u8>>>,
        /// All resize operations: (rows, cols, pixel_width, pixel_height).
        pub resizes: Mutex<Vec<(u16, u16, u16, u16)>>,
    }

    impl MockPty {
        /// Creates a new mock PTY.
        pub fn new() -> Self {
            Self::default()
        }

        /// Returns all bytes written to the PTY, concatenated.
        pub fn written_bytes(&self) -> Vec<u8> {
            self.writes
                .lock()
                .unwrap()
                .iter()
                .flatten()
                .copied()
                .collect()
        }

        /// Returns all bytes written as a string (lossy conversion).
        pub fn written_string(&self) -> String {
            String::from_utf8_lossy(&self.written_bytes()).to_string()
        }
    }

    impl PtyWriter for MockPty {
        fn write(&self, bytes: &[u8]) {
            self.writes.lock().unwrap().push(bytes.to_vec());
        }

        fn resize(&self, rows: u16, cols: u16, pixel_width: u16, pixel_height: u16) {
            self.resizes
                .lock()
                .unwrap()
                .push((rows, cols, pixel_width, pixel_height));
        }
    }
}
