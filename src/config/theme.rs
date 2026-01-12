use super::*;

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
