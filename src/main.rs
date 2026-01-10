use std::{
    io::{Read, Write},
    num::NonZeroU32,
    sync::{Arc, Mutex},
    thread,
};

use anyhow::{Context as _, Result};
use font8x8::UnicodeFonts as _;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use softbuffer::{Context, Surface};
use winit::{
    event::{ElementState, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, ModifiersState, NamedKey},
    window::Window,
};

#[derive(Debug, Clone)]
enum UserEvent {
    PtyUpdated,
}

const FONT_SCALE: usize = 2;
const GLYPH_W: usize = 8;
const GLYPH_H: usize = 8;
const CELL_W: usize = GLYPH_W * FONT_SCALE;
const CELL_H: usize = GLYPH_H * FONT_SCALE;

fn main() -> Result<()> {
    let (pty, pty_reader) = spawn_shell()?;
    let pty = Arc::new(pty);
    let parser = Arc::new(Mutex::new(vt100::Parser::new(24, 80, 0)));

    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();

    {
        let parser = Arc::clone(&parser);
        let mut reader = pty_reader;
        thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                let read_len = match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(_) => break,
                };
                {
                    let mut parser = parser.lock().ok().unwrap();
                    parser.process(&buf[..read_len]);
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

    let palette = color_palette();

    resize_terminal(&mut surface, &parser, &pty)?;
    surface.window().request_redraw();

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        match event {
            Event::UserEvent(UserEvent::PtyUpdated) => surface.window().request_redraw(),
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::RedrawRequested => {
                    if let Err(err) = render(&mut surface, &parser, &palette) {
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

fn render(
    surface: &mut Surface<winit::event_loop::OwnedDisplayHandle, Window>,
    parser: &Arc<Mutex<vt100::Parser>>,
    palette: &[u32; 256],
) -> Result<()> {
    let mut buffer = surface
        .buffer_mut()
        .map_err(|e| anyhow::anyhow!("softbuffer buffer_mut failed: {e:?}"))?;
    let buffer_width = buffer.width().get() as usize;
    let buffer_height = buffer.height().get() as usize;
    buffer.fill(0x00_10_10_10);

    let (contents, cursor, screen_cells) = {
        let parser = parser.lock().unwrap();
        let screen = parser.screen();
        let contents = screen.contents();
        let cursor = screen.cursor_position();

        // Pre-fetch all cell data to avoid multiple mutex locks
        let rows = buffer_height / CELL_H;
        let cols = buffer_width / CELL_W;
        let mut cells = Vec::with_capacity(rows * cols);
        for row in 0..rows {
            for col in 0..cols {
                let cell = screen.cell(row as u16, col as u16);
                let fg = cell.map(|c| c.fgcolor()).unwrap_or(vt100::Color::Default);
                let bg = cell.map(|c| c.bgcolor()).unwrap_or(vt100::Color::Default);
                cells.push((fg, bg));
            }
        }
        (contents, cursor, cells)
    };

    let rows = buffer_height / CELL_H;
    let cols = buffer_width / CELL_W;

    for row in 0..rows {
        for col in 0..cols {
            let ch = contents.chars().nth(row * 80 + col).unwrap_or(' ');
            let invert = (row as u16, col as u16) == cursor;
            let (fg, bg) = screen_cells[row * cols + col];
            draw_cell(
                &mut buffer,
                buffer_width,
                buffer_height,
                row,
                col,
                ch,
                invert,
                fg,
                bg,
                palette,
            );
        }
    }

    buffer
        .present()
        .map_err(|e| anyhow::anyhow!("softbuffer present failed: {e:?}"))?;
    Ok(())
}

fn color_to_u32(color: vt100::Color, is_default: u32, palette: &[u32; 256]) -> u32 {
    match color {
        vt100::Color::Default => is_default,
        vt100::Color::Idx(n) => palette[n as usize],
        vt100::Color::Rgb(r, g, b) => 0x00 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32),
    }
}

fn color_palette() -> [u32; 256] {
    let mut palette = [0u32; 256];
    palette[0] = 0x00_00_00_00;
    palette[1] = 0x00_80_00_00;
    palette[2] = 0x00_00_80_00;
    palette[3] = 0x00_80_80_00;
    palette[4] = 0x00_00_00_80;
    palette[5] = 0x00_80_00_80;
    palette[6] = 0x00_00_FF_FF;
    palette[7] = 0x00_C0_C0_C0;
    palette[8] = 0x00_80_80_80;
    palette[9] = 0x00_FF_00_00;
    palette[10] = 0x00_00_FF_00;
    palette[11] = 0x00_FF_FF_00;
    palette[12] = 0x00_00_00_FF;
    palette[13] = 0x00_FF_00_FF;
    palette[14] = 0x00_00_FF_FF;
    palette[15] = 0x00_FF_FF_FF;
    for i in 16..24 {
        let r = 8 + (i - 16) * 10;
        palette[i] = 0x00 | ((r as u32) << 16) | ((r as u32) << 8) | (r as u32);
    }
    for i in 24..232 {
        let idx = i - 24;
        let r = 40 + (idx / 36) * 40;
        let g = 40 + ((idx / 6) % 6) * 40;
        let b = 40 + (idx % 6) * 40;
        palette[i] = 0x00 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
    }
    for i in 232..256 {
        let gray = 8 + (i - 232) * 10;
        palette[i] = 0x00 | ((gray as u32) << 16) | ((gray as u32) << 8) | (gray as u32);
    }
    palette
}

fn draw_cell(
    backbuffer: &mut [u32],
    width: usize,
    height: usize,
    row: usize,
    col: usize,
    ch: char,
    invert: bool,
    fg_color: vt100::Color,
    bg_color: vt100::Color,
    palette: &[u32; 256],
) {
    let default_bg = 0x00_10_10_10;
    let default_fg = 0x00_D0_D0_D0;

    let fg = color_to_u32(fg_color, default_fg, palette);
    let bg = color_to_u32(bg_color, default_bg, palette);

    let bg = if invert { fg } else { bg };
    let fg = if invert { bg } else { fg };

    let x0 = col * CELL_W;
    let y0 = row * CELL_H;

    for y in y0..(y0 + CELL_H).min(height) {
        for x in x0..(x0 + CELL_W).min(width) {
            backbuffer[y * width + x] = bg;
        }
    }

    let glyph = font8x8::BASIC_FONTS.get(ch).unwrap_or([0; 8]);
    for gy in 0..GLYPH_H {
        let bits = glyph[gy];
        for gx in 0..GLYPH_W {
            let on = (bits >> gx) & 1 == 1;
            if !on {
                continue;
            }

            for sy in 0..FONT_SCALE {
                for sx in 0..FONT_SCALE {
                    let x = x0 + gx * FONT_SCALE + sx;
                    let y = y0 + gy * FONT_SCALE + sy;
                    if x < width && y < height {
                        backbuffer[y * width + x] = fg;
                    }
                }
            }
        }
    }
}

fn key_to_pty_bytes(key: &Key, mods: ModifiersState) -> Option<Vec<u8>> {
    let ctrl = mods.control_key();

    match key {
        Key::Named(NamedKey::Enter) => Some(b"\r".to_vec()),
        Key::Named(NamedKey::Tab) => Some(b"\t".to_vec()),
        Key::Named(NamedKey::Space) => {
            if ctrl {
                Some(vec![0x00])
            } else {
                Some(b" ".to_vec())
            }
        }
        Key::Named(NamedKey::Backspace) => Some(vec![0x7f]),
        Key::Named(NamedKey::Escape) => Some(vec![0x1b]),
        Key::Named(NamedKey::ArrowUp) => Some(b"\x1b[A".to_vec()),
        Key::Named(NamedKey::ArrowDown) => Some(b"\x1b[B".to_vec()),
        Key::Named(NamedKey::ArrowRight) => Some(b"\x1b[C".to_vec()),
        Key::Named(NamedKey::ArrowLeft) => Some(b"\x1b[D".to_vec()),
        Key::Named(NamedKey::Home) => Some(b"\x1b[H".to_vec()),
        Key::Named(NamedKey::End) => Some(b"\x1b[F".to_vec()),
        Key::Named(NamedKey::PageUp) => Some(b"\x1b[5~".to_vec()),
        Key::Named(NamedKey::PageDown) => Some(b"\x1b[6~".to_vec()),
        Key::Named(NamedKey::Delete) => Some(b"\x1b[3~".to_vec()),
        Key::Character(s) => {
            let mut chars = s.chars();
            let ch = chars.next()?;
            if chars.next().is_some() {
                return Some(s.as_bytes().to_vec());
            }
            if ctrl {
                let c = ch.to_ascii_lowercase() as u8;
                if (b'a'..=b'z').contains(&c) {
                    return Some(vec![c - b'a' + 1]);
                }
            }
            Some(ch.to_string().into_bytes())
        }
        _ => None,
    }
}

struct Pty {
    master: Mutex<Box<dyn portable_pty::MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl Pty {
    fn write(&self, bytes: &[u8]) {
        if let Ok(mut writer) = self.writer.lock() {
            let _ = writer.write_all(bytes);
            let _ = writer.flush();
        }
    }

    fn resize(&self, size: PtySize) {
        if let Ok(master) = self.master.lock() {
            let _ = master.resize(size);
        }
    }
}

fn spawn_shell() -> Result<(Pty, Box<dyn Read + Send>)> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
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
