use serde::{Deserialize, Serialize};

use super::super::serde_hex::hex_color_opt;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiTabBarConfig {
    pub gap_px: usize,
    pub side_padding_px: usize,
    pub max_width_cells: usize,
    pub max_width_px_extra: usize,
    pub vertical_padding_px: usize,
    pub tab_internal_padding_px: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_height_cells: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
    pub border: Option<u32>,
    #[serde(
        with = "hex_color_opt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub inactive_tab_background: Option<u32>,
    #[serde(
        with = "hex_color_opt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub inactive_text: Option<u32>,
}

impl Default for UiTabBarConfig {
    fn default() -> Self {
        Self {
            gap_px: 4,
            side_padding_px: 8,
            max_width_cells: 12,
            max_width_px_extra: 16,
            vertical_padding_px: 8,
            tab_internal_padding_px: 16,
            min_height_cells: None,
            font_scale: None,
            background: None,
            border: None,
            inactive_tab_background: None,
            inactive_text: None,
        }
    }
}
