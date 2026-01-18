//! Tab management for the terminal application.
//!
//! Each tab contains its own set of panes with an independent layout tree.

use std::collections::HashMap;

use winit::event_loop::EventLoopProxy;

use yatmux::renderer::TerminalView;
use yatmux::terminal::Terminal;

use crate::app::layout::{LayoutNode, PaneId, Rect, SplitDir, overlap_1d};
use crate::app::pane::Pane;
use crate::app::{AppEvent, spawn_pty_reader};

/// Unique identifier for a tab.
pub type TabId = u64;

/// A tab containing a set of panes with their own layout.
pub struct Tab {
    pub id: TabId,
    pub title: String,
    pub panes: HashMap<PaneId, Pane>,
    pub layout: LayoutNode,
    pub focused_pane: PaneId,
    pub focus_history: Vec<PaneId>,
    pub next_pane_id: PaneId,
}

impl Tab {
    /// Creates a new empty tab with the given ID.
    pub fn new(id: TabId) -> Self {
        Tab {
            id,
            title: format!("Tab {}", id),
            panes: HashMap::new(),
            layout: LayoutNode::Leaf(1),
            focused_pane: 1,
            focus_history: vec![1],
            next_pane_id: 2,
        }
    }

    /// Returns true if this tab has no panes.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.panes.is_empty()
    }

    /// Computes pane rectangles and dividers for the current layout.
    pub fn pane_rects(
        &self,
        buffer_width: usize,
        buffer_height: usize,
    ) -> (Vec<(PaneId, Rect)>, Vec<crate::app::layout::Divider>) {
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

    /// Records a pane ID in the focus history.
    pub fn record_focus(&mut self, id: PaneId) {
        self.focus_history.retain(|&x| x != id);
        self.focus_history.push(id);
    }

    /// Sets the focused pane within this tab.
    pub fn set_focus(&mut self, id: PaneId) {
        if self.focused_pane == id {
            self.record_focus(id);
            return;
        }
        self.focused_pane = id;
        self.record_focus(id);
    }

    /// Falls back to the most recently focused pane that still exists.
    pub fn focus_fallback(&mut self) {
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

    /// Returns a mutable reference to the focused pane.
    pub fn focused_pane_mut(&mut self) -> Option<&mut Pane> {
        self.panes.get_mut(&self.focused_pane)
    }

    /// Returns an immutable reference to the focused pane.
    pub fn focused_pane(&self) -> Option<&Pane> {
        self.panes.get(&self.focused_pane)
    }

    /// Spawns a new pane with the given ID and scale.
    pub fn spawn_pane(
        &mut self,
        id: PaneId,
        scale: f32,
        scrollback_lines: usize,
        event_proxy: Option<&EventLoopProxy<AppEvent>>,
        tab_id: TabId,
        shadow_prompt_enabled: bool,
    ) {
        let (pty, reader) = match yatmux::pty::spawn_shell() {
            Ok(result) => result,
            Err(e) => {
                eprintln!("Failed to spawn shell: {e}");
                return;
            }
        };

        let terminal = Terminal::new_with_scrollback(std::sync::Arc::new(pty), scrollback_lines);

        if let Some(proxy) = event_proxy {
            spawn_pty_reader(reader, proxy.clone(), tab_id, id);
        }

        self.panes.insert(
            id,
            Pane {
                terminal,
                view: TerminalView::new(),
                scale: scale.clamp(0.25, 64.0),
                shell_title: None,
                shell_cwd: None,
                shell_integration: Default::default(),
                shadow_prompt: Default::default(),
                shadow_prompt_enabled,
                command_running: false,
            },
        );
    }

    /// Spawns the initial pane for this tab.
    pub fn spawn_initial_pane(
        &mut self,
        scale: f32,
        scrollback_lines: usize,
        event_proxy: Option<&EventLoopProxy<AppEvent>>,
        shadow_prompt_enabled: bool,
    ) {
        if !self.panes.is_empty() {
            return;
        }
        self.spawn_pane(
            1,
            scale,
            scrollback_lines,
            event_proxy,
            self.id,
            shadow_prompt_enabled,
        );
    }

    /// Splits the focused pane in the given direction.
    /// Returns false if the split was rejected (e.g., pane too small).
    pub fn split_focused(
        &mut self,
        dir: SplitDir,
        default_scale: f32,
        scrollback_lines: usize,
        event_proxy: Option<&EventLoopProxy<AppEvent>>,
        current_rect: Option<Rect>,
        min_pane_size: usize,
        shadow_prompt_enabled: bool,
    ) -> bool {
        let focused = self.focused_pane;
        if !self.layout.contains_pane(focused) {
            return false;
        }

        // Check if splitting would create panes that are too small
        if let Some(rect) = current_rect {
            let (new_w, new_h) = match dir {
                SplitDir::Vertical => (rect.w / 2, rect.h),
                SplitDir::Horizontal => (rect.w, rect.h / 2),
            };
            if new_w < min_pane_size || new_h < min_pane_size {
                return false;
            }
        }

        let new_id = self.next_pane_id;
        self.next_pane_id += 1;
        let focused_scale = self
            .panes
            .get(&focused)
            .map(|p| p.scale)
            .unwrap_or(default_scale);

        self.spawn_pane(
            new_id,
            focused_scale,
            scrollback_lines,
            event_proxy,
            self.id,
            shadow_prompt_enabled,
        );

        let replacement = LayoutNode::Split {
            dir,
            ratio: 0.5,
            a: Box::new(LayoutNode::Leaf(focused)),
            b: Box::new(LayoutNode::Leaf(new_id)),
        };

        if self.layout.replace_leaf(focused, replacement) {
            self.set_focus(new_id);
            true
        } else {
            false
        }
    }

    /// Closes a pane by ID. Returns true if the tab should be closed (no panes left).
    pub fn close_pane(&mut self, target: PaneId) -> bool {
        self.panes.remove(&target);
        self.focus_history.retain(|&x| x != target);

        if self.panes.is_empty() {
            return true;
        }

        let _ = self.layout.remove_pane(target);
        self.focus_fallback();
        false
    }

    /// Closes the currently focused pane. Returns true if the tab should be closed.
    pub fn close_focused_pane(&mut self) -> bool {
        let target = self.focused_pane;
        self.close_pane(target)
    }

    /// Moves focus in the given direction within this tab.
    pub fn focus_move(
        &mut self,
        dir: SplitDir,
        positive: bool,
        rects: &[(PaneId, Rect)],
        overlap_weight: i64,
    ) -> bool {
        let Some((_, cur_rect)) = rects.iter().find(|(id, _)| *id == self.focused_pane) else {
            return false;
        };

        let mut best: Option<(PaneId, i64)> = None;

        for (id, r) in rects {
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
                        (overlap as i64) * overlap_weight - dist
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
                        (overlap as i64) * overlap_weight - dist
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
                        (overlap as i64) * overlap_weight - dist
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
                        (overlap as i64) * overlap_weight - dist
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
            true
        } else {
            false
        }
    }

    /// Resizes the focused pane in the given direction.
    pub fn resize_focused(&mut self, dir: SplitDir, negative: bool, step: f32) -> bool {
        let delta = match (dir, negative) {
            (SplitDir::Vertical, true) => -step,
            (SplitDir::Vertical, false) => step,
            (SplitDir::Horizontal, true) => -step,
            (SplitDir::Horizontal, false) => step,
        };

        let mut done = false;
        self.layout
            .adjust_ratio_for_pane(self.focused_pane, dir, delta, &mut done);
        done
    }
}
