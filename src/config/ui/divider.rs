use serde::{Deserialize, Serialize};

use super::super::serde_hex::hex_color_opt;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiDividerConfig {
    #[serde(
        with = "hex_color_opt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub color: Option<u32>,
}

impl Default for UiDividerConfig {
    fn default() -> Self {
        Self { color: None }
    }
}
