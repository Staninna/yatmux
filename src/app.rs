//! Application state and event handling using winit's ApplicationHandler pattern.

use std::io::Read;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::thread;

use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowId};

use crate::clipboard::{read_clipboard_text, write_clipboard_text};
use crate::config::{Action, Config};
use crate::constants::{CELL_H, CELL_W, READ_BUFFER_SIZE};
use crate::keys::key_to_pty_bytes;
use crate::renderer::{FontStyle, Renderer, create_palette};
use crate::terminal::Terminal;

/// Custom events for the application.
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// PTY has new output to display.
    PtyOutput,
}

/// Input state for mouse and keyboard.
struct InputState {
    cursor_position: PhysicalPosition<f64>,
    mouse_selecting: bool,
    modifiers: ModifiersState,
}

impl Default for InputState {
    fn default() -> Self {
        InputState {
            cursor_position: PhysicalPosition::new(0.0, 0.0),
            mouse_selecting: false,
            modifiers: ModifiersState::empty(),
        }
    }
}

/// Graphics state for rendering.
struct GraphicsState {
    #[allow(dead_code)]
    context: Context<winit::event_loop::OwnedDisplayHandle>,
    surface: Surface<winit::event_loop::OwnedDisplayHandle, Window>,
    palette: Arc<[u32; 256]>,
}

/// Main application state.
pub struct App {
    config: Config,
    terminal: Option<Terminal>,
    renderer: Renderer,
    graphics: Option<GraphicsState>,
    input: InputState,
    event_proxy: Option<EventLoopProxy<AppEvent>>,
}

impl App {
    /// Creates a new application with the given configuration.
    pub fn new(config: Config) -> Self {
        App {
            config,
            terminal: None,
            renderer: Renderer::new(),
            graphics: None,
            input: InputState::default(),
            event_proxy: None,
        }
    }

    /// Sets the event loop proxy for sending custom events.
    pub fn set_event_proxy(&mut self, proxy: EventLoopProxy<AppEvent>) {
        self.event_proxy = Some(proxy);
    }

    /// Initializes the terminal and starts the PTY reader thread.
    fn initialize_terminal(&mut self) {
        let (pty, reader) = match crate::pty::spawn_shell() {
            Ok(result) => result,
            Err(e) => {
                eprintln!("Failed to spawn shell: {e}");
                return;
            }
        };

        let terminal = Terminal::new(Arc::new(pty));

        // Start PTY reader thread
        if let Some(proxy) = &self.event_proxy {
            spawn_pty_reader(reader, terminal.parser(), proxy.clone());
        }

        self.terminal = Some(terminal);
    }

    /// Creates the window and graphics context.
    fn create_window(&mut self, event_loop: &ActiveEventLoop) {
        let window_attrs = Window::default_attributes().with_title(&self.config.window.title);

        let window = match event_loop.create_window(window_attrs) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("Failed to create window: {e}");
                return;
            }
        };

        let display = event_loop.owned_display_handle();
        let context = match Context::new(display) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to create graphics context: {e:?}");
                return;
            }
        };

        let surface = match Surface::new(&context, window) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to create surface: {e:?}");
                return;
            }
        };

        let palette = Arc::new(create_palette());

        self.graphics = Some(GraphicsState {
            context,
            surface,
            palette,
        });

        // Initial resize
        self.handle_resize();
        self.request_redraw();
    }

    /// Handles window resize events.
    fn handle_resize(&mut self) {
        let Some(graphics) = &mut self.graphics else {
            return;
        };
        let Some(terminal) = &self.terminal else {
            return;
        };

        let size = graphics.surface.window().inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        if let Err(e) = graphics.surface.resize(
            NonZeroU32::new(width).expect("width >= 1"),
            NonZeroU32::new(height).expect("height >= 1"),
        ) {
            eprintln!("Surface resize failed: {e:?}");
            return;
        }

        terminal.resize(width, height, CELL_W, CELL_H);
    }

    /// Renders the terminal.
    fn render(&mut self) {
        let Some(graphics) = &mut self.graphics else {
            return;
        };
        let Some(terminal) = &self.terminal else {
            return;
        };

        if let Err(e) =
            self.renderer
                .render(&mut graphics.surface, &terminal.parser(), &graphics.palette)
        {
            eprintln!("Render error: {e:#}");
        }
    }

    /// Requests a window redraw.
    fn request_redraw(&self) {
        if let Some(graphics) = &self.graphics {
            graphics.surface.window().request_redraw();
        }
    }

    /// Converts a winit key to a string for keybind matching.
    fn key_to_string(key: &Key) -> Option<String> {
        match key {
            Key::Character(c) => Some(c.to_lowercase()),
            Key::Named(named) => {
                let name = match named {
                    NamedKey::Enter => "enter",
                    NamedKey::Tab => "tab",
                    NamedKey::Space => "space",
                    NamedKey::Backspace => "backspace",
                    NamedKey::Escape => "escape",
                    NamedKey::Insert => "insert",
                    NamedKey::Delete => "delete",
                    NamedKey::Home => "home",
                    NamedKey::End => "end",
                    NamedKey::PageUp => "pageup",
                    NamedKey::PageDown => "pagedown",
                    NamedKey::ArrowUp => "up",
                    NamedKey::ArrowDown => "down",
                    NamedKey::ArrowLeft => "left",
                    NamedKey::ArrowRight => "right",
                    NamedKey::F1 => "f1",
                    NamedKey::F2 => "f2",
                    NamedKey::F3 => "f3",
                    NamedKey::F4 => "f4",
                    NamedKey::F5 => "f5",
                    NamedKey::F6 => "f6",
                    NamedKey::F7 => "f7",
                    NamedKey::F8 => "f8",
                    NamedKey::F9 => "f9",
                    NamedKey::F10 => "f10",
                    NamedKey::F11 => "f11",
                    NamedKey::F12 => "f12",
                    _ => return None,
                };
                Some(name.to_string())
            }
            _ => None,
        }
    }

    /// Handles keyboard input events.
    fn handle_keyboard(&mut self, event: &winit::event::KeyEvent) {
        if event.state != ElementState::Pressed {
            return;
        }

        let Some(terminal) = &self.terminal else {
            return;
        };

        // Check for configured keybinds
        if let Some(key_str) = Self::key_to_string(&event.logical_key) {
            let ctrl = self.input.modifiers.control_key();
            let shift = self.input.modifiers.shift_key();
            let alt = self.input.modifiers.alt_key();

            if let Some(action) = self.config.keybinds.get_action(&key_str, ctrl, shift, alt) {
                self.execute_action(action);
                return;
            }
        }

        // Regular text input (when no modifier or just shift)
        if !self.input.modifiers.control_key() && !self.input.modifiers.alt_key() {
            if let Some(text) = &event.text {
                if !text.is_empty() {
                    terminal.write(text.as_bytes());
                    self.request_redraw();
                    return;
                }
            }
        }

        // Special keys (arrows, etc.) that need escape sequences
        if let Some(bytes) = key_to_pty_bytes(&event.logical_key, self.input.modifiers) {
            terminal.write(&bytes);
            self.request_redraw();
        }
    }

    /// Executes a configured action.
    fn execute_action(&mut self, action: Action) {
        match action {
            Action::Copy => self.handle_copy(),
            Action::Paste => self.handle_paste(),
            Action::FontCycle => self.handle_font_cycle(),
            Action::ScrollPageUp => {
                self.renderer.scrollback_scroll_by(24); // Approximate page
                self.request_redraw();
            }
            Action::ScrollPageDown => {
                self.renderer.scrollback_scroll_by(-24);
                self.request_redraw();
            }
            Action::ScrollLineUp => {
                self.renderer.scrollback_scroll_by(1);
                self.request_redraw();
            }
            Action::ScrollLineDown => {
                self.renderer.scrollback_scroll_by(-1);
                self.request_redraw();
            }
            Action::ScrollToTop => {
                self.renderer.scrollback_scroll_by(isize::MAX);
                self.request_redraw();
            }
            Action::ScrollToBottom => {
                self.renderer.scrollback_scroll_by(isize::MIN);
                self.request_redraw();
            }
            Action::ClearScrollback => {
                self.renderer.clear_scrollback();
                self.request_redraw();
            }
            Action::Reset => {
                // Reset terminal state - could be expanded
                self.renderer.clear_scrollback();
                self.renderer.clear_selection();
                self.request_redraw();
            }
        }
    }

    /// Handles paste from clipboard.
    fn handle_paste(&mut self) {
        if let Some(terminal) = &self.terminal {
            if let Some(text) = read_clipboard_text() {
                if !text.is_empty() {
                    terminal.write(text.as_bytes());
                    self.request_redraw();
                }
            }
        }
    }

    /// Handles font cycling.
    fn handle_font_cycle(&mut self) {
        let current = self.renderer.font_style();
        let new_style = current.next();
        self.renderer.set_font_style(new_style);

        eprintln!("Font switched to: {:?}", new_style);

        if let Some(terminal) = &self.terminal {
            let msg = match new_style {
                FontStyle::BoxDrawing => b"+---+\r\n|   |\r\n+---+\r\n".as_slice(),
                FontStyle::Greek => b"alpha beta gamma delta epsilon\r\n".as_slice(),
                FontStyle::Block => b"blocks test mode\r\n".as_slice(),
                FontStyle::Hiragana => b"hiragana test mode\r\n".as_slice(),
                _ => b"switched font mode\r\n".as_slice(),
            };
            terminal.write(msg);
        }

        self.request_redraw();
    }

    /// Handles copy to clipboard.
    fn handle_copy(&mut self) {
        let selection = self.renderer.get_selection_bounds();
        if let Some(terminal) = &self.terminal {
            if let Some(text) = terminal.get_selected_text(selection) {
                if write_clipboard_text(&text) {
                    eprintln!("Copied {} characters to clipboard", text.len());
                }
            }
        }
    }

    /// Handles mouse button events.
    fn handle_mouse_button(&mut self, state: ElementState, button: MouseButton) {
        if button != MouseButton::Left {
            return;
        }

        match state {
            ElementState::Pressed => {
                self.input.mouse_selecting = true;
                if let Some((row, col)) = self
                    .renderer
                    .window_to_cell(self.input.cursor_position.x, self.input.cursor_position.y)
                {
                    self.renderer.start_selection(row, col);
                    self.request_redraw();
                }
            }
            ElementState::Released => {
                self.input.mouse_selecting = false;
            }
        }
    }

    /// Handles mouse movement.
    fn handle_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        self.input.cursor_position = position;

        if self.input.mouse_selecting {
            if let Some((row, col)) = self.renderer.window_to_cell(position.x, position.y) {
                self.renderer.update_selection(row, col);
                self.request_redraw();
            }
        }
    }

    /// Handles mouse wheel scrolling.
    fn handle_scroll(&mut self, delta: MouseScrollDelta) {
        let lines = match delta {
            MouseScrollDelta::LineDelta(_, y) => {
                (y * self.config.terminal.scroll_speed).round() as isize
            }
            MouseScrollDelta::PixelDelta(pos) => (pos.y / CELL_H as f64).round() as isize,
        };

        if lines != 0 {
            self.renderer.scrollback_scroll_by(lines);
            self.request_redraw();
        }
    }
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Initialize terminal if not already done
        if self.terminal.is_none() {
            self.initialize_terminal();
        }

        // Create window if not already done
        if self.graphics.is_none() {
            self.create_window(event_loop);
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::PtyOutput => {
                self.request_redraw();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::RedrawRequested => {
                self.render();
            }

            WindowEvent::Resized(_) => {
                self.handle_resize();
                self.request_redraw();
            }

            WindowEvent::ModifiersChanged(mods) => {
                self.input.modifiers = mods.state();
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.handle_cursor_moved(position);
            }

            WindowEvent::MouseInput { state, button, .. } => {
                self.handle_mouse_button(state, button);
            }

            WindowEvent::MouseWheel { delta, .. } => {
                self.handle_scroll(delta);
            }

            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard(&event);
            }

            _ => {}
        }
    }
}

/// Spawns a thread to read PTY output and send events.
fn spawn_pty_reader(
    mut reader: Box<dyn Read + Send>,
    parser: std::sync::Arc<std::sync::Mutex<vt100::Parser>>,
    proxy: EventLoopProxy<AppEvent>,
) {
    thread::spawn(move || {
        let mut buf = [0u8; READ_BUFFER_SIZE];
        loop {
            let n = match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            };

            if let Ok(mut p) = parser.lock() {
                p.process(&buf[..n]);
            }

            let _ = proxy.send_event(AppEvent::PtyOutput);
        }
    });
}
