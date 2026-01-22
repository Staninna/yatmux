use super::super::*;

impl App {
    pub(super) fn scroll_page(&mut self, lines: isize) {
        if let Some(pane) = self.focused_pane_mut() {
            pane.view.scrollback_scroll_by(lines);
        }
        self.request_redraw();
    }

    pub(super) fn scroll_to_top(&mut self) {
        if let Some(pane) = self.focused_pane_mut() {
            pane.view.scrollback_scroll_by(isize::MAX);
        }
        self.request_redraw();
    }

    pub(super) fn scroll_to_bottom(&mut self) {
        if let Some(pane) = self.focused_pane_mut() {
            pane.view.scrollback_scroll_by(isize::MIN);
        }
        self.request_redraw();
    }

    pub(super) fn clear_scrollback(&mut self) {
        if let Some(pane) = self.focused_pane_mut() {
            pane.terminal.clear_scrollback();
            pane.view.clear_scrollback();
        }
        self.request_redraw();
    }

    pub(super) fn reset_terminal(&mut self) {
        if let Some(pane) = self.focused_pane_mut() {
            pane.terminal.clear_scrollback();
            pane.view.clear_scrollback();
            pane.view.clear_selection();
        }
        self.request_redraw();
    }

    /// Jumps to the previous or next prompt in scrollback.
    pub(super) fn jump_to_prompt(&mut self, forward: bool) {
        // First gather the data we need with an immutable borrow
        let (prompts, visible_start, current_offset) = {
            let Some(pane) = self.focused_pane_mut() else {
                return;
            };
            let prompts = pane.terminal.prompt_positions();
            let visible_start = pane.terminal.visible_start_row();
            let current_offset = pane.view.scrollback_offset();
            (prompts, visible_start, current_offset)
        };

        if prompts.is_empty() {
            return;
        }

        // Current view is showing rows starting at: visible_start - current_offset
        let current_top = visible_start.saturating_sub(current_offset);

        // Find the target prompt
        let target_prompt = if forward {
            // Find the next prompt after current view
            prompts.iter().find(|&&p| p > current_top).copied()
        } else {
            // Find the previous prompt before current view
            prompts.iter().rev().find(|&&p| p < current_top).copied()
        };

        let Some(target) = target_prompt else {
            return;
        };

        // Calculate the scroll offset to show this prompt at the top
        // offset = visible_start - target
        let new_offset = visible_start.saturating_sub(target);

        // Get mutable reference and update
        if let Some(pane) = self.focused_pane_mut() {
            pane.view.scrollback_scroll_to(new_offset);
        }
        self.request_redraw();
    }
}
