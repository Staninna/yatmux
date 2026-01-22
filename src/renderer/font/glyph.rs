use crate::config::FontConfig;
use font8x8::UnicodeFonts;
use log::debug;
use rusttype::{point, Scale};
use std::sync::Arc;

use super::FontRenderer;

#[derive(Clone, Hash, PartialEq, Eq)]
pub(super) struct GlyphCacheKey {
    pub(super) character: char,
    pub(super) family: String,
    pub(super) size_key: u32,
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
    pub fn get_glyph(
        &self,
        character: char,
        font_config: &FontConfig,
    ) -> Result<Option<Arc<RenderedGlyph>>, String> {
        let family = &font_config.family;
        let scale_factor = self.clamp_scale(font_config.scale);
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
