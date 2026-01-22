use super::Action;

impl Action {
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
            Action::ToggleShadowPrompt => "Toggle shadow prompt",

            Action::CycleProfile => "Cycle profile",
            Action::CycleProfileReverse => "Cycle profile (reverse)",
            Action::SwitchToProfile1 => "Switch to profile 1",
            Action::SwitchToProfile2 => "Switch to profile 2",
            Action::SwitchToProfile3 => "Switch to profile 3",
            Action::SwitchToProfile4 => "Switch to profile 4",
            Action::SwitchToProfile5 => "Switch to profile 5",
            Action::SwitchToProfile6 => "Switch to profile 6",
            Action::SwitchToProfile7 => "Switch to profile 7",
            Action::SwitchToProfile8 => "Switch to profile 8",
            Action::SwitchToProfile9 => "Switch to profile 9",

            Action::ReloadConfig => "Reload config",
        }
    }
}
