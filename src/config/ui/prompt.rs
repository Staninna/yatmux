use serde::{Deserialize, Serialize};

use super::super::serde_hex::hex_color_opt;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiStickyPromptConfig {
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
    pub separator: Option<u32>,
}

impl Default for UiStickyPromptConfig {
    fn default() -> Self {
        Self {
            background: None,
            separator: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiContextMenuConfig {
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
    pub hover_background: Option<u32>,
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
}

impl Default for UiContextMenuConfig {
    fn default() -> Self {
        Self {
            background: None,
            hover_background: None,
            text: None,
            border: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiShadowPromptConfig {
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
    pub cursor: Option<u32>,
    #[serde(
        with = "hex_color_opt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub prompt_indicator: Option<u32>,
    #[serde(
        with = "hex_color_opt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub border: Option<u32>,
}

impl Default for UiShadowPromptConfig {
    fn default() -> Self {
        Self {
            background: None,
            text: None,
            cursor: None,
            prompt_indicator: None,
            border: None,
        }
    }
}
