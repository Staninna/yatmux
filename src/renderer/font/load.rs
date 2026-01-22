use log::{debug, warn};
use rusttype::Font;

use super::FontRenderer;

impl FontRenderer {
    pub(super) fn load_font_by_name(&self, family: &str) -> Font<'static> {
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

    pub(super) fn get_default_font(&self) -> Font<'static> {
        self.fonts
            .borrow()
            .get("default")
            .cloned()
            .expect("default font should be available")
    }
}

fn find_system_font(_name: &str) -> Option<std::path::PathBuf> {
    None
}
