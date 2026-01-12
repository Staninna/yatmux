//! Runtime configuration for the terminal emulator.
//!
//! Configuration is loaded from `~/.config/yatmux/config.toml` if it exists,
//! otherwise defaults are used.

mod action;
mod keybind;

pub use action::Action;
pub use keybind::{Keybind, KeybindConfig};

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::constants::{
    DEFAULT_BG_COLOR, DEFAULT_COLS, DEFAULT_FG_COLOR, DEFAULT_ROWS, FONT_SCALE,
    SCROLL_SPEED_MULTIPLIER, SCROLLBACK_CAPACITY, TAB_STOP_WIDTH,
};

/// Serde module for serializing colors as hex strings like "#RRGGBB".
mod hex_color {
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(color: &u32, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("#{:06X}", color))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u32, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        // Accept either a string ("#RRGGBB", "0xRRGGBB") or an integer
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ColorValue {
            String(String),
            Int(u32),
        }

        match ColorValue::deserialize(deserializer)? {
            ColorValue::Int(n) => Ok(n),
            ColorValue::String(s) => {
                parse_color(&s).ok_or_else(|| Error::custom("invalid color format"))
            }
        }
    }

    fn parse_color(s: &str) -> Option<u32> {
        let s = s.trim().trim_start_matches('#').trim_start_matches("0x");
        u32::from_str_radix(s, 16).ok()
    }
}

/// Main configuration structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub window: WindowConfig,
    pub colors: ColorConfig,
    pub terminal: TerminalConfig,
    pub shell_integration: ShellIntegrationConfig,
    pub font: FontConfig,
    pub pane: PaneConfig,
    pub keybinds: KeybindConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            window: WindowConfig::default(),
            colors: ColorConfig::default(),
            terminal: TerminalConfig::default(),
            shell_integration: ShellIntegrationConfig::default(),
            font: FontConfig::default(),
            pane: PaneConfig::default(),
            keybinds: KeybindConfig::default(),
        }
    }
}

impl Config {
    /// Applies any missing defaults and clamps values to sane ranges.
    ///
    /// Note: `serde(default)` already fills in most missing fields, but
    /// collection-like configs (like `keybinds`) need explicit merging to pick
    /// up newly added defaults without overwriting user customizations.
    pub fn apply_defaults(&mut self) {
        self.keybinds.apply_defaults();

        // Keep rendering/input assumptions intact.
        self.font.scale = self.font.scale.clamp(1, 8);

        self.terminal.rows = self.terminal.rows.max(1);
        self.terminal.cols = self.terminal.cols.max(1);
        self.terminal.scrollback_lines = self.terminal.scrollback_lines.max(1);
        self.terminal.tab_width = self.terminal.tab_width.max(1);

        if !self.terminal.scroll_speed.is_finite() || self.terminal.scroll_speed <= 0.0 {
            self.terminal.scroll_speed = SCROLL_SPEED_MULTIPLIER;
        }
    }

    /// Loads configuration from the default path.
    /// If config file doesn't exist, writes defaults and returns them.
    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Config::default();
        };

        // Try to read existing config (new path).
        if let Ok(contents) = fs::read_to_string(&path) {
            if let Ok(mut config) = toml::from_str::<Config>(&contents) {
                config.apply_defaults();
                return config;
            }
            // Config exists but is invalid - don't overwrite, just use defaults.
            eprintln!(
                "Warning: invalid config at {}, using defaults",
                path.display()
            );
            return Config::default();
        }

        // Back-compat: if the old `term` config exists, load it.
        if let Some(old_path) = Self::legacy_config_path() {
            if let Ok(contents) = fs::read_to_string(&old_path) {
                if let Ok(mut config) = toml::from_str::<Config>(&contents) {
                    config.apply_defaults();
                    // Best-effort: write the migrated config to the new location.
                    let _ = config.save();
                    return config;
                }
            }
        }

        // Config doesn't exist - create default.
        let config = Config::default();
        if let Err(e) = config.save() {
            eprintln!("Warning: could not write default config: {e}");
        }
        config
    }

    /// Returns the configuration file path.
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("yatmux").join("config.toml"))
    }

    fn legacy_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("term").join("config.toml"))
    }

    /// Saves the configuration to the default path.
    #[allow(dead_code)]
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path().ok_or("Could not determine config directory")?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }
}

/// Window configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowConfig {
    pub title: String,
}

impl Default for WindowConfig {
    fn default() -> Self {
        WindowConfig {
            title: "yatmux".to_string(),
        }
    }
}

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
    #[serde(skip_serializing_if = "Option::is_none")]
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

/// Terminal behavior configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TerminalConfig {
    /// Default number of rows.
    pub rows: u16,
    /// Default number of columns.
    pub cols: u16,
    /// Maximum lines in scrollback buffer.
    pub scrollback_lines: usize,
    /// Lines to scroll per mouse wheel tick.
    pub scroll_speed: f32,
    /// Tab stop width in characters.
    pub tab_width: usize,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        TerminalConfig {
            rows: DEFAULT_ROWS,
            cols: DEFAULT_COLS,
            scrollback_lines: SCROLLBACK_CAPACITY,
            scroll_speed: SCROLL_SPEED_MULTIPLIER,
            tab_width: TAB_STOP_WIDTH,
        }
    }
}

/// Shell integration configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ShellIntegrationConfig {
    /// Track current working directory via OSC 7.
    pub cwd_from_osc7: bool,

    /// Track prompt/input/output boundaries via OSC 133.
    pub semantic_zones_from_osc133: bool,

    /// Track title changes via OSC 0/1/2.
    pub title_from_osc: bool,

    /// Controls what we show in the tab bar.
    pub tab_title_source: TabTitleSource,

    /// Updates the OS window title to match the active tab.
    pub window_title_follows_active_tab: bool,

    /// Show the current prompt at the bottom when scrolled up.
    pub sticky_prompt: bool,

    /// Shadow prompt mode - type-ahead during command execution.
    pub shadow_prompt: ShadowPromptMode,

    /// Prints debug logs when shell integration signals change.
    pub debug_log: bool,
}

impl Default for ShellIntegrationConfig {
    fn default() -> Self {
        Self {
            cwd_from_osc7: true,
            semantic_zones_from_osc133: true,
            title_from_osc: true,
            tab_title_source: TabTitleSource::Cwd,
            window_title_follows_active_tab: true,
            sticky_prompt: true,
            shadow_prompt: ShadowPromptMode::default(),
            debug_log: false,
        }
    }
}

/// Source for tab bar titles.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TabTitleSource {
    None,
    Cwd,
    Title,
}

/// Shadow prompt mode - when to show type-ahead prompt during command execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ShadowPromptMode {
    /// Never show shadow prompt
    Off,
    /// Show shadow prompt immediately when command starts
    Always,
    /// Show shadow prompt only when user starts typing (default)
    #[default]
    OnTyping,
}

impl Default for TabTitleSource {
    fn default() -> Self {
        Self::Cwd
    }
}

/// Font configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FontConfig {
    /// Font scale multiplier (1 = 8x8, 2 = 16x16, etc.).
    pub scale: usize,
}

impl Default for FontConfig {
    fn default() -> Self {
        FontConfig { scale: FONT_SCALE }
    }
}

/// Pane configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PaneConfig {
    /// Padding in pixels for all sides (overridden by specific side settings).
    #[serde(default)]
    padding: Option<usize>,
    /// Padding in pixels on the left side of each pane.
    #[serde(default)]
    padding_left: Option<usize>,
    /// Padding in pixels on the right side of each pane.
    #[serde(default)]
    padding_right: Option<usize>,
    /// Padding in pixels on the top of each pane.
    #[serde(default)]
    padding_top: Option<usize>,
    /// Padding in pixels on the bottom of each pane.
    #[serde(default)]
    padding_bottom: Option<usize>,
    /// Minimum pane size in pixels (prevents splitting below this size).
    #[serde(default)]
    pub min_size: Option<usize>,
}

const DEFAULT_PANE_PADDING: usize = 8;
const DEFAULT_MIN_PANE_SIZE: usize = 100;

impl Default for PaneConfig {
    fn default() -> Self {
        PaneConfig {
            padding: None,
            padding_left: None,
            padding_right: None,
            padding_top: None,
            padding_bottom: None,
            min_size: None,
        }
    }
}

impl PaneConfig {
    /// Returns the effective left padding.
    pub fn padding_left(&self) -> usize {
        self.padding_left
            .or(self.padding)
            .unwrap_or(DEFAULT_PANE_PADDING)
    }

    /// Returns the effective right padding.
    pub fn padding_right(&self) -> usize {
        self.padding_right
            .or(self.padding)
            .unwrap_or(DEFAULT_PANE_PADDING)
    }

    /// Returns the effective top padding.
    pub fn padding_top(&self) -> usize {
        self.padding_top
            .or(self.padding)
            .unwrap_or(DEFAULT_PANE_PADDING)
    }

    /// Returns the effective bottom padding.
    pub fn padding_bottom(&self) -> usize {
        self.padding_bottom
            .or(self.padding)
            .unwrap_or(DEFAULT_PANE_PADDING)
    }

    /// Returns the minimum pane size in pixels.
    pub fn min_size(&self) -> usize {
        self.min_size.unwrap_or(DEFAULT_MIN_PANE_SIZE)
    }
}
