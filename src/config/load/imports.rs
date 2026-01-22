use std::path::{Path, PathBuf};

use super::super::*;

impl Config {
    pub(super) fn resolve_import_path(base_dir: &Path, input: &str) -> PathBuf {
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
}
