use crate::app::layout::{overlap_1d, PaneId, Rect, SplitDir};
use crate::app::tab::Tab;
use crate::app::pane::Pane;

impl Tab {
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
}
