//! Pane management for the terminal application.

use std::sync::Arc;

use winit::window::CursorIcon;

use term::renderer::TerminalView;
use term::terminal::Terminal;

use crate::app::layout::{LayoutNode, PaneId, SplitDir, overlap_1d};
use crate::app::{App, spawn_pty_reader};

/// A terminal pane with its view and scale.
pub struct Pane {
    pub terminal: Terminal,
    pub view: TerminalView,
    pub scale: usize,
}

impl App {
    /// Records a pane ID in the focus history.
    pub fn record_focus(&mut self, id: PaneId) {
        self.focus_history.retain(|&x| x != id);
        self.focus_history.push(id);
    }

    /// Sets the focused pane.
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

    /// Zooms the focused pane by the given delta.
    pub fn zoom_focused(&mut self, delta: isize) {
        let Some(pane) = self.focused_pane_mut() else {
            return;
        };

        let new_scale = (pane.scale as isize + delta).clamp(1, 8) as usize;
        if new_scale == pane.scale {
            return;
        }

        pane.scale = new_scale;
        self.layout_dirty = true;
        self.update_cursor();
        self.request_redraw();
    }

    /// Resets the focused pane's zoom to the default scale.
    pub fn zoom_reset_focused(&mut self) {
        let new_scale = self.config.font.scale.clamp(1, 8);

        let Some(pane) = self.focused_pane_mut() else {
            return;
        };

        if pane.scale == new_scale {
            return;
        }

        pane.scale = new_scale;
        self.layout_dirty = true;
        self.update_cursor();
        self.request_redraw();
    }

    /// Computes the cell size for a given scale.
    pub fn cell_size_for_scale(scale: usize) -> (usize, usize) {
        let scale = scale.clamp(1, 8);
        (8 * scale, 8 * scale)
    }

    /// Spawns a new pane with the given ID and scale.
    pub fn spawn_pane(&mut self, id: PaneId, scale: usize) {
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
                scale: scale.clamp(1, 8),
            },
        );
    }

    /// Splits the focused pane in the given direction.
    pub fn split_focused(&mut self, dir: SplitDir) {
        let focused = self.focused_pane;
        if !self.layout.contains_pane(focused) {
            return;
        }

        let new_id = self.next_pane_id;
        self.next_pane_id += 1;
        let focused_scale = self
            .panes
            .get(&focused)
            .map(|p| p.scale)
            .unwrap_or(self.config.font.scale);

        self.spawn_pane(new_id, focused_scale);

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

    /// Closes a pane by ID.
    pub fn close_pane(&mut self, target: PaneId) {
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

    /// Closes the currently focused pane.
    pub fn close_focused_pane(&mut self) {
        let target = self.focused_pane;
        self.close_pane(target);
    }

    /// Moves focus in the given direction.
    pub fn focus_move(&mut self, dir: SplitDir, positive: bool) {
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

    /// Resizes the focused pane in the given direction.
    pub fn resize_focused(&mut self, dir: SplitDir, negative: bool) {
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

    /// Updates the cursor icon based on hover state.
    pub fn update_cursor(&self) {
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

    /// Handles paste from clipboard.
    pub fn handle_paste(&mut self) {
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
    pub fn handle_copy(&mut self) {
        let selected_text = self
            .focused_pane_mut()
            .and_then(|pane| pane.view.get_selected_text());

        if let Some(text) = selected_text {
            if self.clipboard.write(&text) {
                eprintln!("Copied {} characters to clipboard", text.len());
            }
        }
    }
}
