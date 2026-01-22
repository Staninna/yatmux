use super::Action;

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

            Action::CopyLastOutput
            | Action::JumpToPrevPrompt
            | Action::JumpToNextPrompt
            | Action::ToggleShadowPrompt => "Shell Integration",

            Action::CycleProfile
            | Action::CycleProfileReverse
            | Action::SwitchToProfile1
            | Action::SwitchToProfile2
            | Action::SwitchToProfile3
            | Action::SwitchToProfile4
            | Action::SwitchToProfile5
            | Action::SwitchToProfile6
            | Action::SwitchToProfile7
            | Action::SwitchToProfile8
            | Action::SwitchToProfile9 => "Profiles",

            Action::ReloadConfig => "Config",
        }
    }
}
