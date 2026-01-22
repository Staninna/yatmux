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

    /// Toggle shadow prompt for the focused pane.
    ToggleShadowPrompt,

    /// Cycle to the next profile.
    CycleProfile,
    /// Cycle to the previous profile.
    CycleProfileReverse,
    /// Switch to profile 1.
    SwitchToProfile1,
    /// Switch to profile 2.
    SwitchToProfile2,
    /// Switch to profile 3.
    SwitchToProfile3,
    /// Switch to profile 4.
    SwitchToProfile4,
    /// Switch to profile 5.
    SwitchToProfile5,
    /// Switch to profile 6.
    SwitchToProfile6,
    /// Switch to profile 7.
    SwitchToProfile7,
    /// Switch to profile 8.
    SwitchToProfile8,
    /// Switch to profile 9.
    SwitchToProfile9,

    /// Reload `config.toml` from disk.
    ReloadConfig,
}
