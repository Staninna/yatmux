use std::fs;
use std::path::PathBuf;

use super::super::*;

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
            Self::default_config_template_for_path(&path)
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
