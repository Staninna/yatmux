use log::debug;
use rusttype::{point, Scale};

use crate::config::FontConfig;

use super::FontRenderer;

impl FontRenderer {
    pub fn cell_size(&self, font_config: &FontConfig) -> (usize, usize) {
        let scale = self.clamp_scale(font_config.scale);
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
        let scale = self.clamp_scale(font_config.scale);
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
        let scale = Scale::uniform(font_config.size * self.clamp_scale(font_config.scale));

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
}
