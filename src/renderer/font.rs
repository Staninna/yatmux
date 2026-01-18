use crate::config::FontConfig;
use font8x8::UnicodeFonts;
use log::{debug, warn};
use rusttype::{Font, Scale, point};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

pub struct FontRenderer {
    fonts: RefCell<HashMap<String, Font<'static>>>,
    glyph_cache: RefCell<HashMap<GlyphCacheKey, Arc<RenderedGlyph>>>,
}

#[derive(Clone, Hash, PartialEq, Eq)]
struct GlyphCacheKey {
    character: char,
    family: String,
    size_key: u32,
}

pub struct RenderedGlyph {
    pub pixels: Arc<[u8]>,
    pub width: usize,
    pub height: usize,
    pub bearing_x: i32,
    pub bearing_y: i32,
    pub advance: f32,
}

const GLYPH_CACHE_MAX_ENTRIES: usize = 10000;

impl FontRenderer {
    pub fn new() -> Result<Self, String> {
        let glyph_cache = RefCell::new(HashMap::new());

        let bundled_font_data = include_bytes!("../../fonts/JetBrainsMono-Regular.ttf").to_vec();
        let static_font_data: &'static [u8] = Box::leak(bundled_font_data.into_boxed_slice());

        let fonts = RefCell::new(HashMap::new());
        let renderer = FontRenderer { fonts, glyph_cache };

        if let Some(font) = Font::try_from_bytes(static_font_data) {
            debug!("Loaded bundled JetBrains Mono font");
            renderer
                .fonts
                .borrow_mut()
                .insert("default".to_string(), font);
        } else {
            return Err("Failed to parse bundled font".to_string());
        }

        Ok(renderer)
    }

    fn load_font_by_name(&self, family: &str) -> Font<'static> {
        if let Some(font) = self.fonts.borrow().get(family).cloned() {
            return font;
        }

        let font_data = match find_system_font(family) {
            Some(path) => match std::fs::read(&path) {
                Ok(data) => data,
                Err(e) => {
                    warn!("Failed to read font file {}: {}", path.display(), e);
                    return self.get_default_font();
                }
            },
            None => {
                debug!(
                    "Font '{}' not found in system, using bundled fallback",
                    family
                );
                return self.get_default_font();
            }
        };

        let static_font_data: &'static [u8] = Box::leak(font_data.into_boxed_slice());
        if let Some(font) = Font::try_from_bytes(static_font_data) {
            debug!("Loaded system font: {}", family);
            self.fonts.borrow_mut().insert(family.to_string(), font);
            self.fonts
                .borrow()
                .get(family)
                .cloned()
                .unwrap_or(self.get_default_font())
        } else {
            warn!("Failed to parse font '{}'", family);
            self.get_default_font()
        }
    }

    fn get_default_font(&self) -> Font<'static> {
        self.fonts
            .borrow()
            .get("default")
            .cloned()
            .expect("default font should be available")
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

    pub fn get_glyph(
        &self,
        character: char,
        font_config: &FontConfig,
    ) -> Result<Option<Arc<RenderedGlyph>>, String> {
        let family = &font_config.family;
        let scale_factor = font_config.scale.clamp(1, 8) as f32;
        let scaled_size = font_config.size * scale_factor;
        let size_key = (scaled_size * 100.0).round().max(1.0) as u32;

        let cache_key = GlyphCacheKey {
            character,
            family: family.clone(),
            size_key,
        };

        if let Some(cached) = self.glyph_cache.borrow().get(&cache_key) {
            return Ok(Some(cached.clone()));
        }

        let font = self.load_font_by_name(family);

        let scale = Scale::uniform(scaled_size);
        let glyph = font.glyph(character).scaled(scale);
        let pg = glyph.positioned(point(0.0, 0.0));

        if let Some(bb) = pg.pixel_bounding_box() {
            let width = bb.width() as usize;
            let height = bb.height() as usize;
            let bearing_x = bb.min.x;
            let bearing_y = -bb.min.y; // Distance from baseline to glyph top

            let mut pixels = vec![0u8; width * height];
            pg.draw(|x, y, v| {
                let idx = (y as usize) * width + (x as usize);
                if idx < pixels.len() {
                    pixels[idx] = (v * 255.0) as u8;
                }
            });

            let advance = font
                .glyph(character)
                .scaled(scale)
                .h_metrics()
                .advance_width;

            let rendered = Arc::new(RenderedGlyph {
                pixels: pixels.into(),
                width,
                height,
                bearing_x,
                bearing_y,
                advance,
            });

            let mut cache = self.glyph_cache.borrow_mut();
            if cache.len() >= GLYPH_CACHE_MAX_ENTRIES {
                cache.clear();
            }

            cache.insert(cache_key, rendered.clone());

            debug!(
                "glyph '{}': w={}, h={}, bearing_y={}, advance={}",
                character, width, height, bearing_y, advance
            );

            return Ok(Some(rendered));
        }

        debug!("glyph '{}': failed to rasterize", character);
        Ok(None)
    }

    pub fn cell_size(&self, font_config: &FontConfig) -> (usize, usize) {
        let scale = font_config.scale.clamp(1, 8) as f32;
        let scaled_size = font_config.size * scale;
        let font = self.load_font_by_name(&font_config.family);
        let rt_scale = Scale::uniform(scaled_size);
        let metrics = font.v_metrics(rt_scale);
        let cell_h = (metrics.ascent - metrics.descent + metrics.line_gap)
            .ceil()
            .max(1.0) as usize;
        let advance = font.glyph('M').scaled(rt_scale).h_metrics().advance_width;
        let cell_w = if advance > 0.0 {
            advance.ceil().max(1.0) as usize
        } else {
            (scaled_size * 0.6).ceil().max(1.0) as usize
        };
        debug!(
            "cell_size: size={}, scale={}, cell={}x{}",
            font_config.size, scale, cell_w, cell_h
        );
        (cell_w, cell_h)
    }

    pub fn baseline_offset(&self, font_config: &FontConfig) -> i32 {
        let scale = font_config.scale.clamp(1, 8) as f32;
        let scaled_size = font_config.size * scale;
        let font = self.load_font_by_name(&font_config.family);
        let metrics = font.v_metrics(Scale::uniform(scaled_size));
        let baseline = metrics.ascent.max(1.0);
        debug!(
            "baseline_offset: size={}, scale={}, baseline={}",
            font_config.size, scale, baseline
        );
        baseline as i32
    }

    /// Returns the maximum bearing_y across all printable ASCII characters.
    /// Used for consistent top-alignment of glyphs in terminal cells.
    pub fn max_bearing_y(&self, font_config: &FontConfig) -> i32 {
        let font = self.load_font_by_name(&font_config.family);
        let scale = Scale::uniform(font_config.size * font_config.scale.clamp(1, 8) as f32);

        let mut max_bearing = 0i32;
        // Measure all printable ASCII to find the tallest glyph
        for ch in ' '..='~' {
            let glyph = font.glyph(ch).scaled(scale);
            let pg = glyph.positioned(point(0.0, 0.0));
            if let Some(bb) = pg.pixel_bounding_box() {
                let bearing_y = -bb.max.y;
                max_bearing = max_bearing.max(bearing_y);
            }
        }

        max_bearing
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

fn find_system_font(_name: &str) -> Option<std::path::PathBuf> {
    None
}

pub fn tab_indicator_glyph() -> [u8; 8] {
    font8x8::BASIC_FONTS.get('>').unwrap_or([0; 8])
}

pub fn get_bitmap_glyph(ch: char) -> [u8; 8] {
    font8x8::BASIC_FONTS
        .get(ch)
        .or_else(|| font8x8::BOX_FONTS.get(ch))
        .or_else(|| font8x8::BLOCK_FONTS.get(ch))
        .or_else(|| font8x8::GREEK_FONTS.get(ch))
        .or_else(|| font8x8::HIRAGANA_FONTS.get(ch))
        .or_else(|| font8x8::LATIN_FONTS.get(ch))
        .or_else(|| font8x8::MISC_FONTS.get(ch))
        .or_else(|| font8x8::SGA_FONTS.get(ch))
        .unwrap_or([0; 8])
}
