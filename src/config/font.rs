use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FontConfig {
    #[serde(default = "default_family")]
    pub family: String,
    #[serde(default = "default_size")]
    pub size: f32,
    #[serde(default = "default_scale")]
    pub scale: usize,
    #[serde(default)]
    pub weight: FontWeight,
    #[serde(default)]
    pub slant: FontSlant,
}

fn default_family() -> String {
    "JetBrains Mono".to_string()
}

fn default_size() -> f32 {
    14.0
}

fn default_scale() -> usize {
    1
}

impl Default for FontConfig {
    fn default() -> Self {
        FontConfig {
            family: default_family(),
            size: default_size(),
            scale: default_scale(),
            weight: FontWeight::Regular,
            slant: FontSlant::Normal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FontWeight {
    Thin,
    ExtraLight,
    Light,
    SemiLight,
    Regular,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
}

impl Default for FontWeight {
    fn default() -> Self {
        FontWeight::Regular
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FontSlant {
    Normal,
    Italic,
}

impl Default for FontSlant {
    fn default() -> Self {
        FontSlant::Normal
    }
}

impl FontConfig {
    pub fn scaled_size(&self) -> f32 {
        self.size * self.scale as f32
    }
}
