use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use yatmux::config::{Config, PluginConfig};

use super::manager::Plugin;

pub(super) fn discover_plugins(_config: &Config, plugin_cfg: &PluginConfig) -> Vec<Plugin> {
    let base_dir = Config::config_path()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    let mut paths = Vec::new();
    if plugin_cfg.enable_default_dir {
        if let Some(dir) = dirs::config_dir() {
            paths.push(dir.join("yatmux").join("plugins"));
        }
    }
    for path in &plugin_cfg.paths {
        paths.push(resolve_path(&base_dir, path));
    }

    let mut seen = HashSet::new();
    let mut plugins = Vec::new();

    for path in paths {
        if let Some(list) = discover_from_path(&path) {
            for (root, script) in list {
                if !seen.insert(script.clone()) {
                    continue;
                }
                let name = root
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("plugin")
                    .to_string();
                plugins.push(Plugin { name, root, script });
            }
        }
    }

    plugins
}

pub(super) fn discover_from_path(path: &Path) -> Option<Vec<(PathBuf, PathBuf)>> {
    if !path.exists() {
        return None;
    }
    if path.is_file() {
        if path.file_name().and_then(|s| s.to_str()) == Some("plugin.sh") {
            let root = path.parent().unwrap_or(path).to_path_buf();
            return Some(vec![(root, path.to_path_buf())]);
        }
        return None;
    }

    if path.is_dir() {
        let script = path.join("plugin.sh");
        if script.exists() {
            return Some(vec![(path.to_path_buf(), script)]);
        }

        let mut out = Vec::new();
        let entries = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(e) => {
                eprintln!("Failed to read plugins dir {}: {e}", path.display());
                return None;
            }
        };

        let mut dirs = Vec::new();
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_dir() {
                    dirs.push(entry.path());
                }
            }
        }
        dirs.sort();

        for dir in dirs {
            let script = dir.join("plugin.sh");
            if script.exists() {
                out.push((dir, script));
            }
        }
        return Some(out);
    }

    None
}

pub(super) fn resolve_path(base_dir: &Path, input: &str) -> PathBuf {
    let s = input.trim();
    if s == "~" || s.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            if s == "~" {
                return home;
            }
            return home.join(&s[2..]);
        }
    }

    let p = PathBuf::from(s);
    if p.is_absolute() {
        p
    } else {
        base_dir.join(p)
    }
}
