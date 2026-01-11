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
use winit::window::{CursorIcon, Window, WindowId};

use term::clipboard::{ClipboardProvider, SystemClipboard};
use term::config::{Action, Config};
use term::constants::{CELL_H, CELL_W, READ_BUFFER_SIZE};
use term::keys::key_to_pty_bytes;
use term::renderer::{Renderer, TerminalView, create_palette};
use term::terminal::Terminal;

/// Custom events for the application.
#[derive(Debug)]
pub enum AppEvent {
    /// PTY has produced output bytes.
    PtyOutput(Vec<u8>),
}

trait UrlOpener: Send + Sync {
    fn open(&self, url: &str) -> anyhow::Result<()>;
}

struct SystemUrlOpener;

impl UrlOpener for SystemUrlOpener {
    fn open(&self, url: &str) -> anyhow::Result<()> {
        open::that(url).map_err(anyhow::Error::from)
    }
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
    view: TerminalView,
    renderer: Renderer,
    clipboard: Box<dyn ClipboardProvider>,
    url_opener: Box<dyn UrlOpener>,
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
            view: TerminalView::new(),
            renderer: Renderer::new(),
            clipboard: Box::new(SystemClipboard::new()),
            url_opener: Box::new(SystemUrlOpener),
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
        let (pty, reader) = match term::pty::spawn_shell() {
            Ok(result) => result,
            Err(e) => {
                eprintln!("Failed to spawn shell: {e}");
                return;
            }
        };

        let terminal = Terminal::new_with_scrollback(
            Arc::new(pty),
            self.config.terminal.scrollback_lines as usize,
        );

        // Start PTY reader thread
        if let Some(proxy) = &self.event_proxy {
            spawn_pty_reader(reader, proxy.clone());
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

        // IMPORTANT: compute terminal size from the actual render buffer.
        // On some platforms/scales, `inner_size()` can diverge from the buffer size,
        // leading to a PTY that thinks it's only a few columns wide.
        let (buffer_width, buffer_height) = match graphics.surface.buffer_mut() {
            Ok(buffer) => (buffer.width().get(), buffer.height().get()),
            Err(e) => {
                eprintln!("softbuffer buffer_mut failed during resize: {e:?}");
                return;
            }
        };

        terminal.resize(buffer_width, buffer_height, CELL_W, CELL_H);
    }

    /// Renders the terminal.
    fn render(&mut self) {
        let Some(graphics) = &mut self.graphics else {
            return;
        };
        let Some(terminal) = &self.terminal else {
            return;
        };

        if let Err(e) = self.renderer.render(
            &mut graphics.surface,
            terminal,
            &graphics.palette,
            &mut self.view,
        ) {
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

        // Handle search mode input first
        if self.view.is_search_active() {
            self.handle_search_keyboard(event);
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
                // Skip search-mode-only actions when not in search mode
                if !action.is_search_mode_only() {
                    self.execute_action(action);
                    return;
                }
            }
        }

        // Regular text input (when no modifier or just shift)
        if !self.input.modifiers.control_key() && !self.input.modifiers.alt_key() {
            if let Some(text) = &event.text {
                if !text.is_empty() {
                    // Snap to bottom when user types
                    self.view.scrollback_snap_to_bottom();
                    terminal.write(text.as_bytes());
                    self.request_redraw();
                    return;
                }
            }
        }

        // Special keys (arrows, etc.) that need escape sequences
        if let Some(bytes) = key_to_pty_bytes(&event.logical_key, self.input.modifiers) {
            // Snap to bottom when user types
            self.view.scrollback_snap_to_bottom();
            terminal.write(&bytes);
            self.request_redraw();
        }
    }

    /// Handles keyboard input when search mode is active.
    fn handle_search_keyboard(&mut self, event: &winit::event::KeyEvent) {
        // Check for configurable keybinds first
        if let Some(key_str) = Self::key_to_string(&event.logical_key) {
            let ctrl = self.input.modifiers.control_key();
            let shift = self.input.modifiers.shift_key();
            let alt = self.input.modifiers.alt_key();

            if let Some(action) = self.config.keybinds.get_action(&key_str, ctrl, shift, alt) {
                match action {
                    Action::SearchClose => {
                        self.view.deactivate_search();
                        self.request_redraw();
                        return;
                    }
                    Action::SearchConfirm => {
                        if self.view.search_match_count() > 0 {
                            self.view.search_next();
                        }
                        self.request_redraw();
                        return;
                    }
                    Action::SearchNext => {
                        self.view.search_next();
                        self.request_redraw();
                        return;
                    }
                    Action::SearchPrev => {
                        self.view.search_prev();
                        self.request_redraw();
                        return;
                    }
                    Action::SearchToggleCase => {
                        self.view.search_toggle_case();
                        self.request_redraw();
                        return;
                    }
                    // Other actions are not relevant in search mode
                    _ => {}
                }
            }
        }

        // Handle text input for search query
        match &event.logical_key {
            Key::Named(NamedKey::Backspace) => {
                self.view.search_pop_char();
                self.request_redraw();
            }
            Key::Character(s) => {
                if !self.input.modifiers.control_key() && !self.input.modifiers.alt_key() {
                    for ch in s.chars() {
                        self.view.search_push_char(ch);
                    }
                    self.request_redraw();
                }
            }
            _ => {}
        }
    }

    /// Executes a configured action.
    fn execute_action(&mut self, action: Action) {
        match action {
            Action::Copy => self.handle_copy(),
            Action::Paste => self.handle_paste(),
            Action::ScrollPageUp => {
                self.view.scrollback_scroll_by(24); // Approximate page
                self.request_redraw();
            }
            Action::ScrollPageDown => {
                self.view.scrollback_scroll_by(-24);
                self.request_redraw();
            }
            Action::ScrollLineUp => {
                self.view.scrollback_scroll_by(1);
                self.request_redraw();
            }
            Action::ScrollLineDown => {
                self.view.scrollback_scroll_by(-1);
                self.request_redraw();
            }
            Action::ScrollToTop => {
                self.view.scrollback_scroll_by(isize::MAX);
                self.request_redraw();
            }
            Action::ScrollToBottom => {
                self.view.scrollback_scroll_by(isize::MIN);
                self.request_redraw();
            }
            Action::ClearScrollback => {
                if let Some(terminal) = &self.terminal {
                    terminal.clear_scrollback();
                }
                self.view.clear_scrollback();
                self.request_redraw();
            }
            Action::Reset => {
                if let Some(terminal) = &self.terminal {
                    terminal.clear_scrollback();
                }
                self.view.clear_scrollback();
                self.view.clear_selection();
                self.request_redraw();
            }
            Action::SearchFind => {
                self.view.activate_search();
                self.request_redraw();
            }
            // Search mode actions - only meaningful when search is active
            Action::SearchClose => {
                self.view.deactivate_search();
                self.request_redraw();
            }
            Action::SearchNext => {
                self.view.search_next();
                self.request_redraw();
            }
            Action::SearchPrev => {
                self.view.search_prev();
                self.request_redraw();
            }
            Action::SearchToggleCase => {
                self.view.search_toggle_case();
                self.request_redraw();
            }
            Action::SearchConfirm => {
                if self.view.search_match_count() > 0 {
                    self.view.search_next();
                }
                self.request_redraw();
            }
        }
    }

    /// Handles paste from clipboard.
    fn handle_paste(&mut self) {
        if let Some(terminal) = &self.terminal {
            if let Some(text) = self.clipboard.read() {
                if !text.is_empty() {
                    terminal.write(text.as_bytes());
                    self.request_redraw();
                }
            }
        }
    }

    /// Handles copy to clipboard.
    fn handle_copy(&mut self) {
        if let Some(text) = self.view.get_selected_text() {
            if self.clipboard.write(&text) {
                eprintln!("Copied {} characters to clipboard", text.len());
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
                // Check for URL click first
                if let Some((row, col)) = self
                    .view
                    .window_to_cell(self.input.cursor_position.x, self.input.cursor_position.y)
                {
                    if let Some(url) = self.view.url_at(row, col) {
                        // Open URL in browser
                        if let Err(e) = self.url_opener.open(&url) {
                            eprintln!("Failed to open URL: {e}");
                        }
                        return;
                    }

                    // Start selection
                    self.input.mouse_selecting = true;
                    self.view.start_selection(row, col);
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
            if let Some((row, col)) = self.view.window_to_cell(position.x, position.y) {
                self.view.update_selection(row, col);
                self.request_redraw();
            }
        } else {
            // Update URL hover state
            if let Some((row, col)) = self.view.window_to_cell(position.x, position.y) {
                if self.view.update_url_hover(row, col) {
                    self.request_redraw();
                    self.update_cursor();
                }
            } else {
                self.view.clear_url_hover();
                self.update_cursor();
            }
        }
    }

    /// Updates the cursor icon based on hover state.
    fn update_cursor(&self) {
        if let Some(graphics) = &self.graphics {
            let cursor = if self.view.has_hovered_url() {
                CursorIcon::Pointer
            } else {
                CursorIcon::Text
            };
            graphics.surface.window().set_cursor(cursor);
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
            self.view.scrollback_scroll_by(lines);
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
            AppEvent::PtyOutput(bytes) => {
                if let Some(terminal) = &self.terminal {
                    terminal.process(&bytes);
                }
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
fn spawn_pty_reader(mut reader: Box<dyn Read + Send>, proxy: EventLoopProxy<AppEvent>) {
    thread::spawn(move || {
        let mut buf = [0u8; READ_BUFFER_SIZE];
        loop {
            let n = match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            };

            let _ = proxy.send_event(AppEvent::PtyOutput(buf[..n].to_vec()));
        }
    });
}
