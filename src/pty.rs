use std::io::{Read, Write};
use std::sync::Mutex;

use anyhow::{Context as _, Result};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};

pub struct Pty {
    master: Mutex<Box<dyn portable_pty::MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl Pty {
    pub fn write(&self, bytes: &[u8]) {
        if let Ok(mut writer) = self.writer.lock() {
            let _ = writer.write_all(bytes);
            let _ = writer.flush();
        }
    }

    pub fn resize(&self, size: PtySize) {
        if let Ok(master) = self.master.lock() {
            let _ = master.resize(size);
        }
    }
}

pub fn spawn_shell() -> Result<(Pty, Box<dyn Read + Send>)> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: crate::DEFAULT_ROWS,
            cols: crate::DEFAULT_COLS,
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
