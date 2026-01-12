use super::*;

/// Terminal behavior configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TerminalConfig {
    /// Default number of rows.
    pub rows: u16,
    /// Default number of columns.
    pub cols: u16,
    /// Maximum lines in scrollback buffer.
    pub scrollback_lines: usize,
    /// Lines to scroll per mouse wheel tick.
    pub scroll_speed: f32,
    /// Tab stop width in characters.
    pub tab_width: usize,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        TerminalConfig {
            rows: DEFAULT_ROWS,
            cols: DEFAULT_COLS,
            scrollback_lines: SCROLLBACK_CAPACITY,
            scroll_speed: SCROLL_SPEED_MULTIPLIER,
            tab_width: TAB_STOP_WIDTH,
        }
    }
}
