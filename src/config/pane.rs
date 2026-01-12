use super::*;

/// Pane configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PaneConfig {
    /// Padding in pixels for all sides (overridden by specific side settings).
    #[serde(default)]
    padding: Option<usize>,
    /// Padding in pixels on the left side of each pane.
    #[serde(default)]
    padding_left: Option<usize>,
    /// Padding in pixels on the right side of each pane.
    #[serde(default)]
    padding_right: Option<usize>,
    /// Padding in pixels on the top of each pane.
    #[serde(default)]
    padding_top: Option<usize>,
    /// Padding in pixels on the bottom of each pane.
    #[serde(default)]
    padding_bottom: Option<usize>,
    /// Minimum pane size in pixels (prevents splitting below this size).
    #[serde(default)]
    pub min_size: Option<usize>,
}

const DEFAULT_PANE_PADDING: usize = 8;
const DEFAULT_MIN_PANE_SIZE: usize = 100;

impl Default for PaneConfig {
    fn default() -> Self {
        PaneConfig {
            padding: None,
            padding_left: None,
            padding_right: None,
            padding_top: None,
            padding_bottom: None,
            min_size: None,
        }
    }
}

impl PaneConfig {
    /// Returns the effective left padding.
    pub fn padding_left(&self) -> usize {
        self.padding_left
            .or(self.padding)
            .unwrap_or(DEFAULT_PANE_PADDING)
    }

    /// Returns the effective right padding.
    pub fn padding_right(&self) -> usize {
        self.padding_right
            .or(self.padding)
            .unwrap_or(DEFAULT_PANE_PADDING)
    }

    /// Returns the effective top padding.
    pub fn padding_top(&self) -> usize {
        self.padding_top
            .or(self.padding)
            .unwrap_or(DEFAULT_PANE_PADDING)
    }

    /// Returns the effective bottom padding.
    pub fn padding_bottom(&self) -> usize {
        self.padding_bottom
            .or(self.padding)
            .unwrap_or(DEFAULT_PANE_PADDING)
    }

    /// Returns the minimum pane size in pixels.
    pub fn min_size(&self) -> usize {
        self.min_size.unwrap_or(DEFAULT_MIN_PANE_SIZE)
    }
}
