use serde::{Deserialize, Serialize};

mod divider;
mod help;
mod prompt;
mod search;
mod tab_bar;
mod toast;

pub use divider::UiDividerConfig;
pub use help::UiHelpConfig;
pub use prompt::{UiContextMenuConfig, UiShadowPromptConfig, UiStickyPromptConfig};
pub use search::UiSearchConfig;
pub use tab_bar::UiTabBarConfig;
pub use toast::UiToastConfig;

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
