//! Application state and event handling using winit's ApplicationHandler pattern.

pub mod actions;
pub mod graphics;
pub mod input;
pub mod layout;
pub mod pane;
pub mod tab;

use std::io::Read;
use std::thread;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::keyboard::{Key, NamedKey};
use winit::window::WindowId;

use yatmux::clipboard::{ClipboardProvider, SystemClipboard};
use yatmux::config::{Action, Config, ShadowPromptMode};
use yatmux::constants::{CELL_H, READ_BUFFER_SIZE};
use yatmux::keys::key_to_pty_bytes;
use yatmux::renderer::Renderer;

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

    last_window_title: Option<String>,

    /// Toast message state (message, show time)
    toast: Option<(String, Instant)>,

    /// Context menu state (items, position, selected index)
    context_menu: Option<ContextMenu>,
}

/// Context menu state.
#[derive(Clone)]
pub struct ContextMenu {
    /// Menu items (label, action identifier)
    pub items: Vec<(&'static str, ContextMenuAction)>,
    /// Screen position where menu was opened
    pub x: usize,
    pub y: usize,
    /// Currently hovered item index
    pub hovered: Option<usize>,
}

/// Actions that can be triggered from the context menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextMenuAction {
    Copy,
    Paste,
    SelectAll,
    Search,
    OpenUrl,
    ClearScrollback,
    Reset,
    ScrollToTop,
    ScrollToBottom,
    CopyLastOutput,
    JumpToPrevPrompt,
    JumpToNextPrompt,
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
            last_window_title: None,
            toast: None,
            context_menu: None,
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

    /// Shows a toast message for a short duration.
    pub fn show_toast(&mut self, message: impl Into<String>) {
        self.toast = Some((message.into(), Instant::now()));
        self.request_redraw();
    }

    /// Returns the current toast message if it should still be visible.
    /// Toast duration is 1.5 seconds.
    pub fn current_toast(&self) -> Option<&str> {
        let duration_ms = self.config.ui.toast.duration_ms as u128;
        self.toast.as_ref().and_then(|(msg, time)| {
            if time.elapsed().as_millis() < duration_ms {
                Some(msg.as_str())
            } else {
                None
            }
        })
    }

    fn reload_config(&mut self) {
        self.config = Config::load();

        // Update palette immediately for ANSI colors/themes.
        if let Some(graphics) = &mut self.graphics {
            graphics.palette = std::sync::Arc::new(yatmux::renderer::create_palette_with_ansi(
                self.config.colors.palette,
            ));
        }

        // Force window title re-sync with the new config.
        self.last_window_title = None;
        self.sync_window_title();

        // Layout and rendering depend on config (padding, UI, etc.).
        self.layout_dirty = true;
        self.show_toast("Config reloaded");
        self.request_redraw();
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
            self.config
                .shell_integration
                .shadow_prompt_enabled_by_default,
        );

        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
        self.layout_dirty = true;
        self.refresh_active_tab_title_from_focused_pane();
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
        self.refresh_active_tab_title_from_focused_pane();
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
        self.refresh_active_tab_title_from_focused_pane();
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
        self.refresh_active_tab_title_from_focused_pane();
        self.request_redraw();
    }

    /// Switches to the tab at the given index (0-indexed).
    pub fn goto_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_tab = index;
            self.layout_dirty = true;
            self.refresh_active_tab_title_from_focused_pane();
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
                    | Action::CopyLastOutput
                    | Action::JumpToPrevPrompt
                    | Action::JumpToNextPrompt
                    | Action::ToggleShadowPrompt
                    | Action::ReloadConfig
            ) {
                self.execute_action(action);
                return;
            }
        }

        let mut needs_redraw = false;
        let mut action_to_execute: Option<Action> = None;
        let shadow_mode = self.config.shell_integration.shadow_prompt;

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
                // Check if this is Enter key - only snap to bottom on Enter
                let is_enter = matches!(event.logical_key, Key::Named(NamedKey::Enter));

                // Check if we should use shadow prompt (command is running - use cached state)
                // Never use shadow prompt in alt-screen apps (htop, vim, less).
                let is_command_running = shadow_mode != ShadowPromptMode::Off
                    && pane.shadow_prompt_enabled
                    && pane.command_running
                    && !pane.terminal.is_alt_screen_active();

                if is_command_running {
                    // Route input to shadow prompt instead of terminal.
                    // If a key isn't handled by the shadow prompt (eg. Ctrl+C), forward it to PTY.
                    let handled =
                        Self::handle_shadow_prompt_input(pane, &event.logical_key, modifiers);
                    needs_redraw |= handled;

                    if !handled {
                        if let Some(bytes) = key_to_pty_bytes(&event.logical_key, modifiers) {
                            pane.terminal.write(&bytes);
                            needs_redraw = true;
                        }
                    }
                } else {
                    // Regular terminal input
                    if !ctrl && !alt {
                        if let Some(text) = &event.text {
                            if !text.is_empty() {
                                if is_enter {
                                    pane.view.scrollback_snap_to_bottom();
                                    // Mark command as running when Enter is pressed
                                    pane.command_running = true;
                                }
                                pane.terminal.write(text.as_bytes());
                                needs_redraw = true;
                            }
                        }
                    }

                    if !needs_redraw {
                        if let Some(bytes) = key_to_pty_bytes(&event.logical_key, modifiers) {
                            if is_enter {
                                pane.view.scrollback_snap_to_bottom();
                                // Mark command as running when Enter is pressed
                                pane.command_running = true;
                            }
                            pane.terminal.write(&bytes);
                            needs_redraw = true;
                        }
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

    /// Handles input for the shadow prompt during command execution.
    /// Returns true if input was handled and a redraw is needed.
    fn handle_shadow_prompt_input(
        pane: &mut Pane,
        key: &Key,
        modifiers: winit::keyboard::ModifiersState,
    ) -> bool {
        let ctrl = modifiers.control_key();
        let alt = modifiers.alt_key();

        match key {
            Key::Named(NamedKey::Backspace) => {
                pane.shadow_prompt.backspace();
                true
            }
            Key::Named(NamedKey::Delete) => {
                pane.shadow_prompt.delete();
                true
            }
            Key::Named(NamedKey::ArrowLeft) => {
                pane.shadow_prompt.move_left();
                true
            }
            Key::Named(NamedKey::ArrowRight) => {
                pane.shadow_prompt.move_right();
                true
            }
            Key::Named(NamedKey::Home) => {
                pane.shadow_prompt.move_home();
                true
            }
            Key::Named(NamedKey::End) => {
                pane.shadow_prompt.move_end();
                true
            }
            Key::Named(NamedKey::Escape) => {
                // Clear shadow prompt on Escape
                pane.shadow_prompt.clear();
                true
            }
            Key::Named(NamedKey::Enter) => {
                // Add newline to buffer (for multi-line commands)
                pane.shadow_prompt.insert('\n');
                true
            }
            Key::Named(NamedKey::Space) => {
                if !ctrl && !alt {
                    pane.shadow_prompt.insert(' ');
                    true
                } else {
                    false
                }
            }
            Key::Character(s) => {
                // Regular text input (when not ctrl/alt modified)
                if !ctrl && !alt {
                    pane.shadow_prompt.insert_str(s);
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Handles mouse button events.
    fn handle_mouse_button(&mut self, state: ElementState, button: MouseButton) {
        match button {
            MouseButton::Left => self.handle_left_click(state),
            MouseButton::Right => self.handle_right_click(state),
            MouseButton::Middle => self.handle_middle_click(state),
            _ => {}
        }
    }

    /// Handles left mouse button events.
    fn handle_left_click(&mut self, state: ElementState) {
        // Close context menu on any left click
        if self.context_menu.is_some() {
            if state == ElementState::Pressed {
                // Check if clicking on a menu item
                if let Some(action) = self.context_menu_item_at_cursor() {
                    self.execute_context_menu_action(action);
                }
                self.context_menu = None;
                self.request_redraw();
            }
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
        self.refresh_active_tab_title_from_focused_pane();

        // Extract what we need before borrowing tab mutably
        let local = Self::localize_pos(pane_rect, cursor_pos);

        // Check if terminal wants mouse events
        let (is_mouse_grabbed, cell_coords, _scale) = {
            let Some(tab) = self.active_tab() else {
                return;
            };
            let Some(pane) = tab.panes.get(&pane_id) else {
                return;
            };
            let (cell_w, cell_h) = Self::cell_size_for_scale(pane.scale);
            let coords = pane.view.window_to_cell(local.x, local.y, cell_w, cell_h);
            (pane.terminal.is_mouse_grabbed(), coords, pane.scale)
        };

        // If terminal application wants mouse events, forward them
        if is_mouse_grabbed {
            if let Some((row, col)) = cell_coords {
                use yatmux::terminal::{
                    KeyModifiers, MouseButton as TermMouseButton, MouseEventKind,
                };
                let kind = match state {
                    ElementState::Pressed => MouseEventKind::Press,
                    ElementState::Released => MouseEventKind::Release,
                };
                let modifiers = KeyModifiers::NONE; // TODO: track modifier keys
                if let Some(tab) = self.active_tab() {
                    if let Some(pane) = tab.panes.get(&pane_id) {
                        pane.terminal
                            .mouse_event(col, row, TermMouseButton::Left, kind, modifiers);
                    }
                }
            }
            return;
        }

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
                    // Click-to-position cursor in shell input when semantic zones are available.
                    if self.try_click_move_shell_cursor(pane_id, row, col) {
                        self.request_redraw();
                        return;
                    }

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

    /// Handles right mouse button events (context menu).
    fn handle_right_click(&mut self, state: ElementState) {
        if state != ElementState::Pressed {
            return;
        }

        // Close existing menu if clicking elsewhere
        if self.context_menu.is_some() {
            self.context_menu = None;
            self.request_redraw();
            return;
        }

        let cursor_pos = self.input.cursor_position;
        let x = cursor_pos.x as usize;
        let y = cursor_pos.y as usize;

        // Build context menu items based on current state
        let mut items: Vec<(&'static str, ContextMenuAction)> = Vec::new();

        let focused_pane = self.active_tab().and_then(|t| t.focused_pane());

        let has_selection = focused_pane
            .map(|p| p.view.has_selection())
            .unwrap_or(false);
        let has_last_output = focused_pane
            .and_then(|pane| pane.terminal.last_command_output())
            .is_some();
        let has_prompts = focused_pane
            .map(|pane| !pane.terminal.prompt_positions().is_empty())
            .unwrap_or(false);

        // Check if there's a URL under cursor
        let has_url = self.url_at_cursor().is_some();

        if has_selection {
            items.push(("Copy", ContextMenuAction::Copy));
        }
        items.push(("Paste", ContextMenuAction::Paste));
        items.push(("Select All", ContextMenuAction::SelectAll));
        items.push(("Search", ContextMenuAction::Search));
        if has_url {
            items.push(("Open URL", ContextMenuAction::OpenUrl));
        }

        items.push(("Scroll to Top", ContextMenuAction::ScrollToTop));
        items.push(("Scroll to Bottom", ContextMenuAction::ScrollToBottom));
        items.push(("Clear Scrollback", ContextMenuAction::ClearScrollback));
        items.push(("Reset Terminal", ContextMenuAction::Reset));
        if has_last_output {
            items.push(("Copy Last Output", ContextMenuAction::CopyLastOutput));
        }
        if has_prompts {
            items.push((
                "Jump to Previous Prompt",
                ContextMenuAction::JumpToPrevPrompt,
            ));
            items.push(("Jump to Next Prompt", ContextMenuAction::JumpToNextPrompt));
        }

        self.context_menu = Some(ContextMenu {
            items,
            x,
            y,
            hovered: Some(0),
        });
        self.request_redraw();
    }

    /// Handles middle mouse button events (paste).
    fn handle_middle_click(&mut self, state: ElementState) {
        if state != ElementState::Pressed {
            return;
        }

        // Close context menu if open
        if self.context_menu.is_some() {
            self.context_menu = None;
            self.request_redraw();
            return;
        }

        // Paste from clipboard
        self.handle_paste();
    }

    /// Returns the URL at the current cursor position, if any.
    fn url_at_cursor(&self) -> Option<String> {
        let (buffer_width, buffer_height) = self.last_buffer_size;
        if buffer_width == 0 || buffer_height == 0 {
            return None;
        }

        let cursor_pos = self.input.cursor_position;
        let (rects, _) = self.pane_rects(buffer_width as usize, buffer_height as usize);
        let (pane_id, pane_rect) = self.pane_at_position(&rects, cursor_pos)?;

        let tab = self.active_tab()?;
        let pane = tab.panes.get(&pane_id)?;

        let local = Self::localize_pos(pane_rect, cursor_pos);
        let (cell_w, cell_h) = Self::cell_size_for_scale(pane.scale);
        let (row, col) = pane.view.window_to_cell(local.x, local.y, cell_w, cell_h)?;

        pane.view.url_at(row, col)
    }

    /// Returns the context menu action at the current cursor position.
    fn context_menu_item_at_cursor(&self) -> Option<ContextMenuAction> {
        let menu = self.context_menu.as_ref()?;
        let cursor_pos = self.input.cursor_position;

        let scale = self.config.font.scale.clamp(1, 8);
        let item_height = 8 * scale + 8; // cell height + padding
        let menu_width = 12 * 8 * scale; // ~12 chars width

        let x = cursor_pos.x as usize;
        let y = cursor_pos.y as usize;

        // Check if cursor is within menu bounds
        if x < menu.x || x >= menu.x + menu_width {
            return None;
        }

        if y < menu.y {
            return None;
        }

        let relative_y = y - menu.y;
        let item_index = relative_y / item_height;

        menu.items.get(item_index).map(|(_, action)| *action)
    }

    /// Executes a context menu action.
    fn execute_context_menu_action(&mut self, action: ContextMenuAction) {
        match action {
            ContextMenuAction::Copy => {
                self.handle_copy();
            }
            ContextMenuAction::Paste => {
                self.handle_paste();
            }
            ContextMenuAction::SelectAll => {
                if let Some(tab) = self.active_tab_mut() {
                    if let Some(pane) = tab.focused_pane_mut() {
                        pane.view.select_all();
                    }
                }
                self.request_redraw();
            }
            ContextMenuAction::Search => {
                self.execute_action(Action::SearchFind);
            }
            ContextMenuAction::OpenUrl => {
                if let Some(url) = self.url_at_cursor() {
                    if let Err(e) = self.url_opener.open(&url) {
                        eprintln!("Failed to open URL: {e}");
                    }
                }
            }
            ContextMenuAction::ScrollToTop => {
                self.execute_action(Action::ScrollToTop);
            }
            ContextMenuAction::ScrollToBottom => {
                self.execute_action(Action::ScrollToBottom);
            }
            ContextMenuAction::ClearScrollback => {
                self.execute_action(Action::ClearScrollback);
            }
            ContextMenuAction::Reset => {
                self.execute_action(Action::Reset);
            }
            ContextMenuAction::CopyLastOutput => {
                self.execute_action(Action::CopyLastOutput);
            }
            ContextMenuAction::JumpToPrevPrompt => {
                self.execute_action(Action::JumpToPrevPrompt);
            }
            ContextMenuAction::JumpToNextPrompt => {
                self.execute_action(Action::JumpToNextPrompt);
            }
        }
    }

    /// Returns the current context menu if any.
    pub fn context_menu(&self) -> Option<&ContextMenu> {
        self.context_menu.as_ref()
    }

    /// Attempts to reposition the cursor within the current shell input.
    ///
    /// This relies on OSC 133 semantic zones (Prompt/Input) and works best for
    /// single-line inputs where the click is on the cursor line.
    fn try_click_move_shell_cursor(
        &mut self,
        pane_id: PaneId,
        click_row: usize,
        click_col: usize,
    ) -> bool {
        if !self.config.shell_integration.semantic_zones_from_osc133 {
            return false;
        }

        let Some(tab) = self.active_tab() else {
            return false;
        };
        let Some(pane) = tab.panes.get(&pane_id) else {
            return false;
        };

        // Don't fight running apps/commands.
        if pane.command_running {
            return false;
        }

        let ((cursor_row, cursor_col), _cursor_visible) = pane.terminal.cursor();
        let cursor_row = cursor_row as usize;
        let cursor_col = cursor_col as usize;

        // For now, only support click-to-move on the cursor row.
        if click_row != cursor_row {
            return false;
        }

        let visible_start = pane.terminal.visible_start_row();
        let cursor_phys_y = visible_start + cursor_row;
        let click_phys_y = visible_start + click_row;

        let zones = match pane.terminal.semantic_zones() {
            Ok(z) => z,
            Err(_) => return false,
        };

        let input_zone = zones.iter().rev().find(|z| {
            z.semantic_type == tattoy_wezterm_term::SemanticType::Input
                && cursor_phys_y >= z.start_y as usize
                && cursor_phys_y <= z.end_y as usize
        });
        let Some(zone) = input_zone else {
            return false;
        };

        let in_zone = |phys_y: usize, col: usize| -> bool {
            let start_y = zone.start_y as usize;
            let end_y = zone.end_y as usize;
            if phys_y < start_y || phys_y > end_y {
                return false;
            }
            if start_y == end_y {
                return col >= zone.start_x && col < zone.end_x;
            }
            if phys_y == start_y {
                return col >= zone.start_x;
            }
            if phys_y == end_y {
                return col < zone.end_x;
            }
            true
        };

        if !in_zone(cursor_phys_y, cursor_col) {
            return false;
        }
        if !in_zone(click_phys_y, click_col) {
            return false;
        }

        let delta = click_col as isize - cursor_col as isize;
        if delta == 0 {
            return true;
        }

        let steps = delta
            .unsigned_abs()
            .min(self.config.interaction.click_move_max_steps);
        let seq: &[u8] = if delta > 0 { b"\x1b[C" } else { b"\x1b[D" };

        let mut bytes = Vec::with_capacity(steps * seq.len());
        for _ in 0..steps {
            bytes.extend_from_slice(seq);
        }
        pane.terminal.write(&bytes);
        true
    }

    /// Handles mouse movement.
    fn handle_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        self.input.cursor_position = position;

        let (buffer_width, buffer_height) = self.last_buffer_size;
        if buffer_width == 0 || buffer_height == 0 {
            self.update_cursor();
            return;
        }

        // Update context menu hover state if menu is open
        if let Some(ref mut menu) = self.context_menu {
            let scale = self.config.font.scale.clamp(1, 8);
            let item_height = 8 * scale + 8;
            let y = position.y as usize;
            if y >= menu.y {
                let relative_y = y - menu.y;
                let item_index = relative_y / item_height;
                if item_index < menu.items.len() {
                    menu.hovered = Some(item_index);
                } else {
                    menu.hovered = None;
                }
            } else {
                menu.hovered = None;
            }
            self.request_redraw();
            return;
        }

        let (rects, _divs) = self.pane_rects(buffer_width as usize, buffer_height as usize);
        let Some((pane_id, pane_rect)) = self.pane_at_position(&rects, position) else {
            self.update_cursor();
            return;
        };

        let local = Self::localize_pos(pane_rect, position);
        let mouse_selecting = self.input.mouse_selecting;

        // Check if terminal wants mouse events
        let is_mouse_grabbed = self
            .active_tab()
            .and_then(|t| t.panes.get(&pane_id))
            .map(|p| p.terminal.is_mouse_grabbed())
            .unwrap_or(false);

        if is_mouse_grabbed {
            let Some(tab) = self.active_tab() else {
                return;
            };
            let Some(pane) = tab.panes.get(&pane_id) else {
                return;
            };
            let (cell_w, cell_h) = Self::cell_size_for_scale(pane.scale);
            if let Some((row, col)) = pane.view.window_to_cell(local.x, local.y, cell_w, cell_h) {
                use yatmux::terminal::{
                    KeyModifiers, MouseButton as TermMouseButton, MouseEventKind,
                };
                let modifiers = KeyModifiers::NONE;
                pane.terminal.mouse_event(
                    col,
                    row,
                    TermMouseButton::None,
                    MouseEventKind::Move,
                    modifiers,
                );
            }
            return;
        }

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
            // Check if terminal wants mouse events (for scroll in apps like less, vim)
            if pane.terminal.is_mouse_grabbed() {
                use yatmux::terminal::{
                    KeyModifiers, MouseButton as TermMouseButton, MouseEventKind,
                };
                let button = if lines > 0 {
                    TermMouseButton::WheelUp(lines as usize)
                } else {
                    TermMouseButton::WheelDown((-lines) as usize)
                };
                let modifiers = KeyModifiers::NONE;
                // Use current cursor position in cell coordinates
                let (cell_w, cell_h) = Self::cell_size_for_scale(pane.scale);
                let (row, col) = pane
                    .view
                    .window_to_cell(cursor_pos.x, cursor_pos.y, cell_w, cell_h)
                    .unwrap_or((0, 0));
                pane.terminal
                    .mouse_event(col, row, button, MouseEventKind::Press, modifiers);
            } else {
                pane.view.scrollback_scroll_by(lines);
            }
            self.request_redraw();
        }
    }

    fn apply_shell_integration_updates(&mut self, tab_idx: usize, pane: PaneId) {
        let cfg = &self.config.shell_integration;
        let tab_id = self.tabs[tab_idx].id;
        let focused_pane = self.tabs[tab_idx].focused_pane;
        let is_active_tab = tab_idx == self.active_tab;

        let mut new_title: Option<String> = None;

        {
            let Some(pane_state) = self.tabs[tab_idx].panes.get_mut(&pane) else {
                return;
            };

            if cfg.cwd_from_osc7 {
                pane_state.shell_cwd = pane_state.terminal.shell_cwd();
            }

            // Only fetch semantic zones when debug logging is enabled (expensive operation)
            if cfg.semantic_zones_from_osc133 && cfg.debug_log {
                if let Ok(zones) = pane_state.terminal.semantic_zones() {
                    if !zones.is_empty() {
                        eprintln!("[shell] tab={} pane={} zones:", tab_id, pane);
                        for zone in &zones {
                            eprintln!(
                                "  {:?} rows {}..{}",
                                zone.semantic_type, zone.start_y, zone.end_y
                            );
                        }
                    }
                }
            }

            let status = pane_state.terminal.shell_integration_status();
            if cfg.debug_log && status != pane_state.shell_integration {
                eprintln!(
                    "[shell] tab={} pane={} any={} osc7={} osc133={} title={}",
                    tab_id,
                    pane,
                    status.any(),
                    status.osc7_cwd,
                    status.osc133_semantic,
                    status.osc_title
                );
            }
            pane_state.shell_integration = status;

            // Note: command_running state is now updated in PtyOutput handler
            // by detecting prompt markers in raw bytes (much cheaper than get_semantic_zones)

            if cfg.title_from_osc {
                pane_state.shell_title = pane_state.terminal.shell_title();
            }

            // Only update the tab title based on the focused pane.
            if focused_pane == pane {
                new_title = match cfg.tab_title_source {
                    yatmux::config::TabTitleSource::None => None,
                    yatmux::config::TabTitleSource::Cwd => pane_state
                        .shell_cwd
                        .as_deref()
                        .map(Self::cwd_url_to_tab_title)
                        .or_else(|| pane_state.shell_title.clone()),
                    yatmux::config::TabTitleSource::Title => pane_state.shell_title.clone(),
                };
            }
        }

        if let Some(title) = new_title {
            let title = Self::sanitize_title(&title);
            if !title.is_empty() {
                self.tabs[tab_idx].title = title;
                if is_active_tab {
                    self.sync_window_title();
                }
            }
        }
    }

    fn refresh_active_tab_title_from_focused_pane(&mut self) {
        let tab_idx = self.active_tab;
        let cfg = &self.config.shell_integration;

        let Some(tab) = self.tabs.get_mut(tab_idx) else {
            return;
        };
        let focused = tab.focused_pane;
        let Some(pane) = tab.panes.get(&focused) else {
            self.sync_window_title();
            return;
        };

        let new_title = match cfg.tab_title_source {
            yatmux::config::TabTitleSource::None => None,
            yatmux::config::TabTitleSource::Cwd => pane
                .shell_cwd
                .as_deref()
                .map(Self::cwd_url_to_tab_title)
                .or_else(|| pane.shell_title.clone()),
            yatmux::config::TabTitleSource::Title => pane.shell_title.clone(),
        };

        if let Some(title) = new_title {
            let title = Self::sanitize_title(&title);
            if !title.is_empty() {
                tab.title = title;
            }
        }

        self.sync_window_title();
    }

    fn sanitize_title(s: &str) -> String {
        // Remove newlines/control chars; keep it single-line and readable.
        s.chars()
            .filter(|&ch| ch != '\n' && ch != '\r' && !ch.is_control())
            .collect::<String>()
            .trim()
            .to_string()
    }

    fn cwd_url_to_tab_title(cwd_url: &str) -> String {
        // Typical OSC 7 payload is a file:// URL.
        // Prefer showing a friendly basename in the tab bar.
        let mut s = cwd_url.trim();
        if let Some(stripped) = s.strip_prefix("file://") {
            s = stripped;
        }

        // Drop query/fragment if present.
        s = s.split(['?', '#']).next().unwrap_or(s);

        // Normalize multiple slashes (e.g. file:///home -> ///home -> /home).
        while s.starts_with("//") {
            s = &s[1..];
        }

        let trimmed = s.trim_end_matches('/');
        let base = trimmed.rsplit('/').next().unwrap_or(trimmed);
        if base.is_empty() {
            "/".to_string()
        } else {
            base.to_string()
        }
    }

    fn sync_window_title(&mut self) {
        if !self
            .config
            .shell_integration
            .window_title_follows_active_tab
        {
            return;
        }
        let Some(graphics) = &self.graphics else {
            return;
        };

        let base = self.config.window.title.trim();
        let tab_title = self
            .tabs
            .get(self.active_tab)
            .map(|t| t.title.trim())
            .filter(|t| !t.is_empty())
            .unwrap_or("yatmux");

        let new_title = if base.is_empty() {
            tab_title.to_string()
        } else {
            format!("{tab_title} — {base}")
        };

        if self.last_window_title.as_deref() == Some(new_title.as_str()) {
            return;
        }

        graphics.surface.window().set_title(&new_title);
        self.last_window_title = Some(new_title);
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
                let Some(tab_idx) = self.tabs.iter().position(|t| t.id == tab) else {
                    return;
                };

                // Check for prompt marker in raw bytes BEFORE processing
                // OSC 133;A marks prompt start - means command finished
                let has_prompt_marker = bytes.windows(6).any(|w| w == b"]133;A" || w == b"]133;B");

                {
                    let t = &self.tabs[tab_idx];
                    if let Some(p) = t.panes.get(&pane) {
                        p.terminal.process(&bytes);
                    }
                }

                // Update prompt state
                if let Some(pane_state) = self.tabs[tab_idx].panes.get_mut(&pane) {
                    // If we detected a prompt marker, flush shadow prompt
                    if has_prompt_marker {
                        pane_state.command_running = false;
                        let buffered = pane_state.shadow_prompt.take();
                        if !buffered.is_empty() {
                            pane_state.terminal.write(buffered.as_bytes());
                        }
                    }
                }

                self.apply_shell_integration_updates(tab_idx, pane);
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
