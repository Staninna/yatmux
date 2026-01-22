use super::super::*;

impl App {
    pub(super) fn open_search(&mut self) {
        if let Some(pane) = self.focused_pane_mut() {
            pane.view.activate_search();
        }
        self.request_redraw();
    }
}
