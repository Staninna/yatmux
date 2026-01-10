//! Runtime configuration for the terminal emulator.
//!
//! Configuration is loaded from `~/.config/term/config.toml` if it exists,
//! otherwise defaults are used.

use std::collections::HashMap;
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
    /// Loads configuration from the default path.
    /// If config file doesn't exist, writes defaults and returns them.
    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Config::default();
        };

        // Try to read existing config
        if let Ok(contents) = fs::read_to_string(&path) {
            if let Ok(config) = toml::from_str(&contents) {
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
    /// Optional custom 16-color palette (ANSI colors 0-15).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub palette: Option<[u32; 16]>,
}

impl Default for ColorConfig {
    fn default() -> Self {
        ColorConfig {
            background: DEFAULT_BG_COLOR,
            foreground: DEFAULT_FG_COLOR,
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

/// Terminal actions that can be bound to keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Copy selected text to clipboard.
    Copy,
    /// Paste from clipboard.
    Paste,
    /// Scroll up by one page.
    ScrollPageUp,
    /// Scroll down by one page.
    ScrollPageDown,
    /// Scroll up by one line.
    ScrollLineUp,
    /// Scroll down by one line.
    ScrollLineDown,
    /// Scroll to top of scrollback.
    ScrollToTop,
    /// Scroll to bottom (live view).
    ScrollToBottom,
    /// Clear the scrollback buffer.
    ClearScrollback,
    /// Reset the terminal.
    Reset,
    /// Open search mode.
    SearchFind,
    /// Close search mode.
    SearchClose,
    /// Navigate to next search match.
    SearchNext,
    /// Navigate to previous search match.
    SearchPrev,
    /// Toggle search case sensitivity.
    SearchToggleCase,
    /// Confirm search / go to current match.
    SearchConfirm,
}

impl Action {
    /// Returns true if this action only applies in search mode.
    pub fn is_search_mode_only(&self) -> bool {
        matches!(
            self,
            Action::SearchClose
                | Action::SearchNext
                | Action::SearchPrev
                | Action::SearchToggleCase
                | Action::SearchConfirm
        )
    }
}

/// A keybind specification.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Keybind {
    pub key: String,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Keybind {
    /// Parses a keybind string like "ctrl+shift+c" or "f12".
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.to_lowercase();
        let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();

        let mut ctrl = false;
        let mut shift = false;
        let mut alt = false;
        let mut key = None;

        for part in parts {
            match part {
                "ctrl" | "control" => ctrl = true,
                "shift" => shift = true,
                "alt" | "meta" => alt = true,
                k => key = Some(k.to_string()),
            }
        }

        key.map(|k| Keybind {
            key: k,
            ctrl,
            shift,
            alt,
        })
    }

    /// Checks if this keybind matches the given key and modifiers.
    pub fn matches(&self, key: &str, ctrl: bool, shift: bool, alt: bool) -> bool {
        self.key.eq_ignore_ascii_case(key)
            && self.ctrl == ctrl
            && self.shift == shift
            && self.alt == alt
    }
}

/// Keybind configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeybindConfig {
    /// Map of keybind strings to actions.
    #[serde(flatten)]
    pub bindings: HashMap<String, Action>,
}

impl Default for KeybindConfig {
    fn default() -> Self {
        let mut bindings = HashMap::new();

        // General actions
        bindings.insert("ctrl+shift+c".to_string(), Action::Copy);
        bindings.insert("ctrl+shift+v".to_string(), Action::Paste);
        bindings.insert("ctrl+v".to_string(), Action::Paste);
        bindings.insert("shift+insert".to_string(), Action::Paste);
        bindings.insert("shift+pageup".to_string(), Action::ScrollPageUp);
        bindings.insert("shift+pagedown".to_string(), Action::ScrollPageDown);
        bindings.insert("shift+up".to_string(), Action::ScrollLineUp);
        bindings.insert("shift+down".to_string(), Action::ScrollLineDown);
        bindings.insert("ctrl+shift+home".to_string(), Action::ScrollToTop);
        bindings.insert("ctrl+shift+end".to_string(), Action::ScrollToBottom);
        bindings.insert("ctrl+shift+f".to_string(), Action::SearchFind);
        bindings.insert("ctrl+shift+k".to_string(), Action::ClearScrollback);

        // Search mode actions
        bindings.insert("escape".to_string(), Action::SearchClose);
        bindings.insert("enter".to_string(), Action::SearchConfirm);
        bindings.insert("ctrl+n".to_string(), Action::SearchNext);
        bindings.insert("ctrl+p".to_string(), Action::SearchPrev);
        bindings.insert("ctrl+c".to_string(), Action::SearchToggleCase);
        bindings.insert("down".to_string(), Action::SearchNext);
        bindings.insert("up".to_string(), Action::SearchPrev);

        KeybindConfig { bindings }
    }
}

impl KeybindConfig {
    /// Finds the action for a given key and modifiers.
    pub fn get_action(&self, key: &str, ctrl: bool, shift: bool, alt: bool) -> Option<Action> {
        for (bind_str, action) in &self.bindings {
            if let Some(keybind) = Keybind::parse(bind_str) {
                if keybind.matches(key, ctrl, shift, alt) {
                    return Some(*action);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.window.title, "term");
        assert_eq!(config.colors.background, DEFAULT_BG_COLOR);
        assert_eq!(config.terminal.rows, DEFAULT_ROWS);
    }

    #[test]
    fn test_config_parse() {
        let toml = r#"
            [window]
            title = "my-term"

            [colors]
            background = 0x000000
            foreground = 0xFFFFFF

            [terminal]
            scrollback_lines = 10000
        "#;

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.window.title, "my-term");
        assert_eq!(config.colors.background, 0x000000);
        assert_eq!(config.colors.foreground, 0xFFFFFF);
        assert_eq!(config.terminal.scrollback_lines, 10000);
    }

    #[test]
    fn test_config_serialize() {
        let config = Config::default();
        let toml = toml::to_string(&config).unwrap();
        assert!(toml.contains("[window]"));
        assert!(toml.contains("[colors]"));
        // Verify colors are serialized as hex strings
        assert!(toml.contains("background = \"#"));
        assert!(toml.contains("foreground = \"#"));
    }

    #[test]
    fn test_keybind_parse() {
        let kb = Keybind::parse("ctrl+shift+c").unwrap();
        assert_eq!(kb.key, "c");
        assert!(kb.ctrl);
        assert!(kb.shift);
        assert!(!kb.alt);

        let kb = Keybind::parse("f12").unwrap();
        assert_eq!(kb.key, "f12");
        assert!(!kb.ctrl);
        assert!(!kb.shift);
        assert!(!kb.alt);

        let kb = Keybind::parse("alt+enter").unwrap();
        assert_eq!(kb.key, "enter");
        assert!(!kb.ctrl);
        assert!(!kb.shift);
        assert!(kb.alt);
    }

    #[test]
    fn test_keybind_matches() {
        let kb = Keybind::parse("ctrl+shift+c").unwrap();
        assert!(kb.matches("c", true, true, false));
        assert!(!kb.matches("c", true, false, false));
        assert!(!kb.matches("v", true, true, false));
    }

    #[test]
    fn test_keybind_config_get_action() {
        let config = KeybindConfig::default();
        assert_eq!(
            config.get_action("c", true, true, false),
            Some(Action::Copy)
        );
        assert_eq!(
            config.get_action("v", true, false, false),
            Some(Action::Paste)
        );
        assert_eq!(
            config.get_action("pageup", false, true, false),
            Some(Action::ScrollPageUp)
        );
        assert_eq!(config.get_action("x", true, true, false), None);
    }

    #[test]
    fn test_keybind_config_parse() {
        let toml = r#"
            [keybinds]
            "ctrl+c" = "copy"
            "ctrl+v" = "paste"
            "f1" = "scroll_to_top"
        "#;

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(
            config.keybinds.get_action("c", true, false, false),
            Some(Action::Copy)
        );
        assert_eq!(
            config.keybinds.get_action("f1", false, false, false),
            Some(Action::ScrollToTop)
        );
    }

    #[test]
    fn test_search_keybinds() {
        let config = KeybindConfig::default();

        // Search mode keybinds
        assert_eq!(
            config.get_action("escape", false, false, false),
            Some(Action::SearchClose)
        );
        assert_eq!(
            config.get_action("enter", false, false, false),
            Some(Action::SearchConfirm)
        );
        assert_eq!(
            config.get_action("n", true, false, false),
            Some(Action::SearchNext)
        );
        assert_eq!(
            config.get_action("p", true, false, false),
            Some(Action::SearchPrev)
        );
        assert_eq!(
            config.get_action("c", true, false, false),
            Some(Action::SearchToggleCase)
        );
    }

    #[test]
    fn test_search_keybinds_configurable() {
        let toml = r#"
            [keybinds]
            "ctrl+g" = "search_close"
            "ctrl+j" = "search_next"
            "ctrl+k" = "search_prev"
        "#;

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(
            config.keybinds.get_action("g", true, false, false),
            Some(Action::SearchClose)
        );
        assert_eq!(
            config.keybinds.get_action("j", true, false, false),
            Some(Action::SearchNext)
        );
        assert_eq!(
            config.keybinds.get_action("k", true, false, false),
            Some(Action::SearchPrev)
        );
    }

    #[test]
    fn test_search_arrow_keybinds() {
        let config = KeybindConfig::default();

        // Arrow keys should work for search navigation
        assert_eq!(
            config.get_action("down", false, false, false),
            Some(Action::SearchNext)
        );
        assert_eq!(
            config.get_action("up", false, false, false),
            Some(Action::SearchPrev)
        );
    }
}
