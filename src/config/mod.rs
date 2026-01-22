//! Runtime configuration for the terminal emulator.
//!
//! Configuration is loaded from `~/.config/yatmux/config.toml` if it exists,
//! otherwise defaults are used.

mod action;
mod builtin_themes;
mod colors;
mod config;
mod experimental;
mod font;
mod interaction;
mod keybind;
mod load;
mod merge;
mod pane;
mod plugins;
mod profile;
mod serde_hex;
mod shell_integration;
mod terminal;
mod theme;
mod ui;
mod window;

pub use action::Action;
pub use colors::ColorConfig;
pub use config::Config;
pub use experimental::{ExperimentalConfig, FontScaleClampConfig};
pub use font::{FontConfig, FontSlant, FontWeight};
pub use interaction::InteractionConfig;
pub use keybind::{Keybind, KeybindAction, KeybindConfig, PluginKeybind};
pub use pane::PaneConfig;
pub use plugins::PluginConfig;
pub use profile::{ProfileDefinition, ProfilesConfig};
pub use shell_integration::{ShadowPromptMode, ShellIntegrationConfig, TabTitleSource};
pub use terminal::TerminalConfig;
pub use theme::ThemeConfig;
pub use ui::{
    UiConfig, UiContextMenuConfig, UiDividerConfig, UiHelpConfig, UiSearchConfig,
    UiShadowPromptConfig, UiStickyPromptConfig, UiTabBarConfig, UiToastConfig,
};
pub use window::WindowConfig;

use serde::{Deserialize, Serialize};

use crate::constants::{
    DEFAULT_BG_COLOR, DEFAULT_COLS, DEFAULT_FG_COLOR, DEFAULT_ROWS, SCROLL_SPEED_MULTIPLIER,
    SCROLLBACK_CAPACITY, TAB_STOP_WIDTH,
};

// Make serde helper modules available for `#[serde(with = "...")]` in submodules.
use serde_hex::{hex_color, hex_palette_opt};
