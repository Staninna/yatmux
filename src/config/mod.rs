//! Runtime configuration for the terminal emulator.
//!
//! Configuration is loaded from `~/.config/yatmux/config.toml` if it exists,
//! otherwise defaults are used.

mod action;
mod keybind;

pub use action::Action;
pub use keybind::{Keybind, KeybindConfig};

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

mod builtin_themes {
    include!(concat!(env!("OUT_DIR"), "/builtin_themes.rs"));
}

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
        let s = s.trim();
        let s = s.trim_start_matches('#').trim_start_matches("0x");

        // #RGB shorthand -> #RRGGBB
        if s.len() == 3 {
            let mut expanded = String::with_capacity(6);
            for ch in s.chars() {
                expanded.push(ch);
                expanded.push(ch);
            }
            return u32::from_str_radix(&expanded, 16).ok();
        }

        u32::from_str_radix(s, 16).ok()
    }
}

/// Serde module for serializing optional colors.
mod hex_color_opt {
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(color: &Option<u32>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match color {
            Some(c) => super::hex_color::serialize(c, serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Some(super::hex_color::deserialize(deserializer)?))
    }
}

/// Serde module for deserializing optional 16-color palettes.
mod hex_palette_opt {
    use serde::{Deserialize, Deserializer, Serializer};

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ColorValue {
        String(String),
        Int(u32),
    }

    fn parse_color(s: &str) -> Option<u32> {
        let s = s.trim();
        let s = s.trim_start_matches('#').trim_start_matches("0x");

        if s.len() == 3 {
            let mut expanded = String::with_capacity(6);
            for ch in s.chars() {
                expanded.push(ch);
                expanded.push(ch);
            }
            return u32::from_str_radix(&expanded, 16).ok();
        }

        u32::from_str_radix(s, 16).ok()
    }

    pub fn serialize<S>(palette: &Option<[u32; 16]>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match palette {
            None => serializer.serialize_none(),
            Some(p) => {
                let items: Vec<String> = p.iter().map(|c| format!("#{:06X}", c)).collect();
                serializer.serialize_some(&items)
            }
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<[u32; 16]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        let opt = Option::<Vec<ColorValue>>::deserialize(deserializer)?;
        let Some(values) = opt else {
            return Ok(None);
        };

        if values.len() != 16 {
            return Err(Error::custom("palette must have exactly 16 colors"));
        }

        let mut out = [0u32; 16];
        for (i, v) in values.into_iter().enumerate() {
            out[i] = match v {
                ColorValue::Int(n) => n,
                ColorValue::String(s) => {
                    parse_color(&s).ok_or_else(|| Error::custom("invalid color format"))?
                }
            };
        }

        Ok(Some(out))
    }
}

/// Theme selection and imports.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    /// Optional theme name.
    pub name: Option<String>,
    /// Additional config files to merge in (relative to `config.toml`).
    pub imports: Vec<String>,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: Some("dracula".to_string()),
            imports: Vec::new(),
        }
    }
}

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

/// Interaction tuning.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct InteractionConfig {
    pub click_move_max_steps: usize,
    pub pane_resize_step: f32,
    pub focus_move_overlap_weight: i64,
}

impl Default for InteractionConfig {
    fn default() -> Self {
        Self {
            click_move_max_steps: 512,
            pane_resize_step: 0.05,
            focus_move_overlap_weight: 1000,
        }
    }
}

/// Main configuration structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: ThemeConfig,

    pub window: WindowConfig,
    pub colors: ColorConfig,
    pub terminal: TerminalConfig,
    pub shell_integration: ShellIntegrationConfig,
    pub font: FontConfig,
    pub pane: PaneConfig,
    pub keybinds: KeybindConfig,

    pub ui: UiConfig,
    pub interaction: InteractionConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            theme: ThemeConfig::default(),
            window: WindowConfig::default(),
            colors: ColorConfig::default(),
            terminal: TerminalConfig::default(),
            shell_integration: ShellIntegrationConfig::default(),
            font: FontConfig::default(),
            pane: PaneConfig::default(),
            keybinds: KeybindConfig::default(),
            ui: UiConfig::default(),
            interaction: InteractionConfig::default(),
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

        // UI safety clamps.
        self.ui.toast.duration_ms = self.ui.toast.duration_ms.min(60_000);
        self.ui.search.right_reserved_px = self.ui.search.right_reserved_px.min(2_000);
        self.ui.tab_bar.gap_px = self.ui.tab_bar.gap_px.min(128);
        self.ui.tab_bar.side_padding_px = self.ui.tab_bar.side_padding_px.min(256);
        self.ui.tab_bar.max_width_cells = self.ui.tab_bar.max_width_cells.clamp(4, 200);
        self.ui.tab_bar.max_width_px_extra = self.ui.tab_bar.max_width_px_extra.min(512);

        self.interaction.click_move_max_steps =
            self.interaction.click_move_max_steps.clamp(1, 10_000);
        if !self.interaction.pane_resize_step.is_finite()
            || self.interaction.pane_resize_step <= 0.0
        {
            self.interaction.pane_resize_step = InteractionConfig::default().pane_resize_step;
        }
        self.interaction.pane_resize_step = self.interaction.pane_resize_step.clamp(0.005, 0.5);
        self.interaction.focus_move_overlap_weight = self
            .interaction
            .focus_move_overlap_weight
            .clamp(1, 1_000_000);
    }

    /// Loads configuration from the default path.
    ///
    /// Config precedence for native UI colors: theme > [ui] > [colors]
    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Config::default();
        };

        // If config is missing, write a commented template.
        // Then continue loading using that template so themes apply on first boot.
        let contents = if path.exists() {
            match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Warning: could not read config at {}: {e}", path.display());
                    return Config::default();
                }
            }
        } else {
            if let Err(e) = Self::write_default_template(&path) {
                eprintln!("Warning: could not write default config: {e}");
            }
            Self::default_config_template()
        };

        let root_value = match contents.parse::<toml::Value>() {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Warning: invalid config at {}: {e}", path.display());
                return Config::default();
            }
        };

        let base_dir = path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        // Load theme/import settings (best-effort).
        let mut theme_name: Option<String> = None;
        let mut imports: Vec<String> = Vec::new();

        if let Some(theme_table) = root_value.get("theme").and_then(|v| v.as_table()) {
            theme_name = theme_table
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .and_then(|s| {
                    if s.is_empty() {
                        None
                    } else if matches!(s.as_str(), "none" | "off" | "disabled") {
                        None
                    } else {
                        Some(s)
                    }
                });

            if let Some(arr) = theme_table.get("imports").and_then(|v| v.as_array()) {
                for v in arr {
                    if let Some(s) = v.as_str() {
                        imports.push(s.to_string());
                    }
                }
            }
        }

        // Default theme if not specified in config.
        if theme_name.is_none() {
            theme_name = Config::default().theme.name.clone();
        }

        // Load theme TOML (best-effort).
        let mut theme_value: Option<toml::Value> = None;
        if let Some(name) = theme_name.as_deref() {
            if let Some(toml_text) = Self::builtin_theme_toml(name) {
                match toml_text.parse::<toml::Value>() {
                    Ok(v) => theme_value = Some(v),
                    Err(e) => eprintln!("Warning: invalid built-in theme '{name}': {e}"),
                }
            } else if let Some(theme_path) = Self::theme_path(name) {
                match fs::read_to_string(&theme_path) {
                    Ok(toml_text) => match toml_text.parse::<toml::Value>() {
                        Ok(v) => theme_value = Some(v),
                        Err(e) => eprintln!(
                            "Warning: invalid theme TOML at {}: {e}",
                            theme_path.display()
                        ),
                    },
                    Err(e) => eprintln!(
                        "Warning: could not read theme {}: {e}",
                        theme_path.display()
                    ),
                }
            }
        }

        let mut merged = toml::Value::Table(toml::map::Map::new());

        // 1) Optional imports.
        for import in imports {
            let import_path = Self::resolve_import_path(&base_dir, &import);
            match fs::read_to_string(&import_path) {
                Ok(toml_text) => match toml_text.parse::<toml::Value>() {
                    Ok(v) => Self::deep_merge(&mut merged, v),
                    Err(e) => eprintln!(
                        "Warning: invalid import TOML at {}: {e}",
                        import_path.display()
                    ),
                },
                Err(e) => eprintln!(
                    "Warning: could not read import {}: {e}",
                    import_path.display()
                ),
            }
        }

        // 2) Main config file wins for most settings.
        Self::deep_merge(&mut merged, root_value);

        // 3) Theme overrides for native UI colors.
        if let Some(theme_value) = theme_value.as_ref() {
            Self::apply_theme_overrides(&mut merged, theme_value);
        }

        match merged.try_into::<Config>() {
            Ok(mut config) => {
                config.apply_defaults();
                config
            }
            Err(e) => {
                eprintln!("Warning: could not deserialize merged config: {e}");
                Config::default()
            }
        }
    }

    /// Returns the configuration file path.
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("yatmux").join("config.toml"))
    }

    #[allow(dead_code)]
    fn legacy_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("term").join("config.toml"))
    }

    fn builtin_theme_toml(name: &str) -> Option<&'static str> {
        builtin_themes::BUILTIN_THEMES
            .iter()
            .find_map(|(n, toml)| (*n == name).then_some(*toml))
    }

    fn builtin_theme_names() -> &'static [&'static str] {
        builtin_themes::BUILTIN_THEME_NAMES
    }

    fn theme_path(name: &str) -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("yatmux").join("themes").join(format!("{name}.toml")))
    }

    fn resolve_import_path(base_dir: &Path, input: &str) -> PathBuf {
        let s = input.trim();

        // ~ or ~/path
        if s == "~" || s.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                if s == "~" {
                    return home;
                }
                return home.join(&s[2..]);
            }
        }

        let p = PathBuf::from(s);
        if p.is_absolute() { p } else { base_dir.join(p) }
    }

    fn write_default_template(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, Self::default_config_template())?;
        Ok(())
    }

    fn deep_merge(dst: &mut toml::Value, src: toml::Value) {
        match (dst, src) {
            (toml::Value::Table(dst_table), toml::Value::Table(src_table)) => {
                for (k, v) in src_table {
                    match dst_table.get_mut(&k) {
                        Some(existing) => Self::deep_merge(existing, v),
                        None => {
                            dst_table.insert(k, v);
                        }
                    }
                }
            }
            (dst_slot, src_value) => {
                *dst_slot = src_value;
            }
        }
    }

    fn apply_theme_overrides(merged: &mut toml::Value, theme: &toml::Value) {
        let Some(theme_table) = theme.as_table() else {
            return;
        };

        // Precedence for native UI colors: theme > [ui] > [colors]
        for key in ["ui", "colors"] {
            let Some(theme_part) = theme_table.get(key) else {
                continue;
            };

            let mut slot = merged
                .get(key)
                .cloned()
                .unwrap_or_else(|| toml::Value::Table(toml::map::Map::new()));

            Self::deep_merge(&mut slot, theme_part.clone());

            if let Some(dst_table) = merged.as_table_mut() {
                dst_table.insert(key.to_string(), slot);
            }
        }
    }

    fn join_theme_names() -> String {
        let mut s = String::new();
        for (i, name) in Self::builtin_theme_names().iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            if *name == "dracula" {
                s.push_str("dracula (default)");
            } else {
                s.push_str(name);
            }
        }
        s
    }

    fn default_config_template() -> String {
        let mut out = String::new();

        out.push_str("# yatmux configuration\n");
        out.push_str("#\n");
        out.push_str("# Location: ~/.config/yatmux/config.toml\n");
        out.push_str("#\n");
        out.push_str("# Apply changes by restarting yatmux or using Reload config (default: ctrl+shift+r).\n");
        out.push_str("#\n");
        out.push_str("# UI color precedence: theme > [ui] > [colors]\n");
        out.push_str("#\n");
        out.push_str("# Color formats:\n");
        out.push_str("# - \"#RGB\" / \"#RRGGBB\" strings\n");
        out.push_str("# - 0xRRGGBB integers\n");
        out.push_str("#\n");
        out.push_str("# Built-in themes: ");
        out.push_str(&Self::join_theme_names());
        out.push_str("\n\n");

        out.push_str("[theme]\n");
        out.push_str("# Pick a theme by name.\n");
        out.push_str("#\n");
        out.push_str("# Examples:\n");
        out.push_str("# name = \"dracula\"\n");
        out.push_str("# name = \"light\"\n");
        out.push_str("# name = \"gruvbox-dark\"\n");
        out.push_str("#\n");
        out.push_str("# Set to \"\" (empty) or \"off\" to disable theme loading.\n");
        out.push_str("name = \"dracula\"\n");
        out.push_str("# Merge extra files before this config.toml.\n");
        out.push_str(
            "# imports = [\"./local-overrides.toml\", \"~/dotfiles/yatmux/common.toml\"]\n",
        );
        out.push_str("imports = []\n\n");

        out.push_str("[window]\n");
        out.push_str("# title = \"yatmux\"\n\n");

        out.push_str("[terminal]\n");
        out.push_str("# rows = 24\n");
        out.push_str("# cols = 80\n");
        out.push_str("# scrollback_lines = 4096\n");
        out.push_str("# scroll_speed = 3.0\n");
        out.push_str("# tab_width = 8\n\n");

        out.push_str("[font]\n");
        out.push_str("# scale = 2\n\n");

        out.push_str("[pane]\n");
        out.push_str("# Pane layout tweaks. Values are in pixels.\n");
        out.push_str("# Padding adds space between the pane border and the terminal grid.\n");
        out.push_str("#\n");
        out.push_str("# padding = 8\n");
        out.push_str("# padding_left = 8\n");
        out.push_str("# padding_right = 8\n");
        out.push_str("# padding_top = 8\n");
        out.push_str("# padding_bottom = 8\n");
        out.push_str("#\n");
        out.push_str("# Prevent splitting panes too small:\n");
        out.push_str("# min_size = 100\n\n");

        out.push_str("[shell_integration]\n");
        out.push_str("# cwd_from_osc7 = true\n");
        out.push_str("# semantic_zones_from_osc133 = true\n");
        out.push_str("# title_from_osc = true\n");
        out.push_str("# tab_title_source = \"cwd\" # none|cwd|title\n");
        out.push_str("# window_title_follows_active_tab = true\n");
        out.push_str("# sticky_prompt = true\n");
        out.push_str("# shadow_prompt = \"on_typing\" # off|always|on_typing\n");
        out.push_str("# shadow_prompt_enabled_by_default = false\n");
        out.push_str("# debug_log = false\n\n");

        out.push_str("[colors]\n");
        out.push_str("# Base colors for the terminal and as a fallback for UI chrome.\n");
        out.push_str("#\n");
        out.push_str("# With a theme enabled, themes take precedence over [colors] for UI (and often terminal).\n");
        out.push_str("# To fully control colors, disable themes and set these explicitly.\n");
        out.push_str("#\n");
        out.push_str("# background = \"#101010\"\n");
        out.push_str("# foreground = \"#D0D0D0\"\n");
        out.push_str("# accent = \"#66AAFF\"\n");
        out.push_str("#\n");
        out.push_str(
            "# Optional 16-color ANSI palette (colors 0-15). Must have exactly 16 entries:\n",
        );
        out.push_str("# palette = [ \"#000000\", \"#800000\", ... ]\n\n");

        out.push_str("[ui]\n");
        out.push_str("# UI chrome settings (tab bar, overlays, borders).\n");
        out.push_str("# These override [colors] but are overridden by theme files.\n\n");

        out.push_str("[ui.tab_bar]\n");
        out.push_str("# gap_px = 4\n");
        out.push_str("# side_padding_px = 8\n");
        out.push_str("# max_width_cells = 12\n");
        out.push_str("# max_width_px_extra = 16\n\n");

        out.push_str("[ui.search]\n");
        out.push_str("# right_reserved_px = 100\n");
        out.push_str("# match_bg = \"#4A4A00\"\n");
        out.push_str("# current_match_bg = \"#806000\"\n\n");

        out.push_str("[ui.toast]\n");
        out.push_str("# duration_ms = 1500\n\n");

        out.push_str("[ui.help]\n");
        out.push_str("# padding_x_cells = 2\n");
        out.push_str("# padding_y_cells = 1\n\n");

        out.push_str("[ui.dividers]\n");
        out.push_str("# Pane borders + split lines.\n");
        out.push_str("# - Inactive panes use this color for a thin outline.\n");
        out.push_str("# - The focused pane gets an additional accent border.\n");
        out.push_str("#\n");
        out.push_str("# color = \"#222\"\n\n");

        out.push_str("[interaction]\n");
        out.push_str("# click_move_max_steps = 512\n");
        out.push_str("# pane_resize_step = 0.05\n");
        out.push_str("# focus_move_overlap_weight = 1000\n\n");

        out.push_str("[keybinds]\n");
        out.push_str("# \"ctrl+shift+r\" = \"reload_config\"\n");
        out.push_str("# \"ctrl+shift+-\" = \"none\"\n");

        out.push_str("\n# Troubleshooting\n");
        out.push_str("# - Theme not changing? You may have [colors]/[ui] overrides; disable theme or remove overrides.\n");
        out.push_str("# - ANSI colors wrong? [colors].palette must have exactly 16 entries.\n");

        out
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
    #[serde(
        with = "hex_palette_opt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
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

    /// Whether shadow prompt is enabled by default for new panes.
    pub shadow_prompt_enabled_by_default: bool,

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
            shadow_prompt_enabled_by_default: false,
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

#[cfg(test)]
mod merge_tests {
    use super::Config;

    #[test]
    fn theme_overrides_ui_and_colors() {
        let mut merged: toml::Value = r##"
            [colors]
            background = "#000"

            [ui.toast]
            duration_ms = 123
        "##
        .parse()
        .unwrap();

        let theme: toml::Value = r##"
            [colors]
            background = "#111"

            [ui.toast]
            duration_ms = 999
        "##
        .parse()
        .unwrap();

        Config::apply_theme_overrides(&mut merged, &theme);

        let merged_table = merged.as_table().unwrap();
        let colors = merged_table.get("colors").unwrap().as_table().unwrap();
        assert_eq!(colors.get("background").unwrap().as_str().unwrap(), "#111");

        let ui = merged_table.get("ui").unwrap().as_table().unwrap();
        let toast = ui.get("toast").unwrap().as_table().unwrap();
        assert_eq!(toast.get("duration_ms").unwrap().as_integer().unwrap(), 999);
    }
}
