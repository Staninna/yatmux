use std::collections::HashMap;

use crate::app::layout::{LayoutNode, PaneId};
use crate::app::pane::Pane;

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
}
