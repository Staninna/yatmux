use std::io::Read;
use std::thread;

use winit::event_loop::EventLoopProxy;

use yatmux::constants::READ_BUFFER_SIZE;

use super::AppEvent;
use super::layout::PaneId;
use super::tab::TabId;

/// Spawns a thread to read PTY output and send events.
pub fn spawn_pty_reader(
    mut reader: Box<dyn Read + Send>,
    proxy: EventLoopProxy<AppEvent>,
    tab: TabId,
    pane: PaneId,
) {
    thread::spawn(move || {
        let mut buf = [0u8; READ_BUFFER_SIZE];
        loop {
            let n = match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            };

            let _ = proxy.send_event(AppEvent::PtyOutput {
                tab,
                pane,
                bytes: buf[..n].to_vec(),
            });
        }

        let _ = proxy.send_event(AppEvent::PtyExited { tab, pane });
    });
}
