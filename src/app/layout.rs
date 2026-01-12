//! Layout management for terminal panes using a binary tree structure.

/// Unique identifier for a pane.
pub type PaneId = u64;

/// A rectangle representing a region in screen space.
#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

impl Rect {
    /// Returns true if the point (x, y) is contained within this rectangle.
    pub fn contains(&self, x: f64, y: f64) -> bool {
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

/// Direction of a split between panes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitDir {
    /// Split left/right.
    Vertical,
    /// Split top/bottom.
    Horizontal,
}

/// A node in the binary tree layout structure.
#[derive(Clone, Debug)]
pub enum LayoutNode {
    /// A leaf node containing a single pane.
    Leaf(PaneId),
    /// A split node containing two children.
    Split {
        dir: SplitDir,
        ratio: f32,
        a: Box<LayoutNode>,
        b: Box<LayoutNode>,
    },
}

/// A divider line between panes.
#[derive(Clone, Copy, Debug)]
pub struct Divider {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

impl LayoutNode {
    /// Computes the rectangles for all leaf panes and dividers.
    pub fn leaf_rects(
        &self,
        rect: Rect,
        out: &mut Vec<(PaneId, Rect)>,
        dividers: &mut Vec<Divider>,
    ) {
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

    /// Replaces a leaf node with a new node. Returns true if successful.
    pub fn replace_leaf(&mut self, target: PaneId, replacement: LayoutNode) -> bool {
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

    /// Returns true if this node or any of its children contain the given pane.
    pub fn contains_pane(&self, target: PaneId) -> bool {
        match self {
            LayoutNode::Leaf(id) => *id == target,
            LayoutNode::Split { a, b, .. } => a.contains_pane(target) || b.contains_pane(target),
        }
    }

    /// Returns the first leaf pane ID in the tree.
    pub fn first_leaf(&self) -> Option<PaneId> {
        match self {
            LayoutNode::Leaf(id) => Some(*id),
            LayoutNode::Split { a, b, .. } => a.first_leaf().or_else(|| b.first_leaf()),
        }
    }

    /// Removes a pane from the tree. Returns true if the tree is still valid.
    pub fn remove_pane(&mut self, target: PaneId) -> bool {
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

    /// Adjusts the split ratio for the pane in the given direction.
    pub fn adjust_ratio_for_pane(
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

/// Computes the 1D overlap between two ranges.
pub fn overlap_1d(a0: usize, a_len: usize, b0: usize, b_len: usize) -> usize {
    let a1 = a0 + a_len;
    let b1 = b0 + b_len;
    let start = a0.max(b0);
    let end = a1.min(b1);
    end.saturating_sub(start)
}

/// Fills a rectangle with a solid color.
pub fn fill_rect(
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

/// Draws a 1px border around a rectangle.
pub fn draw_border(
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
