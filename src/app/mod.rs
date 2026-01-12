//! Application state and event handling using winit's ApplicationHandler pattern.

pub mod actions;
pub mod graphics;
pub mod input;
pub mod layout;
pub mod pane;
pub mod tab;

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
use layout::{PaneId, Rect};
use pane::Pane;
use tab::{Tab, TabId};

/// Custom events for the application.
#[derive(Debug)]
pub enum AppEvent {
    /// PTY has produced output bytes.
    PtyOutput {
        tab: TabId,
        pane: PaneId,
        bytes: Vec<u8>,
    },
    /// PTY has closed (shell exited).
    PtyExited { tab: TabId, pane: PaneId },
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

    // Tab management
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    pub next_tab_id: TabId,

    // Global state
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
            tabs: Vec::new(),
            active_tab: 0,
            next_tab_id: 1,
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

    /// Returns a reference to the active tab.
    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active_tab)
    }

    /// Returns a mutable reference to the active tab.
    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active_tab)
    }

    /// Returns the active tab's focused pane ID, if any.
    #[allow(dead_code)]
    pub fn focused_pane_id(&self) -> Option<PaneId> {
        self.active_tab().map(|t| t.focused_pane)
    }

    /// Returns a mutable reference to the focused pane in the active tab.
    pub fn focused_pane_mut(&mut self) -> Option<&mut Pane> {
        self.active_tab_mut().and_then(|t| t.focused_pane_mut())
    }

    /// Computes pane rectangles for the active tab.
    pub fn pane_rects(
        &self,
        buffer_width: usize,
        buffer_height: usize,
    ) -> (Vec<(PaneId, Rect)>, Vec<layout::Divider>) {
        // Reserve space for tab bar when there are multiple tabs
        let tab_bar_height = self.tab_bar_height();
        let pane_height = buffer_height.saturating_sub(tab_bar_height);

        if let Some(tab) = self.active_tab() {
            let (mut rects, dividers) = tab.pane_rects(buffer_width, pane_height);
            // Offset all rects by the tab bar height
            for (_, rect) in &mut rects {
                rect.y += tab_bar_height;
            }
            (rects, dividers)
        } else {
            (Vec::new(), Vec::new())
        }
    }

    /// Returns the height of the tab bar in pixels.
    pub fn tab_bar_height(&self) -> usize {
        if self.tabs.len() > 1 {
            let scale = self.config.font.scale.clamp(1, 8);
            8 * scale + 4 // cell height + padding
        } else {
            0
        }
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

    /// Creates a new tab and returns its ID.
    pub fn new_tab(&mut self) -> TabId {
        let id = self.next_tab_id;
        self.next_tab_id += 1;

        let mut tab = Tab::new(id);
        tab.spawn_initial_pane(
            self.config.font.scale,
            self.config.terminal.scrollback_lines as usize,
            self.event_proxy.as_ref(),
        );

        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
        self.layout_dirty = true;
        id
    }

    /// Closes the tab at the given index.
    pub fn close_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }

        self.tabs.remove(index);

        if self.tabs.is_empty() {
            self.should_exit = true;
            return;
        }

        // Adjust active tab index
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }

        self.layout_dirty = true;
    }

    /// Closes the active tab.
    pub fn close_active_tab(&mut self) {
        self.close_tab(self.active_tab);
    }

    /// Switches to the next tab.
    pub fn next_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        self.active_tab = (self.active_tab + 1) % self.tabs.len();
        self.layout_dirty = true;
        self.request_redraw();
    }

    /// Switches to the previous tab.
    pub fn prev_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        self.active_tab = if self.active_tab == 0 {
            self.tabs.len() - 1
        } else {
            self.active_tab - 1
        };
        self.layout_dirty = true;
        self.request_redraw();
    }

    /// Switches to the tab at the given index (0-indexed).
    pub fn goto_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_tab = index;
            self.layout_dirty = true;
            self.request_redraw();
        }
    }

    /// Initializes the first tab if none exist.
    fn initialize_first_tab(&mut self) {
        if !self.tabs.is_empty() {
            return;
        }
        self.new_tab();
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

        // Tab and pane actions are always handled, even while searching.
        if let Some(action) = non_search_action {
            if matches!(
                action,
                Action::NewTab
                    | Action::CloseTab
                    | Action::NextTab
                    | Action::PrevTab
                    | Action::Tab1
                    | Action::Tab2
                    | Action::Tab3
                    | Action::Tab4
                    | Action::Tab5
                    | Action::Tab6
                    | Action::Tab7
                    | Action::Tab8
                    | Action::Tab9
                    | Action::SplitVertical
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
            let Some(tab) = self.active_tab_mut() else {
                return;
            };
            let Some(pane) = tab.focused_pane_mut() else {
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

        // Check if click is in tab bar
        let tab_bar_height = self.tab_bar_height();
        let cursor_pos = self.input.cursor_position;
        if tab_bar_height > 0 && (cursor_pos.y as usize) < tab_bar_height {
            if state == ElementState::Pressed {
                self.handle_tab_bar_click();
            }
            return;
        }

        let (rects, _divs) = self.pane_rects(buffer_width as usize, buffer_height as usize);
        let Some((pane_id, pane_rect)) = self.pane_at_position(&rects, cursor_pos) else {
            return;
        };

        if let Some(tab) = self.active_tab_mut() {
            tab.set_focus(pane_id);
        }

        // Extract what we need before borrowing tab mutably
        let local = Self::localize_pos(pane_rect, cursor_pos);

        match state {
            ElementState::Pressed => {
                // Get pane scale and check for URL
                let (_scale, url_to_open, cell_coords) = {
                    let Some(tab) = self.active_tab_mut() else {
                        return;
                    };
                    let Some(pane) = tab.panes.get_mut(&pane_id) else {
                        return;
                    };

                    let (cell_w, cell_h) = Self::cell_size_for_scale(pane.scale);
                    let coords = pane.view.window_to_cell(local.x, local.y, cell_w, cell_h);
                    let url = coords.and_then(|(row, col)| pane.view.url_at(row, col));
                    (pane.scale, url, coords)
                };

                if let Some(url) = url_to_open {
                    if let Err(e) = self.url_opener.open(&url) {
                        eprintln!("Failed to open URL: {e}");
                    }
                    return;
                }

                if let Some((row, col)) = cell_coords {
                    self.input.mouse_selecting = true;
                    if let Some(tab) = self.active_tab_mut() {
                        if let Some(pane) = tab.panes.get_mut(&pane_id) {
                            pane.view.start_selection(row, col);
                        }
                    }
                    self.request_redraw();
                }
            }
            ElementState::Released => {
                self.input.mouse_selecting = false;
            }
        }
    }

    /// Handles clicking on the tab bar.
    fn handle_tab_bar_click(&mut self) {
        let scale = self.config.font.scale.clamp(1, 8);
        let cell_w = 8 * scale;
        let tab_padding = 8;
        let tab_width = cell_w * 8 + tab_padding * 2; // ~8 chars per tab + padding

        let click_x = self.input.cursor_position.x as usize;
        let tab_index = click_x / tab_width;

        if tab_index < self.tabs.len() {
            self.goto_tab(tab_index);
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

        let local = Self::localize_pos(pane_rect, position);
        let mouse_selecting = self.input.mouse_selecting;

        let Some(tab) = self.active_tab_mut() else {
            return;
        };

        let focused_pane = tab.focused_pane;
        let Some(pane) = tab.panes.get_mut(&pane_id) else {
            return;
        };

        let (cell_w, cell_h) = Self::cell_size_for_scale(pane.scale);

        if mouse_selecting && pane_id == focused_pane {
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
                    .active_tab()
                    .and_then(|t| t.focused_pane())
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
        let cursor_pos = self.input.cursor_position;

        let Some(tab) = self.active_tab_mut() else {
            return;
        };

        let target = if buffer_width > 0 && buffer_height > 0 {
            let (rects, _divs) = tab.pane_rects(buffer_width as usize, buffer_height as usize);
            rects
                .iter()
                .find(|(_, r)| r.contains(cursor_pos.x, cursor_pos.y))
                .map(|(id, _)| *id)
                .unwrap_or(tab.focused_pane)
        } else {
            tab.focused_pane
        };

        if let Some(pane) = tab.panes.get_mut(&target) {
            pane.view.scrollback_scroll_by(lines);
            self.request_redraw();
        }
    }
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.initialize_first_tab();

        if self.graphics.is_none() {
            self.create_window(event_loop);
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::PtyOutput { tab, pane, bytes } => {
                // Find the tab and pane
                if let Some(t) = self.tabs.iter().find(|t| t.id == tab) {
                    if let Some(p) = t.panes.get(&pane) {
                        p.terminal.process(&bytes);
                    }
                }
                self.request_redraw();
            }
            AppEvent::PtyExited { tab, pane } => {
                // Find the tab index
                if let Some(tab_idx) = self.tabs.iter().position(|t| t.id == tab) {
                    let should_close_tab = {
                        let t = &mut self.tabs[tab_idx];
                        if t.panes.contains_key(&pane) {
                            t.close_pane(pane)
                        } else {
                            false
                        }
                    };

                    if should_close_tab {
                        self.close_tab(tab_idx);
                    }

                    self.layout_dirty = true;
                    self.request_redraw();
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
