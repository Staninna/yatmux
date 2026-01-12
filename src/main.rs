//! Terminal emulator entry point.

use anyhow::Result;
use winit::event_loop::EventLoop;

mod app;

use app::{App, AppEvent};
use yatmux::Config;

fn main() -> Result<()> {
    // Load configuration
    let config = Config::load();

    // Create event loop
    let event_loop = EventLoop::<AppEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();

    // Create application
    let mut app = App::new(config);
    app.set_event_proxy(proxy);

    // Run the application
    event_loop.run_app(&mut app)?;

    Ok(())
}
