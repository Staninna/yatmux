//! Terminal actions that can be bound to keys.

use serde::{Deserialize, Serialize};

/// Terminal actions that can be bound to keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Copy selected text to clipboard.
    Copy,
    /// Paste from clipboard.
    Paste,

    /// Split the focused pane vertically.
    SplitVertical,
    /// Split the focused pane horizontally.
    SplitHorizontal,
    /// Move focus to the pane on the left.
    FocusLeft,
    /// Move focus to the pane on the right.
    FocusRight,
    /// Move focus to the pane above.
    FocusUp,
    /// Move focus to the pane below.
    FocusDown,
    /// Resize split to give more space left.
    ResizeLeft,
    /// Resize split to give more space right.
    ResizeRight,
    /// Resize split to give more space up.
    ResizeUp,
    /// Resize split to give more space down.
    ResizeDown,
    /// Close the focused pane.
    ClosePane,

    /// Toggle help popover.
    ToggleHelp,

    /// Increase font scale in the focused pane.
    ZoomIn,
    /// Decrease font scale in the focused pane.
    ZoomOut,
    /// Reset font scale in the focused pane.
    ZoomReset,

    /// Scroll up by one page.
    ScrollPageUp,
    /// Scroll down by one page.
    ScrollPageDown,
    /// Scroll up by one line.
    ScrollLineUp,
    /// Scroll down by one line.
    ScrollLineDown,
    /// Scroll to top of scrollback.
    ScrollToTop,
    /// Scroll to bottom (live view).
    ScrollToBottom,
    /// Clear the scrollback buffer.
    ClearScrollback,
    /// Reset the terminal.
    Reset,

    /// Open search mode.
    SearchFind,
    /// Close search mode.
    SearchClose,
    /// Navigate to next search match.
    SearchNext,
    /// Navigate to previous search match.
    SearchPrev,
    /// Toggle search case sensitivity.
    SearchToggleCase,
    /// Confirm search / go to current match.
    SearchConfirm,
}

impl Action {
    /// Returns true if this action only applies in search mode.
    pub fn is_search_mode_only(&self) -> bool {
        matches!(
            self,
            Action::SearchClose
                | Action::SearchNext
                | Action::SearchPrev
                | Action::SearchToggleCase
                | Action::SearchConfirm
        )
    }

    /// Returns the category name for this action.
    pub fn category(&self) -> &'static str {
        match self {
            Action::Copy | Action::Paste => "General",

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
            | Action::ClosePane => "Panes",

            Action::ToggleHelp => "Help",

            Action::ZoomIn | Action::ZoomOut | Action::ZoomReset => "Zoom",

            Action::ScrollPageUp
            | Action::ScrollPageDown
            | Action::ScrollLineUp
            | Action::ScrollLineDown
            | Action::ScrollToTop
            | Action::ScrollToBottom
            | Action::ClearScrollback
            | Action::Reset => "Scrollback",

            Action::SearchFind
            | Action::SearchClose
            | Action::SearchNext
            | Action::SearchPrev
            | Action::SearchToggleCase
            | Action::SearchConfirm => "Search",
        }
    }

    /// Returns the human-readable label for this action.
    pub fn label(&self) -> &'static str {
        match self {
            Action::Copy => "Copy",
            Action::Paste => "Paste",

            Action::SplitVertical => "Split vertical",
            Action::SplitHorizontal => "Split horizontal",
            Action::ClosePane => "Close pane",
            Action::FocusLeft => "Focus left pane",
            Action::FocusRight => "Focus right pane",
            Action::FocusUp => "Focus upper pane",
            Action::FocusDown => "Focus lower pane",
            Action::ResizeLeft => "Resize: give left",
            Action::ResizeRight => "Resize: give right",
            Action::ResizeUp => "Resize: give up",
            Action::ResizeDown => "Resize: give down",

            Action::ScrollPageUp => "Scroll page up",
            Action::ScrollPageDown => "Scroll page down",
            Action::ScrollLineUp => "Scroll line up",
            Action::ScrollLineDown => "Scroll line down",
            Action::ScrollToTop => "Scroll to top",
            Action::ScrollToBottom => "Scroll to bottom",
            Action::ClearScrollback => "Clear scrollback",
            Action::Reset => "Reset",

            Action::SearchFind => "Search",
            Action::SearchClose => "Close search",
            Action::SearchNext => "Search next",
            Action::SearchPrev => "Search prev",
            Action::SearchToggleCase => "Toggle search case",
            Action::SearchConfirm => "Search confirm",

            Action::ToggleHelp => "Toggle help",

            Action::ZoomIn => "Zoom in",
            Action::ZoomOut => "Zoom out",
            Action::ZoomReset => "Zoom reset",
        }
    }
}
