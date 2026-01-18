use super::*;

impl Config {
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

    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("yatmux").join("config.toml"))
    }

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
        out.push_str("# padding_y_cells = 1\n");
        out.push_str("# font_scale = 2 # Preferred help overlay font scale (1-8)\n\n");

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
