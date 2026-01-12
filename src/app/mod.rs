//! Application state and event handling using winit's ApplicationHandler pattern.

pub mod actions;
pub mod graphics;
pub mod input;
pub mod layout;
pub mod pane;

use std::collections::HashMap;
use std::io::Read;
use std::thread;

use winit::application::ApplicationHandler;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};

use winit::window::WindowId;

use term::clipboard::{ClipboardProvider, SystemClipboard};
use term::config::{Action, Config};
use term::constants::{CELL_H, READ_BUFFER_SIZE};
use term::keys::key_to_pty_bytes;
use term::renderer::Renderer;

use graphics::GraphicsState;
use input::{InputState, apply_search_input, key_event_to_string};
use layout::{Divider, LayoutNode, PaneId, Rect};
use pane::Pane;

/// Custom events for the application.
#[derive(Debug)]
pub enum AppEvent {
    /// PTY has produced output bytes.
    PtyOutput { pane: PaneId, bytes: Vec<u8> },
    /// PTY has closed (shell exited).
    PtyExited { pane: PaneId },
}

/// Trait for opening URLs.
trait UrlOpener: Send + Sync {
    fn open(&self, url: &str) -> anyhow::Result<()>;
}

/// System URL opener implementation.
struct SystemUrlOpener;

impl UrlOpener for SystemUrlOpener {
    fn open(&self, url: &str) -> anyhow::Result<()> {
        open::that(url).map_err(anyhow::Error::from)
    }
}

/// Main application state.
pub struct App {
    pub config: Config,
    pub panes: HashMap<PaneId, Pane>,
    pub layout: LayoutNode,
    pub focused_pane: PaneId,
    pub focus_history: Vec<PaneId>,
    pub next_pane_id: PaneId,
    pub layout_dirty: bool,
    pub last_buffer_size: (u32, u32),

    pub renderer: Renderer,
    pub clipboard: Box<dyn ClipboardProvider>,
    url_opener: Box<dyn UrlOpener>,
    pub graphics: Option<GraphicsState>,
    pub input: InputState,
    pub event_proxy: Option<EventLoopProxy<AppEvent>>,
    pub show_help: bool,
    pub help_scroll: usize,
    pub help_max_scroll: usize,
    pub should_exit: bool,
}

impl App {
    /// Creates a new application with the given configuration.
    pub fn new(config: Config) -> Self {
        App {
            config,
            panes: HashMap::new(),
            layout: LayoutNode::Leaf(1),
            focused_pane: 1,
            focus_history: vec![1],
            next_pane_id: 2,
            layout_dirty: true,
            last_buffer_size: (0, 0),

            renderer: Renderer::new(),
            clipboard: Box::new(SystemClipboard::new()),
            url_opener: Box::new(SystemUrlOpener),
            graphics: None,
            input: InputState::default(),
            event_proxy: None,
            show_help: false,
            help_scroll: 0,
            help_max_scroll: 0,
            should_exit: false,
        }
    }

    /// Sets the event loop proxy for sending custom events.
    pub fn set_event_proxy(&mut self, proxy: EventLoopProxy<AppEvent>) {
        self.event_proxy = Some(proxy);
    }

    /// Computes pane rectangles and dividers for the current layout.
    pub fn pane_rects(
        &self,
        buffer_width: usize,
        buffer_height: usize,
    ) -> (Vec<(PaneId, Rect)>, Vec<Divider>) {
        let mut out = Vec::new();
        let mut dividers = Vec::new();
        let root = Rect {
            x: 0,
            y: 0,
            w: buffer_width,
            h: buffer_height,
        };
        self.layout.leaf_rects(root, &mut out, &mut dividers);
        (out, dividers)
    }

    /// Finds the pane at the given position.
    pub fn pane_at_position(
        &self,
        rects: &[(PaneId, Rect)],
        pos: PhysicalPosition<f64>,
    ) -> Option<(PaneId, Rect)> {
        rects
            .iter()
            .find(|(_id, r)| r.contains(pos.x, pos.y))
            .copied()
    }

    /// Initializes the first pane if none exist.
    fn initialize_first_pane(&mut self) {
        if !self.panes.is_empty() {
            return;
        }
        self.spawn_pane(1, self.config.font.scale);
    }

    /// Localizes a position relative to a pane rectangle.
    fn localize_pos(rect: Rect, pos: PhysicalPosition<f64>) -> PhysicalPosition<f64> {
        PhysicalPosition::new(pos.x - rect.x as f64, pos.y - rect.y as f64)
    }

    /// Handles keyboard input events.
    fn handle_keyboard(&mut self, event: &winit::event::KeyEvent) {
        if event.state != ElementState::Pressed {
            return;
        }

        let modifiers = self.input.modifiers;
        let ctrl = modifiers.control_key();
        let shift = modifiers.shift_key();
        let alt = modifiers.alt_key();

        let key_str = key_event_to_string(event);
        let action = key_str
            .as_deref()
            .and_then(|s| self.config.keybinds.get_action(s, ctrl, shift, alt));
        let non_search_action = action.filter(|a| !a.is_search_mode_only());

        // If the help overlay is open, capture navigation keys for scrolling.
        if self.show_help {
            if let Some(key) = key_str.as_deref() {
                match key {
                    "up" => {
                        self.help_scroll = self.help_scroll.saturating_sub(1);
                        self.request_redraw();
                        return;
                    }
                    "down" => {
                        self.help_scroll = (self.help_scroll + 1).min(self.help_max_scroll);
                        self.request_redraw();
                        return;
                    }
                    "pageup" => {
                        self.help_scroll = self.help_scroll.saturating_sub(10);
                        self.request_redraw();
                        return;
                    }
                    "pagedown" => {
                        self.help_scroll = (self.help_scroll + 10).min(self.help_max_scroll);
                        self.request_redraw();
                        return;
                    }
                    "home" => {
                        self.help_scroll = 0;
                        self.request_redraw();
                        return;
                    }
                    "end" => {
                        self.help_scroll = self.help_max_scroll;
                        self.request_redraw();
                        return;
                    }
                    _ => {}
                }
            }
        }

        // Pane actions are always handled, even while searching.
        if let Some(action) = non_search_action {
            if matches!(
                action,
                Action::SplitVertical
                    | Action::SplitHorizontal
                    | Action::FocusLeft
                    | Action::FocusRight
                    | Action::FocusUp
                    | Action::FocusDown
                    | Action::ResizeLeft
                    | Action::ResizeRight
                    | Action::ResizeUp
                    | Action::ResizeDown
                    | Action::ClosePane
                    | Action::ToggleHelp
            ) {
                self.execute_action(action);
                return;
            }
        }

        let mut needs_redraw = false;
        let mut action_to_execute: Option<Action> = None;

        {
            let Some(pane) = self.panes.get_mut(&self.focused_pane) else {
                return;
            };

            if pane.view.is_search_active() {
                needs_redraw |= apply_search_input(&mut pane.view, modifiers, action, event);
            } else if let Some(action) = non_search_action {
                action_to_execute = Some(action);
            } else {
                // Regular text input.
                if !ctrl && !alt {
                    if let Some(text) = &event.text {
                        if !text.is_empty() {
                            pane.view.scrollback_snap_to_bottom();
                            pane.terminal.write(text.as_bytes());
                            needs_redraw = true;
                        }
                    }
                }

                if !needs_redraw {
                    if let Some(bytes) = key_to_pty_bytes(&event.logical_key, modifiers) {
                        pane.view.scrollback_snap_to_bottom();
                        pane.terminal.write(&bytes);
                        needs_redraw = true;
                    }
                }
            }
        }

        if let Some(action) = action_to_execute {
            self.execute_action(action);
            return;
        }

        if needs_redraw {
            self.request_redraw();
        }
    }

    /// Handles mouse button events.
    fn handle_mouse_button(&mut self, state: ElementState, button: MouseButton) {
        if button != MouseButton::Left {
            return;
        }

        let (buffer_width, buffer_height) = self.last_buffer_size;
        if buffer_width == 0 || buffer_height == 0 {
            return;
        }

        let (rects, _divs) = self.pane_rects(buffer_width as usize, buffer_height as usize);
        let Some((pane_id, pane_rect)) = self.pane_at_position(&rects, self.input.cursor_position)
        else {
            return;
        };

        self.set_focus(pane_id);

        let Some(pane) = self.panes.get_mut(&pane_id) else {
            return;
        };

        let local = Self::localize_pos(pane_rect, self.input.cursor_position);

        match state {
            ElementState::Pressed => {
                let (cell_w, cell_h) = Self::cell_size_for_scale(pane.scale);
                if let Some((row, col)) = pane.view.window_to_cell(local.x, local.y, cell_w, cell_h)
                {
                    if let Some(url) = pane.view.url_at(row, col) {
                        if let Err(e) = self.url_opener.open(&url) {
                            eprintln!("Failed to open URL: {e}");
                        }
                        return;
                    }

                    self.input.mouse_selecting = true;
                    pane.view.start_selection(row, col);
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

        let (buffer_width, buffer_height) = self.last_buffer_size;
        if buffer_width == 0 || buffer_height == 0 {
            self.update_cursor();
            return;
        }

        let (rects, _divs) = self.pane_rects(buffer_width as usize, buffer_height as usize);
        let Some((pane_id, pane_rect)) = self.pane_at_position(&rects, position) else {
            self.update_cursor();
            return;
        };

        let Some(pane) = self.panes.get_mut(&pane_id) else {
            return;
        };

        let local = Self::localize_pos(pane_rect, position);

        let (cell_w, cell_h) = Self::cell_size_for_scale(pane.scale);

        if self.input.mouse_selecting && pane_id == self.focused_pane {
            if let Some((row, col)) = pane.view.window_to_cell(local.x, local.y, cell_w, cell_h) {
                pane.view.update_selection(row, col);
                self.request_redraw();
            }
        } else {
            if let Some((row, col)) = pane.view.window_to_cell(local.x, local.y, cell_w, cell_h) {
                if pane.view.update_url_hover(row, col) {
                    self.request_redraw();
                }
            } else {
                pane.view.clear_url_hover();
            }
            self.update_cursor();
        }
    }

    /// Handles mouse wheel scrolling.
    fn handle_scroll(&mut self, delta: MouseScrollDelta) {
        let lines = match delta {
            MouseScrollDelta::LineDelta(_, y) => {
                (y * self.config.terminal.scroll_speed).round() as isize
            }
            MouseScrollDelta::PixelDelta(pos) => {
                // Use the focused pane's cell height as a reasonable heuristic.
                let cell_h = self
                    .panes
                    .get(&self.focused_pane)
                    .map(|p| Self::cell_size_for_scale(p.scale).1)
                    .unwrap_or(CELL_H);
                (pos.y / cell_h as f64).round() as isize
            }
        };

        if lines == 0 {
            return;
        }

        // When the help overlay is open, scroll it instead of the terminal.
        if self.show_help {
            if lines > 0 {
                self.help_scroll = self.help_scroll.saturating_sub(lines as usize);
            } else {
                self.help_scroll = (self.help_scroll + (-lines) as usize).min(self.help_max_scroll);
            }
            self.request_redraw();
            return;
        }

        let (buffer_width, buffer_height) = self.last_buffer_size;
        let target = if buffer_width > 0 && buffer_height > 0 {
            let (rects, _divs) = self.pane_rects(buffer_width as usize, buffer_height as usize);
            self.pane_at_position(&rects, self.input.cursor_position)
                .map(|(id, _)| id)
                .unwrap_or(self.focused_pane)
        } else {
            self.focused_pane
        };

        if let Some(pane) = self.panes.get_mut(&target) {
            pane.view.scrollback_scroll_by(lines);
            self.request_redraw();
        }
    }
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.initialize_first_pane();

        if self.graphics.is_none() {
            self.create_window(event_loop);
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::PtyOutput { pane, bytes } => {
                if let Some(p) = self.panes.get(&pane) {
                    p.terminal.process(&bytes);
                }
                self.request_redraw();
            }
            AppEvent::PtyExited { pane } => {
                if self.panes.contains_key(&pane) {
                    self.close_pane(pane);
                }
                if self.should_exit {
                    event_loop.exit();
                }
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
                if self.should_exit {
                    event_loop.exit();
                }
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
pub fn spawn_pty_reader(
    mut reader: Box<dyn Read + Send>,
    proxy: EventLoopProxy<AppEvent>,
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
                pane,
                bytes: buf[..n].to_vec(),
            });
        }

        let _ = proxy.send_event(AppEvent::PtyExited { pane });
    });
}
