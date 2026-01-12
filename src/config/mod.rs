//! Runtime configuration for the terminal emulator.
//!
//! Configuration is loaded from `~/.config/term/config.toml` if it exists,
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
    pub font: FontConfig,
    pub keybinds: KeybindConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            window: WindowConfig::default(),
            colors: ColorConfig::default(),
            terminal: TerminalConfig::default(),
            font: FontConfig::default(),
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

        // Try to read existing config
        if let Ok(contents) = fs::read_to_string(&path) {
            if let Ok(mut config) = toml::from_str::<Config>(&contents) {
                config.apply_defaults();
                return config;
            }
            // Config exists but is invalid - don't overwrite, just use defaults
            eprintln!(
                "Warning: invalid config at {}, using defaults",
                path.display()
            );
            return Config::default();
        }

        // Config doesn't exist - create default
        let config = Config::default();
        if let Err(e) = config.save() {
            eprintln!("Warning: could not write default config: {e}");
        }
        config
    }

    /// Returns the configuration file path.
    pub fn config_path() -> Option<PathBuf> {
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
            title: "term".to_string(),
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
