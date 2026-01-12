//! Terminal actions that can be bound to keys.

use serde::{Deserialize, Serialize};

/// Terminal actions that can be bound to keys.
///
/// Use `none` to disable a keybinding:
/// ```toml
/// [keybinds]
/// "ctrl+shift+-" = "none"  # Disable horizontal split
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Disabled action - used to unbind a key.
    None,

    /// Copy selected text to clipboard.
    Copy,
    /// Paste from clipboard.
    Paste,

    /// Create a new tab.
    NewTab,
    /// Close the current tab.
    CloseTab,
    /// Switch to the next tab.
    NextTab,
    /// Switch to the previous tab.
    PrevTab,
    /// Switch to tab 1.
    Tab1,
    /// Switch to tab 2.
    Tab2,
    /// Switch to tab 3.
    Tab3,
    /// Switch to tab 4.
    Tab4,
    /// Switch to tab 5.
    Tab5,
    /// Switch to tab 6.
    Tab6,
    /// Switch to tab 7.
    Tab7,
    /// Switch to tab 8.
    Tab8,
    /// Switch to tab 9.
    Tab9,

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
    /// Toggle regex search mode.
    SearchToggleRegex,
    /// Confirm search / go to current match.
    SearchConfirm,

    /// Copy the last command's output to clipboard (requires shell integration).
    CopyLastOutput,
    /// Jump to the previous prompt in scrollback (requires shell integration).
    JumpToPrevPrompt,
    /// Jump to the next prompt in scrollback (requires shell integration).
    JumpToNextPrompt,
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
                | Action::SearchToggleRegex
                | Action::SearchConfirm
        )
    }

    /// Returns the category name for this action.
    pub fn category(&self) -> &'static str {
        match self {
            Action::None => "Disabled",

            Action::Copy | Action::Paste => "General",

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
            | Action::Tab9 => "Tabs",

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
            | Action::SearchToggleRegex
            | Action::SearchConfirm => "Search",

            Action::CopyLastOutput | Action::JumpToPrevPrompt | Action::JumpToNextPrompt => {
                "Shell Integration"
            }
        }
    }

    /// Returns the human-readable label for this action.
    pub fn label(&self) -> &'static str {
        match self {
            Action::None => "Disabled",

            Action::Copy => "Copy",
            Action::Paste => "Paste",

            Action::NewTab => "New tab",
            Action::CloseTab => "Close tab",
            Action::NextTab => "Next tab",
            Action::PrevTab => "Previous tab",
            Action::Tab1 => "Go to tab 1",
            Action::Tab2 => "Go to tab 2",
            Action::Tab3 => "Go to tab 3",
            Action::Tab4 => "Go to tab 4",
            Action::Tab5 => "Go to tab 5",
            Action::Tab6 => "Go to tab 6",
            Action::Tab7 => "Go to tab 7",
            Action::Tab8 => "Go to tab 8",
            Action::Tab9 => "Go to tab 9",

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
            Action::SearchNext => "Next match",
            Action::SearchPrev => "Previous match",
            Action::SearchToggleCase => "Toggle case",
            Action::SearchToggleRegex => "Toggle regex",
            Action::SearchConfirm => "Confirm",

            Action::ToggleHelp => "Toggle help",

            Action::ZoomIn => "Zoom in",
            Action::ZoomOut => "Zoom out",
            Action::ZoomReset => "Zoom reset",

            Action::CopyLastOutput => "Copy last output",
            Action::JumpToPrevPrompt => "Previous prompt",
            Action::JumpToNextPrompt => "Next prompt",
        }
    }
}
