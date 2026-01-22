use crate::config::FontConfig;
use log::debug;
use rusttype::Font;
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

mod glyph;
mod load;
mod metrics;

pub use glyph::{get_bitmap_glyph, tab_indicator_glyph, RenderedGlyph};

pub struct FontRenderer {
    fonts: RefCell<HashMap<String, Font<'static>>>,
    glyph_cache: RefCell<HashMap<glyph::GlyphCacheKey, Arc<RenderedGlyph>>>,
    scale_min: Cell<f32>,
    scale_max: Cell<f32>,
}

/// Font Memory Management
///
/// This renderer uses Box::leak() to create 'static lifetime font data required by rusttype::Font.
/// - Bundled fonts: Leaked once at initialization (~280KB)
/// - System fonts: Leaked when first loaded, cached forever
///
/// Typical memory cost: <2MB for 1-3 fonts (acceptable for long-lived process).
/// Trade-off: Simple lifetime management vs. no font hot-reloading.
impl FontRenderer {
    pub fn new() -> Result<Self, String> {
        let glyph_cache = RefCell::new(HashMap::new());

        let bundled_font_data =
            include_bytes!("../../../fonts/JetBrainsMono-Regular.ttf").to_vec();
        let static_font_data: &'static [u8] = Box::leak(bundled_font_data.into_boxed_slice());

        let fonts = RefCell::new(HashMap::new());
        let renderer = FontRenderer {
            fonts,
            glyph_cache,
            scale_min: Cell::new(1.0),
            scale_max: Cell::new(8.0),
        };

        if let Some(font) = Font::try_from_bytes(static_font_data) {
            debug!("Loaded bundled JetBrains Mono font");
            let mut fonts = renderer.fonts.borrow_mut();
            fonts.insert("default".to_string(), font.clone());
            fonts.insert("JetBrains Mono".to_string(), font);
        } else {
            return Err("Failed to parse bundled font".to_string());
        }

        Ok(renderer)
    }

    pub fn set_scale_clamp(&self, scale_min: f32, scale_max: f32) {
        let mut min = if scale_min.is_finite() { scale_min } else { 1.0 };
        let mut max = if scale_max.is_finite() { scale_max } else { 8.0 };
        min = min.clamp(0.25, 64.0);
        max = max.clamp(0.25, 64.0);
        if min > max {
            std::mem::swap(&mut min, &mut max);
        }

        let old_min = self.scale_min.get();
        let old_max = self.scale_max.get();
        if (old_min, old_max) == (min, max) {
            return;
        }

        self.scale_min.set(min);
        self.scale_max.set(max);
        self.clear_cache();
    }

    pub fn clamp_scale(&self, scale: f32) -> f32 {
        let min = self.scale_min.get();
        let max = self.scale_max.get();
        scale.clamp(min, max)
    }

    pub fn quantize_scale(&self, scale: f32) -> usize {
        self.clamp_scale(scale).round().max(1.0) as usize
    }

    pub fn configure(&self, config: FontConfig) {
        debug!(
            "Configuring font: family={}, size={}, weight={:?}, slant={:?}",
            config.family, config.size, config.weight, config.slant
        );
        let _ = self.load_font_by_name(&config.family);
        self.clear_cache();
        debug!("Font configuration complete, cache cleared");
    }

    pub fn clear_cache(&self) {
        self.glyph_cache.borrow_mut().clear();
    }
}

impl Default for FontRenderer {
    fn default() -> Self {
        Self::new().expect("Failed to initialize font renderer")
    }
}
