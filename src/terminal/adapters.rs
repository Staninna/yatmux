use std::io::{self, Write};
use std::sync::Arc;

use crate::core::color::Color;

use crate::pty::PtyWriter;

use tattoy_wezterm_cell::color::ColorAttribute;
use tattoy_wezterm_term::TerminalConfiguration;
use tattoy_wezterm_term::color::ColorPalette;

#[derive(Debug)]
pub(super) struct TermConfig {
    pub(super) scrollback: usize,
}

impl TerminalConfiguration for TermConfig {
    fn scrollback_size(&self) -> usize {
        self.scrollback
    }

    fn color_palette(&self) -> ColorPalette {
        ColorPalette::default()
    }
}

#[derive(Clone)]
pub(super) struct PtyWriteAdapter {
    pub(super) pty: Arc<dyn PtyWriter>,
}

impl Write for PtyWriteAdapter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.pty.write(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(super) fn color_attr_to_color(color: ColorAttribute) -> Color {
    match color {
        ColorAttribute::Default => Color::Default,
        ColorAttribute::PaletteIndex(idx) => Color::Idx(idx),
        ColorAttribute::TrueColorWithDefaultFallback(srgba) => {
            let (r, g, b, _) = srgba.as_rgba_u8();
            Color::Rgb(r, g, b)
        }
        ColorAttribute::TrueColorWithPaletteFallback(srgba, _fallback) => {
            let (r, g, b, _) = srgba.as_rgba_u8();
            Color::Rgb(r, g, b)
        }
    }
}
