use super::*;

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
