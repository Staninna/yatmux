use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use super::KeybindAction;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileDefinition {
    /// Keybindings specific to this profile
    #[serde(flatten)]
    pub keybinds: HashMap<String, KeybindAction>,

    /// Optional border color override (hex color)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_color: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ProfilesConfig {
    #[serde(flatten)]
    pub profiles: HashMap<String, ProfileDefinition>,
}

impl ProfilesConfig {
    pub fn get_profile(&self, name: &str) -> Option<&ProfileDefinition> {
        self.profiles.get(name).or_else(|| self.profiles.get("default"))
    }

    pub fn profile_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.profiles.keys().cloned().collect();
        names.sort();
        // Ensure "default" comes first
        if let Some(pos) = names.iter().position(|n| n == "default") {
            names.remove(pos);
            names.insert(0, "default".to_string());
        }
        names
    }
}

impl Default for ProfilesConfig {
    fn default() -> Self {
        let mut profiles = HashMap::new();
        profiles.insert(
            "default".to_string(),
            ProfileDefinition {
                keybinds: HashMap::new(),
                border_color: None,
            },
        );
        ProfilesConfig { profiles }
    }
}
