use anyhow::{anyhow, Result};

use super::super::Terminal;

impl Terminal {
    /// Returns the current shell-reported title, if any.
    pub fn shell_title(&self) -> Option<String> {
        // Avoid false positives from the terminal model's default title.
        if !self.shell_integration_status().osc_title {
            return None;
        }

        let term = self.term.lock().ok()?;
        let title = term.get_title().trim();
        if title.is_empty() {
            None
        } else {
            Some(title.to_string())
        }
    }

    /// Returns the current shell-reported working directory (OSC 7), if any.
    pub fn shell_cwd(&self) -> Option<String> {
        if !self.shell_integration_status().osc7_cwd {
            return None;
        }

        let term = self.term.lock().ok()?;
        term.get_current_dir().map(|u| u.to_string())
    }

    /// Computes semantic zones from OSC 133 markers (prompt/input/output).
    pub fn semantic_zones(&self) -> Result<Vec<tattoy_wezterm_term::SemanticZone>> {
        let mut term = self
            .term
            .lock()
            .map_err(|_| anyhow!("terminal mutex poisoned"))?;
        let zones = term.get_semantic_zones()?;

        let has_markers = zones
            .iter()
            .any(|z| z.semantic_type != tattoy_wezterm_term::SemanticType::Output);

        if has_markers {
            if let Ok(mut state) = self.shell_integration.lock() {
                state.osc133_semantic = true;
            }
        }

        Ok(zones)
    }
}
