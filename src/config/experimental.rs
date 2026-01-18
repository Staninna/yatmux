use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct FontScaleClampConfig {
    pub min: f32,
    pub max: f32,
}

impl Default for FontScaleClampConfig {
    fn default() -> Self {
        Self { min: 1.0, max: 8.0 }
    }
}

impl FontScaleClampConfig {
    pub fn normalized(self) -> (f32, f32) {
        let mut min = if self.min.is_finite() { self.min } else { 1.0 };
        let mut max = if self.max.is_finite() { self.max } else { 8.0 };

        // Keep values sane; this is used to size glyph caches and pixel loops.
        min = min.clamp(0.25, 64.0);
        max = max.clamp(0.25, 64.0);

        if min > max {
            std::mem::swap(&mut min, &mut max);
        }

        (min, max)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExperimentalConfig {
    pub font_scale_clamp: FontScaleClampConfig,
}

impl Default for ExperimentalConfig {
    fn default() -> Self {
        Self {
            font_scale_clamp: FontScaleClampConfig::default(),
        }
    }
}

