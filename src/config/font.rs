use super::*;

/// Font configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FontConfig {
    /// Font scale multiplier (1 = 8x8, 2 = 16x16, etc.).
    pub scale: usize,
}

impl Default for FontConfig {
    fn default() -> Self {
        FontConfig { scale: FONT_SCALE }
    }
}
