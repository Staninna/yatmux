use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow};
use vt100::Color;

use crate::core::grid::RowSnapshot;

use tattoy_wezterm_term::{Alert, AlertHandler};

use super::Terminal;
use super::adapters::color_attr_to_vt100;

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

/// Information about the current prompt for sticky prompt display.
#[derive(Debug, Clone)]
pub struct StickyPromptInfo {
    pub rows: Vec<RowSnapshot>,
    /// Cursor position relative to the prompt rows (row, col).
    pub cursor: Option<(usize, usize)>,
}

#[derive(Debug, Default)]
pub(super) struct ShellIntegrationState {
    pub(super) osc7_cwd: bool,
    pub(super) osc133_semantic: bool,
    pub(super) osc_title: bool,
}

#[derive(Clone)]
pub(super) struct ShellIntegrationAlertHandler {
    pub(super) state: Arc<Mutex<ShellIntegrationState>>,
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
            Alert::WindowTitleChanged(_)
            | Alert::TabTitleChanged(_)
            | Alert::IconTitleChanged(_) => {
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

    /// Returns the content of the current (last) prompt and input lines, if available.
    /// This is used for sticky prompt display when scrolled up.
    /// Returns the rows that make up the prompt+input (may be multiple lines) and cursor position.
    pub fn current_prompt_rows(&self) -> Option<StickyPromptInfo> {
        let mut term = self.term.lock().ok()?;
        let zones = term.get_semantic_zones().ok()?;

        // Find the last Prompt zone - this confirms we have shell integration active
        let _prompt_zone = zones
            .iter()
            .rev()
            .find(|z| z.semantic_type == tattoy_wezterm_term::SemanticType::Prompt)?;

        let screen = term.screen();
        let cursor = term.cursor_pos();
        let (term_rows, cols) = *self.size.lock().ok()?;
        let cols_usize = cols as usize;
        let term_rows_usize = term_rows as usize;

        // cursor.y is screen-relative (0 = top of visible area)
        // scrollback_rows() returns total lines in buffer (scrollback + visible)
        // Visible lines start at index: total_lines - terminal_height
        // So physical line of cursor = (total_lines - terminal_height) + cursor.y
        let total_lines = screen.scrollback_rows();
        let visible_start = total_lines.saturating_sub(term_rows_usize);
        let cursor_phys_y = visible_start + cursor.y as usize;
        let cursor_x = cursor.x as usize;

        // Collect the line at the cursor position
        let mut result_rows = Vec::new();

        screen.for_each_phys_line(|idx, line| {
            if idx == cursor_phys_y {
                let mut cells = Vec::with_capacity(cols_usize);
                for col in 0..cols_usize {
                    if let Some(cell) = line.get_cell(col) {
                        let grapheme = cell.str();
                        let ch = grapheme.chars().next().unwrap_or(' ');
                        let attrs = cell.attrs();
                        let fg = color_attr_to_vt100(attrs.foreground());
                        let bg = color_attr_to_vt100(attrs.background());
                        cells.push((ch, fg, bg));
                    } else {
                        cells.push((' ', Color::Default, Color::Default));
                    }
                }
                let tabs = vec![None; cols_usize];
                result_rows.push(RowSnapshot::new(cells, tabs));
            }
        });

        if result_rows.is_empty() {
            return None;
        }

        // Cursor is on row 0 (the only row we collected), at column cursor_x
        let relative_cursor = Some((0, cursor_x));

        Some(StickyPromptInfo {
            rows: result_rows,
            cursor: relative_cursor,
        })
    }

    /// Returns the text content of the last command's output (requires shell integration).
    /// This finds the most recent Output zone that comes after a Prompt/Input zone.
    pub fn last_command_output(&self) -> Option<String> {
        let mut term = self.term.lock().ok()?;
        let zones = term.get_semantic_zones().ok()?;
        let (_term_rows, cols) = *self.size.lock().ok()?;
        let cols_usize = cols as usize;

        // Find the last Output zone that follows a Prompt or Input zone
        // We look for the pattern: Prompt -> Input -> Output
        let mut last_output_zone: Option<&tattoy_wezterm_term::SemanticZone> = None;

        for zone in zones.iter().rev() {
            if zone.semantic_type == tattoy_wezterm_term::SemanticType::Output {
                // Check if there's a prompt/input before this output
                let has_prompt_before = zones.iter().any(|z| {
                    (z.semantic_type == tattoy_wezterm_term::SemanticType::Prompt
                        || z.semantic_type == tattoy_wezterm_term::SemanticType::Input)
                        && z.end_y < zone.start_y
                });
                if has_prompt_before {
                    last_output_zone = Some(zone);
                    break;
                }
            }
        }

        let output_zone = last_output_zone?;
        let start_y = output_zone.start_y as usize;
        let end_y = output_zone.end_y as usize;

        let screen = term.screen();
        let mut lines = Vec::new();

        screen.for_each_phys_line(|idx, line| {
            if idx >= start_y && idx <= end_y {
                let mut line_text = String::new();
                for col in 0..cols_usize {
                    if let Some(cell) = line.get_cell(col) {
                        let grapheme = cell.str();
                        line_text.push_str(grapheme);
                    } else {
                        line_text.push(' ');
                    }
                }
                // Trim trailing whitespace from each line
                lines.push(line_text.trim_end().to_string());
            }
        });

        if lines.is_empty() {
            return None;
        }

        // Join lines and trim trailing empty lines
        let mut text = lines.join("\n");
        while text.ends_with('\n') {
            text.pop();
        }

        Some(text)
    }

    /// Returns a list of all prompt line indices (physical row indices).
    /// Used for jumping between prompts.
    pub fn prompt_positions(&self) -> Vec<usize> {
        let mut term = match self.term.lock() {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };

        let zones = match term.get_semantic_zones() {
            Ok(z) => z,
            Err(_) => return Vec::new(),
        };

        zones
            .iter()
            .filter(|z| z.semantic_type == tattoy_wezterm_term::SemanticType::Prompt)
            .map(|z| z.start_y as usize)
            .collect()
    }

    /// Returns true if a command is currently running (not at a prompt).
    /// This is detected by checking if the last semantic zone is Output,
    /// or if the cursor is past the Input zone.
    pub fn is_command_running(&self) -> bool {
        let mut term = match self.term.lock() {
            Ok(t) => t,
            Err(_) => return false,
        };

        let zones = match term.get_semantic_zones() {
            Ok(z) => z,
            Err(_) => return false,
        };

        // If no zones, we can't determine state (shell integration not active)
        if zones.is_empty() {
            return false;
        }

        // Get the last zone
        let last_zone = match zones.last() {
            Some(z) => z,
            None => return false,
        };

        // If last zone is Output, a command is running
        if last_zone.semantic_type == tattoy_wezterm_term::SemanticType::Output {
            return true;
        }

        // If last zone is Input and we've moved past it (command submitted but no output yet)
        // Check if cursor is past the input zone
        if last_zone.semantic_type == tattoy_wezterm_term::SemanticType::Input {
            let cursor = term.cursor_pos();
            let screen = term.screen();
            let (term_rows, _) = match self.size.lock() {
                Ok(s) => *s,
                Err(_) => return false,
            };
            let total_lines = screen.scrollback_rows();
            let visible_start = total_lines.saturating_sub(term_rows as usize);
            let cursor_phys_y = visible_start + cursor.y as usize;

            // If cursor is past the end of the input zone, command was submitted
            if cursor_phys_y > last_zone.end_y as usize {
                return true;
            }
        }

        false
    }
}
