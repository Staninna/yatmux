use serde::{Deserialize, Serialize};

use super::super::serde_hex::hex_color_opt;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiToastConfig {
    pub duration_ms: u64,
    pub font_scale: Option<f32>,
    #[serde(
        with = "hex_color_opt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub background: Option<u32>,
    #[serde(
        with = "hex_color_opt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub text: Option<u32>,
    #[serde(
        with = "hex_color_opt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub border: Option<u32>,
    pub bottom_margin_cells: usize,
}

impl Default for UiToastConfig {
    fn default() -> Self {
        Self {
            duration_ms: 1500,
            font_scale: None,
            background: None,
            text: None,
            border: None,
            bottom_margin_cells: 2,
        }
    }
}
