//! Application state and event handling using winit's ApplicationHandler pattern.

use std::collections::HashMap;
use std::io::Read;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::thread;

use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};
use winit::window::{CursorIcon, Window, WindowId};

use term::clipboard::{ClipboardProvider, SystemClipboard};
use term::config::{Action, Config};
use term::constants::{CELL_H, CELL_W, READ_BUFFER_SIZE};
use term::keys::key_to_pty_bytes;
use term::renderer::{HelpSection, Renderer, TerminalView, create_palette};
use term::terminal::Terminal;

type PaneId = u64;

#[derive(Clone, Copy, Debug)]
struct Rect {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

impl Rect {
    fn contains(&self, x: f64, y: f64) -> bool {
        let x = x as isize;
        let y = y as isize;
        if x < 0 || y < 0 {
            return false;
        }
        let x = x as usize;
        let y = y as usize;
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SplitDir {
    Vertical,
    Horizontal,
}

#[derive(Clone, Debug)]
enum LayoutNode {
    Leaf(PaneId),
    Split {
        dir: SplitDir,
        ratio: f32,
        a: Box<LayoutNode>,
        b: Box<LayoutNode>,
    },
}

#[derive(Clone, Copy, Debug)]
struct Divider {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

impl LayoutNode {
    fn leaf_rects(&self, rect: Rect, out: &mut Vec<(PaneId, Rect)>, dividers: &mut Vec<Divider>) {
        match self {
            LayoutNode::Leaf(id) => out.push((*id, rect)),
            LayoutNode::Split { dir, ratio, a, b } => {
                // Reserve 1px divider line.
                let divider_thickness = 1usize;

                match dir {
                    SplitDir::Vertical => {
                        if rect.w <= divider_thickness + 1 {
                            a.leaf_rects(rect, out, dividers);
                            return;
                        }
                        let min_w = 1usize;
                        let max_div = rect.w.saturating_sub(divider_thickness + min_w);
                        let mut div = ((rect.w as f32) * ratio).round() as isize;
                        div = div.clamp(min_w as isize, max_div as isize);
                        let div = div as usize;

                        let left = Rect {
                            x: rect.x,
                            y: rect.y,
                            w: div,
                            h: rect.h,
                        };
                        let right = Rect {
                            x: rect.x + div + divider_thickness,
                            y: rect.y,
                            w: rect.w.saturating_sub(div + divider_thickness),
                            h: rect.h,
                        };

                        dividers.push(Divider {
                            x: rect.x + div,
                            y: rect.y,
                            w: divider_thickness,
                            h: rect.h,
                        });

                        a.leaf_rects(left, out, dividers);
                        b.leaf_rects(right, out, dividers);
                    }
                    SplitDir::Horizontal => {
                        if rect.h <= divider_thickness + 1 {
                            a.leaf_rects(rect, out, dividers);
                            return;
                        }
                        let min_h = 1usize;
                        let max_div = rect.h.saturating_sub(divider_thickness + min_h);
                        let mut div = ((rect.h as f32) * ratio).round() as isize;
                        div = div.clamp(min_h as isize, max_div as isize);
                        let div = div as usize;

                        let top = Rect {
                            x: rect.x,
                            y: rect.y,
                            w: rect.w,
                            h: div,
                        };
                        let bottom = Rect {
                            x: rect.x,
                            y: rect.y + div + divider_thickness,
                            w: rect.w,
                            h: rect.h.saturating_sub(div + divider_thickness),
                        };

                        dividers.push(Divider {
                            x: rect.x,
                            y: rect.y + div,
                            w: rect.w,
                            h: divider_thickness,
                        });

                        a.leaf_rects(top, out, dividers);
                        b.leaf_rects(bottom, out, dividers);
                    }
                }
            }
        }
    }

    fn replace_leaf(&mut self, target: PaneId, replacement: LayoutNode) -> bool {
        match self {
            LayoutNode::Leaf(id) => {
                if *id == target {
                    *self = replacement;
                    true
                } else {
                    false
                }
            }
            LayoutNode::Split { a, b, .. } => {
                a.replace_leaf(target, replacement.clone()) || b.replace_leaf(target, replacement)
            }
        }
    }

    fn contains_pane(&self, target: PaneId) -> bool {
        match self {
            LayoutNode::Leaf(id) => *id == target,
            LayoutNode::Split { a, b, .. } => a.contains_pane(target) || b.contains_pane(target),
        }
    }

    fn first_leaf(&self) -> Option<PaneId> {
        match self {
            LayoutNode::Leaf(id) => Some(*id),
            LayoutNode::Split { a, b, .. } => a.first_leaf().or_else(|| b.first_leaf()),
        }
    }

    fn remove_pane(&mut self, target: PaneId) -> bool {
        fn without(node: LayoutNode, target: PaneId) -> Option<LayoutNode> {
            match node {
                LayoutNode::Leaf(id) => {
                    if id == target {
                        None
                    } else {
                        Some(LayoutNode::Leaf(id))
                    }
                }
                LayoutNode::Split { dir, ratio, a, b } => {
                    let a = without(*a, target);
                    let b = without(*b, target);

                    match (a, b) {
                        (None, None) => None,
                        (Some(only), None) | (None, Some(only)) => Some(only),
                        (Some(a), Some(b)) => Some(LayoutNode::Split {
                            dir,
                            ratio,
                            a: Box::new(a),
                            b: Box::new(b),
                        }),
                    }
                }
            }
        }

        let old = std::mem::replace(self, LayoutNode::Leaf(target));
        if let Some(new) = without(old, target) {
            *self = new;
            true
        } else {
            // Restore to a minimal leaf; caller should handle.
            *self = LayoutNode::Leaf(target);
            false
        }
    }

    fn adjust_ratio_for_pane(
        &mut self,
        target: PaneId,
        axis: SplitDir,
        delta: f32,
        done: &mut bool,
    ) -> bool {
        match self {
            LayoutNode::Leaf(id) => *id == target,
            LayoutNode::Split { dir, ratio, a, b } => {
                let in_a = a.adjust_ratio_for_pane(target, axis, delta, done);
                let in_b = if in_a {
                    false
                } else {
                    b.adjust_ratio_for_pane(target, axis, delta, done)
                };

                if (in_a || in_b) && !*done && *dir == axis {
                    // `ratio` controls size of `a`. If the target is in `b`, invert delta.
                    let signed_delta = if in_a { delta } else { -delta };
                    *ratio = (*ratio + signed_delta).clamp(0.1, 0.9);
                    *done = true;
                }

                in_a || in_b
            }
        }
    }
}

struct Pane {
    terminal: Terminal,
    view: TerminalView,
}

/// Custom events for the application.
#[derive(Debug)]
pub enum AppEvent {
    /// PTY has produced output bytes.
    PtyOutput { pane: PaneId, bytes: Vec<u8> },
    /// PTY has closed (shell exited).
    PtyExited { pane: PaneId },
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
    panes: HashMap<PaneId, Pane>,
    layout: LayoutNode,
    focused_pane: PaneId,
    focus_history: Vec<PaneId>,
    next_pane_id: PaneId,
    layout_dirty: bool,
    last_buffer_size: (u32, u32),

    renderer: Renderer,
    clipboard: Box<dyn ClipboardProvider>,
    url_opener: Box<dyn UrlOpener>,
    graphics: Option<GraphicsState>,
    input: InputState,
    event_proxy: Option<EventLoopProxy<AppEvent>>,
    show_help: bool,
    help_scroll: usize,
    help_max_scroll: usize,
    should_exit: bool,
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

    fn record_focus(&mut self, id: PaneId) {
        self.focus_history.retain(|&x| x != id);
        self.focus_history.push(id);
    }

    fn set_focus(&mut self, id: PaneId) {
        if self.focused_pane == id {
            self.record_focus(id);
            return;
        }
        self.focused_pane = id;
        self.record_focus(id);
    }

    fn focus_fallback(&mut self) {
        if let Some(&id) = self
            .focus_history
            .iter()
            .rev()
            .find(|&&id| self.panes.contains_key(&id))
        {
            self.focused_pane = id;
            self.record_focus(id);
            return;
        }

        if let Some(id) = self.layout.first_leaf() {
            self.focused_pane = id;
            self.record_focus(id);
        }
    }

    fn focused_pane_mut(&mut self) -> Option<&mut Pane> {
        self.panes.get_mut(&self.focused_pane)
    }

    fn pane_rects(
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

    fn pane_at_position(
        &self,
        rects: &[(PaneId, Rect)],
        pos: PhysicalPosition<f64>,
    ) -> Option<(PaneId, Rect)> {
        rects
            .iter()
            .find(|(_id, r)| r.contains(pos.x, pos.y))
            .copied()
    }

    fn initialize_first_pane(&mut self) {
        if !self.panes.is_empty() {
            return;
        }
        self.spawn_pane(1);
    }

    fn spawn_pane(&mut self, id: PaneId) {
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

        if let Some(proxy) = &self.event_proxy {
            spawn_pty_reader(reader, proxy.clone(), id);
        }

        self.panes.insert(
            id,
            Pane {
                terminal,
                view: TerminalView::new(),
            },
        );
    }

    fn split_focused(&mut self, dir: SplitDir) {
        let focused = self.focused_pane;
        if !self.layout.contains_pane(focused) {
            return;
        }

        let new_id = self.next_pane_id;
        self.next_pane_id += 1;
        self.spawn_pane(new_id);

        let replacement = LayoutNode::Split {
            dir,
            ratio: 0.5,
            a: Box::new(LayoutNode::Leaf(focused)),
            b: Box::new(LayoutNode::Leaf(new_id)),
        };

        if self.layout.replace_leaf(focused, replacement) {
            self.set_focus(new_id);
            self.layout_dirty = true;
            self.request_redraw();
        }
    }

    fn close_pane(&mut self, target: PaneId) {
        self.panes.remove(&target);
        self.focus_history.retain(|&x| x != target);

        if self.panes.is_empty() {
            self.should_exit = true;
            return;
        }

        let _ = self.layout.remove_pane(target);

        // Prefer the most recently focused still-alive pane.
        self.focus_fallback();

        self.layout_dirty = true;
        self.update_cursor();
        self.request_redraw();
    }

    fn close_focused_pane(&mut self) {
        let target = self.focused_pane;
        self.close_pane(target);
    }

    fn focus_move(&mut self, dir: SplitDir, positive: bool) {
        let (buffer_width, buffer_height) = self.last_buffer_size;
        if buffer_width == 0 || buffer_height == 0 {
            return;
        }

        let (rects, _divs) = self.pane_rects(buffer_width as usize, buffer_height as usize);
        let Some((_, cur_rect)) = rects.iter().find(|(id, _)| *id == self.focused_pane) else {
            return;
        };

        let mut best: Option<(PaneId, i64)> = None;

        for (id, r) in &rects {
            if *id == self.focused_pane {
                continue;
            }

            let score = match (dir, positive) {
                (SplitDir::Vertical, false) => {
                    // left
                    if r.x + r.w <= cur_rect.x {
                        let overlap = overlap_1d(r.y, r.h, cur_rect.y, cur_rect.h);
                        if overlap == 0 {
                            continue;
                        }
                        let dist = (cur_rect.x - (r.x + r.w)) as i64;
                        (overlap as i64) * 1000 - dist
                    } else {
                        continue;
                    }
                }
                (SplitDir::Vertical, true) => {
                    // right
                    if cur_rect.x + cur_rect.w <= r.x {
                        let overlap = overlap_1d(r.y, r.h, cur_rect.y, cur_rect.h);
                        if overlap == 0 {
                            continue;
                        }
                        let dist = (r.x - (cur_rect.x + cur_rect.w)) as i64;
                        (overlap as i64) * 1000 - dist
                    } else {
                        continue;
                    }
                }
                (SplitDir::Horizontal, false) => {
                    // up
                    if r.y + r.h <= cur_rect.y {
                        let overlap = overlap_1d(r.x, r.w, cur_rect.x, cur_rect.w);
                        if overlap == 0 {
                            continue;
                        }
                        let dist = (cur_rect.y - (r.y + r.h)) as i64;
                        (overlap as i64) * 1000 - dist
                    } else {
                        continue;
                    }
                }
                (SplitDir::Horizontal, true) => {
                    // down
                    if cur_rect.y + cur_rect.h <= r.y {
                        let overlap = overlap_1d(r.x, r.w, cur_rect.x, cur_rect.w);
                        if overlap == 0 {
                            continue;
                        }
                        let dist = (r.y - (cur_rect.y + cur_rect.h)) as i64;
                        (overlap as i64) * 1000 - dist
                    } else {
                        continue;
                    }
                }
            };

            if best
                .map(|(_, best_score)| score > best_score)
                .unwrap_or(true)
            {
                best = Some((*id, score));
            }
        }

        if let Some((id, _)) = best {
            self.set_focus(id);
            self.update_cursor();
            self.request_redraw();
        }
    }

    fn resize_focused(&mut self, dir: SplitDir, negative: bool) {
        let step = 0.05;
        let delta = match (dir, negative) {
            (SplitDir::Vertical, true) => -step,
            (SplitDir::Vertical, false) => step,
            (SplitDir::Horizontal, true) => -step,
            (SplitDir::Horizontal, false) => step,
        };

        let mut done = false;
        self.layout
            .adjust_ratio_for_pane(self.focused_pane, dir, delta, &mut done);
        if done {
            self.layout_dirty = true;
            self.request_redraw();
        }
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

        self.handle_resize();
        self.request_redraw();
    }

    /// Handles window resize events.
    fn handle_resize(&mut self) {
        let Some(graphics) = &mut self.graphics else {
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

        self.layout_dirty = true;
    }

    fn resize_panes_if_needed(&mut self, buffer_width: u32, buffer_height: u32) {
        if (buffer_width, buffer_height) == self.last_buffer_size && !self.layout_dirty {
            return;
        }

        let (rects, _) = self.pane_rects(buffer_width as usize, buffer_height as usize);

        for (id, rect) in rects {
            if let Some(pane) = self.panes.get(&id) {
                pane.terminal
                    .resize(rect.w as u32, rect.h as u32, CELL_W, CELL_H);
            }
        }

        self.last_buffer_size = (buffer_width, buffer_height);
        self.layout_dirty = false;
    }

    /// Renders all panes.
    fn render(&mut self) {
        // Probe buffer dimensions and snapshot palette without holding borrows.
        let (buffer_width, buffer_height, palette) = {
            let Some(graphics) = &mut self.graphics else {
                return;
            };
            let (bw, bh) = match graphics.surface.buffer_mut() {
                Ok(buffer) => (buffer.width().get(), buffer.height().get()),
                Err(e) => {
                    eprintln!("softbuffer buffer_mut failed: {e:?}");
                    return;
                }
            };
            (bw, bh, graphics.palette.clone())
        };

        // Update PTY sizes if layout/buffer changed.
        self.resize_panes_if_needed(buffer_width, buffer_height);

        let (rects, dividers) = self.pane_rects(buffer_width as usize, buffer_height as usize);

        let Some(graphics) = &mut self.graphics else {
            return;
        };

        let mut buffer = match graphics.surface.buffer_mut() {
            Ok(buffer) => buffer,
            Err(e) => {
                eprintln!("softbuffer buffer_mut failed: {e:?}");
                return;
            }
        };

        buffer.fill(self.config.colors.background);

        for (id, rect) in &rects {
            let Some(pane) = self.panes.get_mut(id) else {
                continue;
            };

            if rect.w < CELL_W || rect.h < CELL_H {
                continue;
            }

            if let Err(e) = self.renderer.paint_terminal_region(
                &mut buffer,
                buffer_width as usize,
                buffer_height as usize,
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                &pane.terminal,
                &palette,
                &mut pane.view,
            ) {
                eprintln!("Render pane {id} error: {e:#}");
            }

            if *id == self.focused_pane {
                draw_border(
                    &mut buffer,
                    buffer_width as usize,
                    buffer_height as usize,
                    *rect,
                    0x66AAFF,
                );
            }
        }

        for d in dividers {
            fill_rect(
                &mut buffer,
                buffer_width as usize,
                buffer_height as usize,
                Rect {
                    x: d.x,
                    y: d.y,
                    w: d.w,
                    h: d.h,
                },
                0x2A2A2A,
            );
        }

        if self.show_help {
            self.help_scroll = self.help_scroll.min(self.help_max_scroll);

            let mut by_category: HashMap<&'static str, Vec<(String, String)>> = HashMap::new();
            for (key, action) in &self.config.keybinds.bindings {
                by_category
                    .entry(action.category())
                    .or_default()
                    .push((key.clone(), action.label().to_string()));
            }

            for bindings in by_category.values_mut() {
                bindings.sort_by(|a, b| a.0.cmp(&b.0));
            }

            let order = ["General", "Panes", "Scrollback", "Search", "Help"];
            let mut sections: Vec<HelpSection> = Vec::new();

            for category in order {
                if let Some(bindings) = by_category.remove(category) {
                    sections.push(HelpSection {
                        title: category.to_string(),
                        bindings,
                    });
                }
            }

            let mut extra: Vec<(&'static str, Vec<(String, String)>)> =
                by_category.into_iter().collect();
            extra.sort_by(|a, b| a.0.cmp(b.0));
            for (category, bindings) in extra {
                sections.push(HelpSection {
                    title: category.to_string(),
                    bindings,
                });
            }

            let (scroll, max_scroll) = self.renderer.paint_help_overlay(
                &mut buffer,
                buffer_width as usize,
                buffer_height as usize,
                "Shortcuts",
                &sections,
                self.help_scroll,
            );
            self.help_scroll = scroll;
            self.help_max_scroll = max_scroll;
        }

        if let Err(e) = buffer.present() {
            eprintln!("softbuffer present failed: {e:?}");
        }
    }

    /// Requests a window redraw.
    fn request_redraw(&self) {
        if let Some(graphics) = &self.graphics {
            graphics.surface.window().request_redraw();
        }
    }

    /// Converts a key event into a stable key string for keybind matching.
    ///
    /// Important: this is based on the *physical* key code where possible, so
    /// `ctrl+shift+-` matches "-" (not "_") and `ctrl+shift+\\` matches "\\" (not "|").
    fn key_event_to_string(event: &winit::event::KeyEvent) -> Option<String> {
        if let PhysicalKey::Code(code) = event.physical_key {
            let s = match code {
                // Letters
                KeyCode::KeyA => "a",
                KeyCode::KeyB => "b",
                KeyCode::KeyC => "c",
                KeyCode::KeyD => "d",
                KeyCode::KeyE => "e",
                KeyCode::KeyF => "f",
                KeyCode::KeyG => "g",
                KeyCode::KeyH => "h",
                KeyCode::KeyI => "i",
                KeyCode::KeyJ => "j",
                KeyCode::KeyK => "k",
                KeyCode::KeyL => "l",
                KeyCode::KeyM => "m",
                KeyCode::KeyN => "n",
                KeyCode::KeyO => "o",
                KeyCode::KeyP => "p",
                KeyCode::KeyQ => "q",
                KeyCode::KeyR => "r",
                KeyCode::KeyS => "s",
                KeyCode::KeyT => "t",
                KeyCode::KeyU => "u",
                KeyCode::KeyV => "v",
                KeyCode::KeyW => "w",
                KeyCode::KeyX => "x",
                KeyCode::KeyY => "y",
                KeyCode::KeyZ => "z",

                // Digits
                KeyCode::Digit0 => "0",
                KeyCode::Digit1 => "1",
                KeyCode::Digit2 => "2",
                KeyCode::Digit3 => "3",
                KeyCode::Digit4 => "4",
                KeyCode::Digit5 => "5",
                KeyCode::Digit6 => "6",
                KeyCode::Digit7 => "7",
                KeyCode::Digit8 => "8",
                KeyCode::Digit9 => "9",

                // Punctuation
                KeyCode::Minus => "-",
                KeyCode::Equal => "=",
                KeyCode::Backquote => "`",
                KeyCode::Backslash | KeyCode::IntlBackslash => "\\",
                KeyCode::Slash => "/",
                KeyCode::Comma => ",",
                KeyCode::Period => ".",
                KeyCode::Semicolon => ";",
                KeyCode::Quote => "'",
                KeyCode::BracketLeft => "[",
                KeyCode::BracketRight => "]",

                // Navigation
                KeyCode::Enter => "enter",
                KeyCode::Tab => "tab",
                KeyCode::Space => "space",
                KeyCode::Backspace => "backspace",
                KeyCode::Escape => "escape",
                KeyCode::Insert => "insert",
                KeyCode::Delete => "delete",
                KeyCode::Home => "home",
                KeyCode::End => "end",
                KeyCode::PageUp => "pageup",
                KeyCode::PageDown => "pagedown",
                KeyCode::ArrowUp => "up",
                KeyCode::ArrowDown => "down",
                KeyCode::ArrowLeft => "left",
                KeyCode::ArrowRight => "right",

                // Function keys
                KeyCode::F1 => "f1",
                KeyCode::F2 => "f2",
                KeyCode::F3 => "f3",
                KeyCode::F4 => "f4",
                KeyCode::F5 => "f5",
                KeyCode::F6 => "f6",
                KeyCode::F7 => "f7",
                KeyCode::F8 => "f8",
                KeyCode::F9 => "f9",
                KeyCode::F10 => "f10",
                KeyCode::F11 => "f11",
                KeyCode::F12 => "f12",

                _ => "",
            };

            if !s.is_empty() {
                return Some(s.to_string());
            }
        }

        // Fallback for platforms/keys without a physical keycode.
        match &event.logical_key {
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

        let modifiers = self.input.modifiers;
        let ctrl = modifiers.control_key();
        let shift = modifiers.shift_key();
        let alt = modifiers.alt_key();

        let key_str = Self::key_event_to_string(event);
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
                needs_redraw |= Self::apply_search_input(&mut pane.view, modifiers, action, event);
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

    fn apply_search_input(
        view: &mut TerminalView,
        modifiers: ModifiersState,
        action: Option<Action>,
        event: &winit::event::KeyEvent,
    ) -> bool {
        if let Some(action) = action {
            match action {
                Action::SearchClose => {
                    view.deactivate_search();
                    return true;
                }
                Action::SearchConfirm => {
                    if view.search_match_count() > 0 {
                        view.search_next();
                    }
                    return true;
                }
                Action::SearchNext => {
                    view.search_next();
                    return true;
                }
                Action::SearchPrev => {
                    view.search_prev();
                    return true;
                }
                Action::SearchToggleCase => {
                    view.search_toggle_case();
                    return true;
                }
                _ => {}
            }
        }

        match &event.logical_key {
            Key::Named(NamedKey::Backspace) => {
                view.search_pop_char();
                true
            }
            Key::Character(s) => {
                if !modifiers.control_key() && !modifiers.alt_key() {
                    for ch in s.chars() {
                        view.search_push_char(ch);
                    }
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Executes a configured action.
    fn execute_action(&mut self, action: Action) {
        match action {
            Action::SplitVertical => self.split_focused(SplitDir::Vertical),
            Action::SplitHorizontal => self.split_focused(SplitDir::Horizontal),

            Action::FocusLeft => self.focus_move(SplitDir::Vertical, false),
            Action::FocusRight => self.focus_move(SplitDir::Vertical, true),
            Action::FocusUp => self.focus_move(SplitDir::Horizontal, false),
            Action::FocusDown => self.focus_move(SplitDir::Horizontal, true),

            // Resize: we interpret arrows as "expand the pane in that direction".
            Action::ResizeLeft => self.resize_focused(SplitDir::Vertical, false),
            Action::ResizeRight => self.resize_focused(SplitDir::Vertical, true),
            Action::ResizeUp => self.resize_focused(SplitDir::Horizontal, false),
            Action::ResizeDown => self.resize_focused(SplitDir::Horizontal, true),
            Action::ClosePane => self.close_focused_pane(),
            Action::ToggleHelp => {
                self.show_help = !self.show_help;
                if self.show_help {
                    self.help_scroll = 0;
                    self.help_max_scroll = 0;
                }
                self.request_redraw();
            }

            Action::Copy => self.handle_copy(),
            Action::Paste => self.handle_paste(),

            Action::ScrollPageUp => {
                if let Some(pane) = self.focused_pane_mut() {
                    pane.view.scrollback_scroll_by(24);
                }
                self.request_redraw();
            }
            Action::ScrollPageDown => {
                if let Some(pane) = self.focused_pane_mut() {
                    pane.view.scrollback_scroll_by(-24);
                }
                self.request_redraw();
            }
            Action::ScrollLineUp => {
                if let Some(pane) = self.focused_pane_mut() {
                    pane.view.scrollback_scroll_by(1);
                }
                self.request_redraw();
            }
            Action::ScrollLineDown => {
                if let Some(pane) = self.focused_pane_mut() {
                    pane.view.scrollback_scroll_by(-1);
                }
                self.request_redraw();
            }
            Action::ScrollToTop => {
                if let Some(pane) = self.focused_pane_mut() {
                    pane.view.scrollback_scroll_by(isize::MAX);
                }
                self.request_redraw();
            }
            Action::ScrollToBottom => {
                if let Some(pane) = self.focused_pane_mut() {
                    pane.view.scrollback_scroll_by(isize::MIN);
                }
                self.request_redraw();
            }
            Action::ClearScrollback => {
                if let Some(pane) = self.focused_pane_mut() {
                    pane.terminal.clear_scrollback();
                    pane.view.clear_scrollback();
                }
                self.request_redraw();
            }
            Action::Reset => {
                if let Some(pane) = self.focused_pane_mut() {
                    pane.terminal.clear_scrollback();
                    pane.view.clear_scrollback();
                    pane.view.clear_selection();
                }
                self.request_redraw();
            }
            Action::SearchFind => {
                if let Some(pane) = self.focused_pane_mut() {
                    pane.view.activate_search();
                }
                self.request_redraw();
            }

            // Search mode actions are handled inside `handle_search_keyboard`.
            Action::SearchClose
            | Action::SearchNext
            | Action::SearchPrev
            | Action::SearchToggleCase
            | Action::SearchConfirm => {}
        }
    }

    /// Handles paste from clipboard.
    fn handle_paste(&mut self) {
        let text = self.clipboard.read();
        let Some(text) = text else {
            return;
        };
        if text.is_empty() {
            return;
        }

        if let Some(pane) = self.focused_pane_mut() {
            pane.terminal.write(text.as_bytes());
            self.request_redraw();
        }
    }

    /// Handles copy to clipboard.
    fn handle_copy(&mut self) {
        let selected_text = self
            .focused_pane_mut()
            .and_then(|pane| pane.view.get_selected_text());

        if let Some(text) = selected_text {
            if self.clipboard.write(&text) {
                eprintln!("Copied {} characters to clipboard", text.len());
            }
        }
    }

    fn localize_pos(rect: Rect, pos: PhysicalPosition<f64>) -> PhysicalPosition<f64> {
        PhysicalPosition::new(pos.x - rect.x as f64, pos.y - rect.y as f64)
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
                if let Some((row, col)) = pane.view.window_to_cell(local.x, local.y) {
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

        if self.input.mouse_selecting && pane_id == self.focused_pane {
            if let Some((row, col)) = pane.view.window_to_cell(local.x, local.y) {
                pane.view.update_selection(row, col);
                self.request_redraw();
            }
        } else {
            if let Some((row, col)) = pane.view.window_to_cell(local.x, local.y) {
                if pane.view.update_url_hover(row, col) {
                    self.request_redraw();
                }
            } else {
                pane.view.clear_url_hover();
            }
            self.update_cursor();
        }
    }

    /// Updates the cursor icon based on hover state.
    fn update_cursor(&self) {
        let Some(graphics) = &self.graphics else {
            return;
        };

        let (buffer_width, buffer_height) = self.last_buffer_size;
        if buffer_width == 0 || buffer_height == 0 {
            return;
        }

        let (rects, _divs) = self.pane_rects(buffer_width as usize, buffer_height as usize);
        let hovered_pane = self
            .pane_at_position(&rects, self.input.cursor_position)
            .map(|(id, _)| id)
            .unwrap_or(self.focused_pane);

        let cursor = if self
            .panes
            .get(&hovered_pane)
            .map(|p| p.view.has_hovered_url())
            .unwrap_or(false)
        {
            CursorIcon::Pointer
        } else {
            CursorIcon::Text
        };

        graphics.surface.window().set_cursor(cursor);
    }

    /// Handles mouse wheel scrolling.
    fn handle_scroll(&mut self, delta: MouseScrollDelta) {
        let lines = match delta {
            MouseScrollDelta::LineDelta(_, y) => {
                (y * self.config.terminal.scroll_speed).round() as isize
            }
            MouseScrollDelta::PixelDelta(pos) => (pos.y / CELL_H as f64).round() as isize,
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

fn overlap_1d(a0: usize, a_len: usize, b0: usize, b_len: usize) -> usize {
    let a1 = a0 + a_len;
    let b1 = b0 + b_len;
    let start = a0.max(b0);
    let end = a1.min(b1);
    end.saturating_sub(start)
}

fn fill_rect(
    buffer: &mut [u32],
    buffer_width: usize,
    buffer_height: usize,
    rect: Rect,
    color: u32,
) {
    let x1 = (rect.x + rect.w).min(buffer_width);
    let y1 = (rect.y + rect.h).min(buffer_height);

    for y in rect.y.min(buffer_height)..y1 {
        let row = y * buffer_width;
        for x in rect.x.min(buffer_width)..x1 {
            buffer[row + x] = color;
        }
    }
}

fn draw_border(
    buffer: &mut [u32],
    buffer_width: usize,
    buffer_height: usize,
    rect: Rect,
    color: u32,
) {
    if rect.w == 0 || rect.h == 0 {
        return;
    }

    let x0 = rect.x.min(buffer_width);
    let y0 = rect.y.min(buffer_height);
    let x1 = (rect.x + rect.w).min(buffer_width);
    let y1 = (rect.y + rect.h).min(buffer_height);

    if x1 <= x0 || y1 <= y0 {
        return;
    }

    // Top and bottom
    for x in x0..x1 {
        buffer[y0 * buffer_width + x] = color;
        buffer[(y1 - 1) * buffer_width + x] = color;
    }

    // Left and right
    for y in y0..y1 {
        buffer[y * buffer_width + x0] = color;
        buffer[y * buffer_width + (x1 - 1)] = color;
    }
}

/// Spawns a thread to read PTY output and send events.
fn spawn_pty_reader(
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
