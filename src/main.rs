use std::{
    io::Read,
    num::NonZeroU32,
    sync::{Arc, Mutex},
    thread,
};

use anyhow::{Context as _, Result};
use portable_pty::PtySize;
use softbuffer::{Context, Surface};
use winit::{
    event::{ElementState, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, ModifiersState, NamedKey},
    window::Window,
};

use crate::constants::{CELL_H, CELL_W, DEFAULT_COLS, DEFAULT_ROWS};

mod constants;
mod keys;
mod pty;
mod renderer;

use constants::*;
use keys::key_to_pty_bytes;
use pty::{Pty, spawn_shell};
use renderer::{FontStyle, Renderer, color_palette};

#[derive(Debug, Clone)]
enum UserEvent {
    PtyUpdated,
}

fn main() -> Result<()> {
    let (pty, pty_reader) = spawn_shell()?;
    let pty = Arc::new(pty);
    let parser = Arc::new(Mutex::new(vt100::Parser::new(
        DEFAULT_ROWS,
        DEFAULT_COLS,
        0,
    )));

    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();

    let palette = Arc::new(color_palette());
    let mut renderer = Renderer::new();

    {
        let parser = Arc::clone(&parser);
        let mut reader = pty_reader;
        thread::spawn(move || {
            let mut buf = [0u8; READ_BUFFER_SIZE];
            loop {
                let read_len = match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(_) => break,
                };
                {
                    if let Ok(mut parser) = parser.lock() {
                        parser.process(&buf[..read_len]);
                    }
                }
                let _ = proxy.send_event(UserEvent::PtyUpdated);
            }
        });
    }

    let display = event_loop.owned_display_handle();
    let window = event_loop.create_window(Window::default_attributes().with_title("term"))?;
    let context = Context::new(display)
        .map_err(|e| anyhow::anyhow!("softbuffer Context::new failed: {e:?}"))?;
    let mut surface = Surface::new(&context, window)
        .map_err(|e| anyhow::anyhow!("softbuffer Surface::new failed: {e:?}"))?;

    let mut modifiers = ModifiersState::default();

    resize_terminal(&mut surface, &parser, &pty)?;
    surface.window().request_redraw();

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        match event {
            Event::UserEvent(UserEvent::PtyUpdated) => surface.window().request_redraw(),
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::RedrawRequested => {
                    if let Err(err) = renderer.render(&mut surface, &parser, &palette) {
                        eprintln!("render error: {err:#}");
                    }
                }
                WindowEvent::CloseRequested => elwt.exit(),
                WindowEvent::Resized(_) => {
                    if let Err(err) = resize_terminal(&mut surface, &parser, &pty) {
                        eprintln!("resize error: {err:#}");
                    }
                    surface.window().request_redraw();
                }
                WindowEvent::ModifiersChanged(new_mods) => modifiers = new_mods.state(),
                WindowEvent::KeyboardInput { event, .. } => {
                    if event.state != ElementState::Pressed {
                        return;
                    }

                    if let Key::Named(NamedKey::F12) = &event.logical_key {
                        let styles: [FontStyle; 8] = [
                            FontStyle::Basic,
                            FontStyle::BoxDrawing,
                            FontStyle::Block,
                            FontStyle::Greek,
                            FontStyle::Hiragana,
                            FontStyle::Latin,
                            FontStyle::Misc,
                            FontStyle::Sga,
                        ];
                        let current = renderer.font_style();
                        let current_idx = styles.iter().position(|&s| s == current).unwrap_or(0);
                        let next_idx = (current_idx + 1) % styles.len();
                        renderer.set_font_style(styles[next_idx]);
                        eprintln!("Font switched to: {:?}", styles[next_idx]);
                        surface.window().request_redraw();
                        return;
                    }

                    if !modifiers.control_key() {
                        if let Some(text) = &event.text {
                            if !text.is_empty() {
                                pty.write(text.as_bytes());
                                surface.window().request_redraw();
                                return;
                            }
                        }
                    }

                    if let Some(bytes) = key_to_pty_bytes(&event.logical_key, modifiers) {
                        pty.write(&bytes);
                        surface.window().request_redraw();
                    }
                }
                _ => {}
            },
            _ => {}
        }
    })?;
    Ok(())
}

fn resize_terminal(
    surface: &mut Surface<winit::event_loop::OwnedDisplayHandle, Window>,
    parser: &Arc<Mutex<vt100::Parser>>,
    pty: &Arc<Pty>,
) -> Result<()> {
    let size = surface.window().inner_size();
    let width = size.width.max(1);
    let height = size.height.max(1);

    surface
        .resize(
            NonZeroU32::new(width).unwrap(),
            NonZeroU32::new(height).unwrap(),
        )
        .map_err(|e| anyhow::anyhow!("softbuffer resize failed: {e:?}"))?;

    let cols = (width as usize / CELL_W).max(1) as u16;
    let rows = (height as usize / CELL_H).max(1) as u16;

    if let Ok(mut parser) = parser.lock() {
        parser.set_size(rows, cols);
    }

    pty.resize(PtySize {
        rows,
        cols,
        pixel_width: width as u16,
        pixel_height: height as u16,
    });

    Ok(())
}
