//! Application state and event handling using winit's ApplicationHandler pattern.

pub mod actions;
pub mod graphics;
pub mod input;
pub mod layout;
pub mod pane;
pub mod tab;

mod context_menu;
mod help_filter;
mod keyboard;
mod mouse;
mod plugins;
mod pty;
mod prompt;
mod shell_integration;
mod tabs;
mod url;
mod winit_handler;

pub use context_menu::{ContextMenu, ContextMenuAction};
pub use help_filter::HelpFilterState;
pub use plugins::{PluginCommand, PluginManager};
pub use pty::spawn_pty_reader;
pub use prompt::{PromptKind, PromptState};

use std::time::Instant;
use std::collections::HashMap;

use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, MouseScrollDelta};
use winit::event_loop::EventLoopProxy;
use winit::keyboard::{Key, NamedKey};

use yatmux::clipboard::{ClipboardProvider, SystemClipboard};
use yatmux::config::{Action, Config, ShadowPromptMode};
use yatmux::keys::key_to_pty_bytes;
use yatmux::renderer::Renderer;

use graphics::GraphicsState;
use input::{InputState, apply_search_input, key_event_to_string};
use layout::{PaneId, Rect};
use pane::Pane;
use tab::{Tab, TabId};
use url::{SystemUrlOpener, UrlOpener};

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
    PluginCommands { plugin: String, commands: Vec<PluginCommand> },
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
    pub help_filter: HelpFilterState,
    pub prompt: Option<PromptState>,
    pub prompt_owners: HashMap<String, String>,
    pub state_owners: HashMap<String, String>,
    pub clipboard_owners: HashMap<String, String>,
    pub plugins: PluginManager,
    plugin_dispatch_depth: usize,
    plugins_started: bool,
    pub shell_warning_dismissed: bool,
    pub should_exit: bool,
    last_window_title: Option<String>,

    /// Toast message state (message, show time)
    toast: Option<(String, Instant)>,

    /// Context menu state (items, position, selected index)
    context_menu: Option<ContextMenu>,
}

impl App {
    /// Creates a new application with the given configuration.
    pub fn new(config: Config) -> Self {
        let plugins = PluginManager::new(&config);
        let app = App {
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
            help_filter: HelpFilterState::new(),
            prompt: None,
            prompt_owners: HashMap::new(),
            state_owners: HashMap::new(),
            clipboard_owners: HashMap::new(),
            plugins,
            plugin_dispatch_depth: 0,
            plugins_started: false,
            shell_warning_dismissed: false,
            should_exit: false,
            last_window_title: None,
            toast: None,
            context_menu: None,
        };
        app.sync_font_scale_clamp();
        app
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
        self.sync_font_scale_clamp();
        self.plugins.reload(&self.config);

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
        self.dispatch_plugin_event(plugins::PluginEvent {
            event: "config_reload".to_string(),
            action: None,
            source: None,
            tab_id: self.active_tab().map(|t| t.id),
            pane_id: self.active_tab().map(|t| t.focused_pane),
            data: None,
        });
    }

    fn sync_font_scale_clamp(&self) {
        let (scale_min, scale_max) = self.config.font_scale_clamp();
        self.renderer
            .font_renderer
            .set_scale_clamp(scale_min, scale_max);
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
        let mut pane_font_config = self.config.font.clone();
        pane_font_config.scale = pane.scale;
        let (cell_w, cell_h) = self.renderer.font_renderer.cell_size(&pane_font_config);
        let (row, col) = pane.view.window_to_cell(local.x, local.y, cell_w, cell_h)?;

        pane.view.url_at(row, col)
    }
}
