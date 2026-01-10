use std::sync::{Arc, Mutex};

use std::collections::VecDeque;

use anyhow::Result;
use font8x8::UnicodeFonts;
use softbuffer::Surface;
use vt100::Color;

use crate::constants::{
    CELL_H, CELL_W, DEFAULT_BG_COLOR, DEFAULT_FG_COLOR, FONT_SCALE, GLYPH_H, GLYPH_W,
    TAB_STOP_WIDTH,
};

const SCROLLBACK_CAPACITY: usize = 4096;

type CellData = (char, Color, Color);

#[derive(Clone)]
struct RowSnapshot {
    cells: Vec<CellData>,
    tabs: Vec<Option<(usize, usize)>>,
}

impl RowSnapshot {
    fn blank(cols: usize) -> Self {
        RowSnapshot {
            cells: vec![(' ', Color::Default, Color::Default); cols],
            tabs: vec![None; cols],
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct CellPos {
    row: usize,
    col: usize,
}

#[derive(Clone, Copy, Debug)]
struct Selection {
    start: CellPos,
    end: CellPos,
}

impl Selection {
    fn normalized(&self) -> (CellPos, CellPos) {
        if (self.start.row, self.start.col) <= (self.end.row, self.end.col) {
            (self.start, self.end)
        } else {
            (self.end, self.start)
        }
    }

    fn contains(&self, row: usize, col: usize) -> bool {
        let (start, end) = self.normalized();
        if row < start.row || row > end.row {
            return false;
        }
        if start.row == end.row {
            return col >= start.col && col <= end.col;
        }
        if row == start.row {
            return col >= start.col;
        }
        if row == end.row {
            return col <= end.col;
        }
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStyle {
    Basic,
    BoxDrawing,
    Block,
    Greek,
    Hiragana,
    Latin,
    Misc,
    Sga,
}

impl FontStyle {
    fn get_glyph(&self, ch: char) -> [u8; 8] {
        let glyph = match self {
            FontStyle::Basic => font8x8::BASIC_FONTS.get(ch).unwrap_or([0; 8]),
            FontStyle::BoxDrawing => font8x8::BOX_FONTS.get(ch).unwrap_or([0; 8]),
            FontStyle::Block => font8x8::BLOCK_FONTS.get(ch).unwrap_or([0; 8]),
            FontStyle::Greek => font8x8::GREEK_FONTS.get(ch).unwrap_or([0; 8]),
            FontStyle::Hiragana => font8x8::HIRAGANA_FONTS.get(ch).unwrap_or([0; 8]),
            FontStyle::Latin => font8x8::LATIN_FONTS.get(ch).unwrap_or([0; 8]),
            FontStyle::Misc => font8x8::MISC_FONTS.get(ch).unwrap_or([0; 8]),
            FontStyle::Sga => font8x8::SGA_FONTS.get(ch).unwrap_or([0; 8]),
        };
        if glyph != [0; 8] {
            return glyph;
        }
        get_fallback_glyph(ch)
    }
}

fn lighten_color(color: u32) -> u32 {
    let r = ((color >> 16) & 0xff).saturating_add(0x20);
    let g = ((color >> 8) & 0xff).saturating_add(0x20);
    let b = (color & 0xff).saturating_add(0x20);
    ((r.min(0xff) as u32) << 16) | ((g.min(0xff) as u32) << 8) | (b.min(0xff) as u32)
}

fn get_fallback_glyph(ch: char) -> [u8; 8] {
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

pub struct Renderer {
    font_style: FontStyle,
    selection: Option<Selection>,
    scrollback_buffer: VecDeque<RowSnapshot>,
    scrollback_offset: usize,
    max_scrollback: usize,
    view_rows: usize,
    view_cols: usize,
}

impl Default for Renderer {
    fn default() -> Self {
        Renderer::new()
    }
}

impl Renderer {
    pub fn new() -> Self {
        Renderer {
            font_style: FontStyle::Basic,
            selection: None,
            scrollback_buffer: VecDeque::new(),
            scrollback_offset: 0,
            max_scrollback: SCROLLBACK_CAPACITY,
            view_rows: 0,
            view_cols: 0,
        }
    }

    pub fn set_font_style(&mut self, style: FontStyle) {
        self.font_style = style;
    }

    pub fn font_style(&self) -> FontStyle {
        self.font_style
    }

    fn set_dimensions(&mut self, rows: usize, cols: usize) {
        if self.view_rows != rows || self.view_cols != cols {
            self.view_rows = rows;
            self.view_cols = cols;
            self.scrollback_buffer.clear();
            self.scrollback_offset = 0;
            self.selection = None;
        }
    }

    fn clamp_position(&self, row: usize, col: usize) -> CellPos {
        CellPos {
            row: row.min(self.view_rows.saturating_sub(1)),
            col: col.min(self.view_cols.saturating_sub(1)),
        }
    }

    fn draw_cell(
        &self,
        backbuffer: &mut [u32],
        width: usize,
        height: usize,
        row: usize,
        col: usize,
        ch: char,
        invert: bool,
        fg_color: Color,
        bg_color: Color,
        palette: &Arc<[u32; 256]>,
        tab_info: Option<(usize, usize)>,
        selected: bool,
    ) {
        let fg = color_to_u32(fg_color, DEFAULT_FG_COLOR, palette);
        let bg = color_to_u32(bg_color, DEFAULT_BG_COLOR, palette);

        let bg = if invert { fg } else { bg };
        let fg = if invert { bg } else { fg };

        let mut fill_color = bg;
        if selected {
            fill_color = lighten_color(bg);
        } else if tab_info.is_some() {
            fill_color = lighten_color(bg);
        }

        let x0 = col * CELL_W;
        let y0 = row * CELL_H;

        for y in y0..(y0 + CELL_H).min(height) {
            for x in x0..(x0 + CELL_W).min(width) {
                backbuffer[y * width + x] = fill_color;
            }
        }

        if let Some((start_col, _)) = tab_info {
            if start_col == col {
                let tab_fg = lighten_color(fg);
                let glyph = font8x8::BASIC_FONTS.get('>').unwrap_or([0; 8]);
                for gy in 0..GLYPH_H {
                    let bits = glyph[gy];
                    for gx in 0..GLYPH_W {
                        let on = (bits >> gx) & 1 == 1;
                        if !on {
                            continue;
                        }

                        for sy in 0..FONT_SCALE {
                            for sx in 0..FONT_SCALE {
                                let x = x0 + gx * FONT_SCALE + sx;
                                let y = y0 + gy * FONT_SCALE + sy;
                                if x < width && y < height {
                                    backbuffer[y * width + x] = tab_fg;
                                }
                            }
                        }
                    }
                }
            }
            return;
        }

        let glyph = self.font_style.get_glyph(ch);
        for gy in 0..GLYPH_H {
            let bits = glyph[gy];
            for gx in 0..GLYPH_W {
                let on = (bits >> gx) & 1 == 1;
                if !on {
                    continue;
                }

                for sy in 0..FONT_SCALE {
                    for sx in 0..FONT_SCALE {
                        let x = x0 + gx * FONT_SCALE + sx;
                        let y = y0 + gy * FONT_SCALE + sy;
                        if x < width && y < height {
                            backbuffer[y * width + x] = fg;
                        }
                    }
                }
            }
        }
    }

    pub fn render(
        &mut self,
        surface: &mut Surface<winit::event_loop::OwnedDisplayHandle, winit::window::Window>,
        parser: &Arc<Mutex<vt100::Parser>>,
        palette: &Arc<[u32; 256]>,
    ) -> Result<()> {
        let mut buffer = surface
            .buffer_mut()
            .map_err(|e| anyhow::anyhow!("softbuffer buffer_mut failed: {e:?}"))?;
        let buffer_width = buffer.width().get() as usize;
        let buffer_height = buffer.height().get() as usize;
        buffer.fill(DEFAULT_BG_COLOR);

        let (cursor, rows, cols, rows_data) = {
            let parser = parser
                .lock()
                .map_err(|_| anyhow::anyhow!("parser mutex poisoned"))?;
            let screen = parser.screen();
            let cursor = screen.cursor_position();

            let rows = buffer_height / CELL_H;
            let cols = buffer_width / CELL_W;
            self.set_dimensions(rows, cols);

            let mut rows_data = Vec::with_capacity(rows);
            for row in 0..rows {
                let mut row_cells = Vec::with_capacity(cols);
                let mut row_tabs = vec![None; cols];
                for col in 0..cols {
                    let cell = screen.cell(row as u16, col as u16);
                    let contents = cell.map(|c| c.contents()).unwrap_or_default();
                    let ch = contents.chars().next().unwrap_or(' ');
                    let fg = cell.map(|c| c.fgcolor()).unwrap_or(Color::Default);
                    let bg = cell.map(|c| c.bgcolor()).unwrap_or(Color::Default);
                    if ch == '\t' {
                        let end_col = ((col / TAB_STOP_WIDTH) + 1) * TAB_STOP_WIDTH;
                        let end_col = end_col.min(cols);
                        for c in col..end_col {
                            row_tabs[c] = Some((col, end_col));
                        }
                    }
                    row_cells.push((ch, fg, bg));
                }
                rows_data.push(RowSnapshot {
                    cells: row_cells,
                    tabs: row_tabs,
                });
            }
            (cursor, rows, cols, rows_data)
        };

        self.push_rows(&rows_data);

        let display_rows = if self.scrollback_offset == 0 {
            rows_data.clone()
        } else {
            let mut view = Vec::with_capacity(rows);
            let buffer_len = self.scrollback_buffer.len();
            let start = buffer_len.saturating_sub(rows + self.scrollback_offset);
            for idx in start..start + rows {
                if let Some(row) = self.scrollback_buffer.get(idx) {
                    view.push(row.clone());
                } else {
                    view.push(RowSnapshot::blank(cols));
                }
            }
            view
        };

        for (row_idx, row_data) in display_rows.iter().enumerate().take(rows) {
            for col in 0..cols {
                let (ch, fg, bg) = row_data.cells[col];
                let invert = self.scrollback_offset == 0 && (row_idx as u16, col as u16) == cursor;
                let tab_info = row_data.tabs[col];
                let selected = self.is_cell_selected(row_idx, col);
                self.draw_cell(
                    &mut buffer,
                    buffer_width,
                    buffer_height,
                    row_idx,
                    col,
                    ch,
                    invert,
                    fg,
                    bg,
                    palette,
                    tab_info,
                    selected,
                );
            }
        }

        buffer
            .present()
            .map_err(|e| anyhow::anyhow!("softbuffer present failed: {e:?}"))?;
        Ok(())
    }

    fn push_rows(&mut self, rows: &[RowSnapshot]) {
        for row in rows {
            self.scrollback_buffer.push_back(row.clone());
            if self.scrollback_buffer.len() > self.max_scrollback {
                self.scrollback_buffer.pop_front();
            }
        }
        let max_offset = self.scrollback_buffer.len().saturating_sub(self.view_rows);
        if self.scrollback_offset > max_offset {
            self.scrollback_offset = max_offset;
        }
    }

    pub fn scrollback_scroll_by(&mut self, delta_lines: isize) {
        if self.scrollback_buffer.len() <= self.view_rows {
            return;
        }
        let max_offset = self.scrollback_buffer.len().saturating_sub(self.view_rows);
        let offset = (self.scrollback_offset as isize + delta_lines).clamp(0, max_offset as isize);
        self.scrollback_offset = offset as usize;
    }

    pub fn start_selection(&mut self, row: usize, col: usize) {
        let pos = self.clamp_position(row, col);
        self.selection = Some(Selection {
            start: pos,
            end: pos,
        });
    }

    pub fn update_selection(&mut self, row: usize, col: usize) {
        if let Some(mut sel) = self.selection {
            sel.end = self.clamp_position(row, col);
            self.selection = Some(sel);
        }
    }

    fn is_cell_selected(&self, row: usize, col: usize) -> bool {
        self.selection
            .map(|sel| sel.contains(row, col))
            .unwrap_or(false)
    }

    pub fn window_to_cell(&self, x: f64, y: f64) -> Option<(usize, usize)> {
        if self.view_rows == 0 || self.view_cols == 0 {
            return None;
        }
        let col = (x as usize) / CELL_W;
        let row = (y as usize) / CELL_H;
        if row >= self.view_rows || col >= self.view_cols {
            return None;
        }
        Some((row, col))
    }
}

pub fn color_palette() -> [u32; 256] {
    let mut palette = [0u32; 256];
    palette[0] = 0x00_00_00_00;
    palette[1] = 0x00_80_00_00;
    palette[2] = 0x00_00_80_00;
    palette[3] = 0x00_80_80_00;
    palette[4] = 0x00_00_00_80;
    palette[5] = 0x00_80_00_80;
    palette[6] = 0x00_00_FF_FF;
    palette[7] = 0x00_C0_C0_C0;
    palette[8] = 0x00_80_80_80;
    palette[9] = 0x00_FF_00_00;
    palette[10] = 0x00_00_FF_00;
    palette[11] = 0x00_FF_FF_00;
    palette[12] = 0x00_00_00_FF;
    palette[13] = 0x00_FF_00_FF;
    palette[14] = 0x00_00_FF_FF;
    palette[15] = 0x00_FF_FF_FF;
    for i in 16..24 {
        let r = 8 + (i - 16) * 10;
        palette[i] = 0x00 | ((r as u32) << 16) | ((r as u32) << 8) | (r as u32);
    }
    for i in 24..232 {
        let idx = i - 24;
        let r = 40 + (idx / 36) * 40;
        let g = 40 + ((idx / 6) % 6) * 40;
        let b = 40 + (idx % 6) * 40;
        palette[i] = 0x00 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
    }
    for i in 232..256 {
        let gray = 8 + (i - 232) * 10;
        palette[i] = 0x00 | ((gray as u32) << 16) | ((gray as u32) << 8) | (gray as u32);
    }
    palette
}

fn color_to_u32(color: Color, is_default: u32, palette: &[u32; 256]) -> u32 {
    match color {
        Color::Default => is_default,
        Color::Idx(n) => palette[n as usize],
        Color::Rgb(r, g, b) => 0x00 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_palette_size() {
        let palette = color_palette();
        assert_eq!(palette.len(), 256);
    }

    #[test]
    fn test_color_palette_standard_colors() {
        let palette = color_palette();
        assert_eq!(palette[0], 0x00_00_00_00);
        assert_eq!(palette[1], 0x00_80_00_00);
        assert_eq!(palette[7], 0x00_C0_C0_C0);
        assert_eq!(palette[15], 0x00_FF_FF_FF);
    }

    #[test]
    fn test_color_palette_grayscale() {
        let palette = color_palette();
        assert_eq!(palette[232], 0x00_08_08_08);
        assert_eq!(palette[245], 0x00_8A_8A_8A);
        assert_eq!(palette[255], 0x00_EE_EE_EE);
    }

    #[test]
    fn test_color_to_u32_default() {
        let palette = color_palette();
        let result = color_to_u32(Color::Default, 0xFF_AABB_CC, &palette);
        assert_eq!(result, 0xFF_AABB_CC);
    }

    #[test]
    fn test_color_to_u32_indexed() {
        let palette = color_palette();
        let result = color_to_u32(Color::Idx(1), 0, &palette);
        assert_eq!(result, palette[1]);
    }

    #[test]
    fn test_color_to_u32_rgb() {
        let palette = color_palette();
        let result = color_to_u32(Color::Rgb(128, 64, 32), 0, &palette);
        assert_eq!(result, 0x00_80_40_20);
    }

    #[test]
    fn test_font_style_get_glyph() {
        let renderer = Renderer::new();
        let glyph = renderer.font_style.get_glyph('A');
        assert_ne!(glyph, [0; 8]);

        let empty_glyph = renderer.font_style.get_glyph('\u{0000}');
        assert_eq!(empty_glyph, [0; 8]);
    }

    #[test]
    fn test_font_style_box_drawing() {
        let style = FontStyle::BoxDrawing;
        let glyph = style.get_glyph('─');
        assert_ne!(glyph, [0; 8]);
    }

    #[test]
    fn test_renderer_default() {
        let renderer = Renderer::new();
        assert_eq!(renderer.font_style(), FontStyle::Basic);
    }

    #[test]
    fn test_renderer_set_font_style() {
        let mut renderer = Renderer::new();
        renderer.set_font_style(FontStyle::BoxDrawing);
        assert_eq!(renderer.font_style(), FontStyle::BoxDrawing);
        renderer.set_font_style(FontStyle::Greek);
        assert_eq!(renderer.font_style(), FontStyle::Greek);
    }

    #[test]
    fn test_font_fallback_ascii_in_box_drawing() {
        let style = FontStyle::BoxDrawing;
        let glyph = style.get_glyph('A');
        assert_ne!(
            glyph, [0; 8],
            "ASCII 'A' should render in BoxDrawing mode via fallback"
        );
    }

    #[test]
    fn test_font_fallback_ascii_in_greek() {
        let style = FontStyle::Greek;
        let glyph = style.get_glyph('z');
        assert_ne!(
            glyph, [0; 8],
            "ASCII 'z' should render in Greek mode via fallback"
        );
    }

    #[test]
    fn test_get_fallback_glyph() {
        let glyph = get_fallback_glyph('A');
        assert_ne!(glyph, [0; 8]);
        let glyph = get_fallback_glyph('─');
        assert_ne!(glyph, [0; 8]);
        let glyph = get_fallback_glyph('α');
        assert_ne!(glyph, [0; 8]);
    }
}
