use std::path::PathBuf;

use super::super::*;

impl Config {
    pub(super) fn builtin_theme_toml(name: &str) -> Option<&'static str> {
        builtin_themes::BUILTIN_THEMES
            .iter()
            .find_map(|(n, toml)| (*n == name).then_some(*toml))
    }

    pub(super) fn builtin_theme_names() -> &'static [&'static str] {
        builtin_themes::BUILTIN_THEME_NAMES
    }

    pub(super) fn theme_path(name: &str) -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("yatmux").join("themes").join(format!("{name}.toml")))
    }
}
