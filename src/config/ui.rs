use super::*;

/// UI chrome configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub tab_bar: UiTabBarConfig,
    pub search: UiSearchConfig,
    pub toast: UiToastConfig,
    pub help: UiHelpConfig,
    pub sticky_prompt: UiStickyPromptConfig,
    pub context_menu: UiContextMenuConfig,
    pub shadow_prompt: UiShadowPromptConfig,
    pub dividers: UiDividerConfig,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            tab_bar: UiTabBarConfig::default(),
            search: UiSearchConfig::default(),
            toast: UiToastConfig::default(),
            help: UiHelpConfig::default(),
            sticky_prompt: UiStickyPromptConfig::default(),
            context_menu: UiContextMenuConfig::default(),
            shadow_prompt: UiShadowPromptConfig::default(),
            dividers: UiDividerConfig::default(),
        }
    }
}

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
    pub font_scale: Option<usize>,

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiToastConfig {
    pub duration_ms: u64,
    pub font_scale: Option<usize>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiHelpConfig {
    pub padding_x_cells: usize,
    pub padding_y_cells: usize,
    pub font_scale: Option<usize>,
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
            padding_y_cells: 1,
            font_scale: None,
            background: None,
            text: None,
            footer_text: None,
        }
    }
}

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
