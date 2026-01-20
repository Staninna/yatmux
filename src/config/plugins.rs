//! Plugin configuration.

use serde::{Deserialize, Serialize};

/// Plugin system configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginConfig {
    /// Enable the plugin system.
    pub enabled: bool,
    /// Extra plugin paths (files or directories).
    pub paths: Vec<String>,
    /// Also load plugins from `~/.config/yatmux/plugins`.
    pub enable_default_dir: bool,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            paths: Vec::new(),
            enable_default_dir: true,
        }
    }
}
