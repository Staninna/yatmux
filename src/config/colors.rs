use super::*;

/// Color configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ColorConfig {
    /// Background color in hex format (#RRGGBB).
    #[serde(with = "hex_color")]
    pub background: u32,
    /// Foreground color in hex format (#RRGGBB).
    #[serde(with = "hex_color")]
    pub foreground: u32,
    /// Accent color used for UI highlights (focused pane, help border).
    #[serde(with = "hex_color")]
    pub accent: u32,
    /// Optional custom 16-color palette (ANSI colors 0-15).
    #[serde(
        with = "hex_palette_opt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub palette: Option<[u32; 16]>,
}

impl Default for ColorConfig {
    fn default() -> Self {
        ColorConfig {
            background: DEFAULT_BG_COLOR,
            foreground: DEFAULT_FG_COLOR,
            accent: 0x66AAFF,
            palette: None,
        }
    }
}
