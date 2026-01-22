use std::sync::{Arc, Mutex};

use tattoy_wezterm_term::{Alert, AlertHandler};

use super::super::Terminal;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ShellIntegrationStatus {
    pub osc7_cwd: bool,
    pub osc133_semantic: bool,
    pub osc_title: bool,
}

impl ShellIntegrationStatus {
    pub fn any(&self) -> bool {
        self.osc7_cwd || self.osc133_semantic || self.osc_title
    }
}

#[derive(Debug, Default)]
pub(crate) struct ShellIntegrationState {
    pub(super) osc7_cwd: bool,
    pub(super) osc133_semantic: bool,
    pub(super) osc_title: bool,
}

#[derive(Clone)]
pub(crate) struct ShellIntegrationAlertHandler {
    pub(crate) state: Arc<Mutex<ShellIntegrationState>>,
}

impl AlertHandler for ShellIntegrationAlertHandler {
    fn alert(&mut self, alert: Alert) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };

        match alert {
            Alert::CurrentWorkingDirectoryChanged => {
                state.osc7_cwd = true;
            }
            Alert::WindowTitleChanged(_) | Alert::TabTitleChanged(_) | Alert::IconTitleChanged(_) => {
                state.osc_title = true;
            }
            _ => {}
        }
    }
}

impl Terminal {
    pub fn shell_integration_status(&self) -> ShellIntegrationStatus {
        let Ok(state) = self.shell_integration.lock() else {
            return ShellIntegrationStatus::default();
        };

        ShellIntegrationStatus {
            osc7_cwd: state.osc7_cwd,
            osc133_semantic: state.osc133_semantic,
            osc_title: state.osc_title,
        }
    }
}
