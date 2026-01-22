use serde::{Deserialize, Serialize};

use super::super::serde_hex::hex_color_opt;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiHelpConfig {
    pub padding_x_cells: usize,
    pub padding_y_cells: usize,
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
    pub footer_text: Option<u32>,
}

impl Default for UiHelpConfig {
    fn default() -> Self {
        Self {
            padding_x_cells: 2,
            padding_y_cells: 0,
            font_scale: None,
            background: None,
            text: None,
            footer_text: None,
        }
    }
}
