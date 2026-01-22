use serde::{Deserialize, Serialize};

use super::super::serde_hex::hex_color_opt;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiSearchConfig {
    pub right_reserved_px: usize,
    #[serde(
        with = "hex_color_opt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub match_bg: Option<u32>,
    #[serde(
        with = "hex_color_opt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub current_match_bg: Option<u32>,
    #[serde(
        with = "hex_color_opt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub bar_bg: Option<u32>,
    #[serde(
        with = "hex_color_opt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub bar_text: Option<u32>,
    #[serde(
        with = "hex_color_opt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub bar_hint_text: Option<u32>,
    #[serde(
        with = "hex_color_opt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub invalid_regex_text: Option<u32>,
}

impl Default for UiSearchConfig {
    fn default() -> Self {
        Self {
            right_reserved_px: 100,
            match_bg: None,
            current_match_bg: None,
            bar_bg: None,
            bar_text: None,
            bar_hint_text: None,
            invalid_regex_text: None,
        }
    }
}
