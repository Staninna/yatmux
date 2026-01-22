use crate::app::layout::{PaneId, Rect, SplitDir};
use crate::app::tab::Tab;

impl Tab {
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
