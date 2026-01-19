use vt100::Color;

use super::Renderer;

use super::super::color::color_to_u32;
use super::super::font;
use super::super::view::TerminalView;
use super::super::{HelpSection, UiStyle};
use crate::config::FontConfig;

impl Renderer {
    pub fn paint_help_overlay(
        &mut self,
        backbuffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        title: &str,
        sections: &[HelpSection],
        filter_query: Option<String>,
        match_count: Option<usize>,
        scroll_offset: usize,
        accent_color: u32,
        _font_scale: f32,
        shell_integration_detected: bool,
        style: &UiStyle,
        font_config: &FontConfig,
    ) -> (usize, usize) {
        let padding_cells_x = style.help_padding_x_cells;
        let padding_cells_y = style.help_padding_y_cells;

        let fixed_lines = if let Some(ref query) = filter_query {
            vec![
                title.to_string(),
                String::new(),
                format!("Filter: {}▎ ({} matches)", query, match_count.unwrap_or(0)),
                String::new(),
            ]
        } else {
            vec![title.to_string(), String::new()]
        };
        let mut blocks: Vec<HelpBlock> = Vec::new();

        let footer_lines: Vec<String> = if shell_integration_detected {
            Vec::new()
        } else {
            vec![
                String::new(),
                "Shell integration not detected".to_string(),
                "Source scripts/shell/yatmux.bash in your shell".to_string(),
            ]
        };

        let max_key_len = sections
            .iter()
            .flat_map(|section| section.bindings.iter().map(|(key, _)| key.len()))
            .max()
            .unwrap_or(0);
        let key_col_width = HELP_KEY_COL_WIDTH.max(max_key_len);

        let mut max_line_len = title.len();
        for line in &fixed_lines {
            max_line_len = max_line_len.max(line.len());
        }
        for line in &footer_lines {
            max_line_len = max_line_len.max(line.len());
        }

        // Handle "no matches" case when filter is active
        if filter_query.is_some() && sections.is_empty() {
            let no_matches_msg = "No matches found";
            max_line_len = max_line_len.max(no_matches_msg.len());
            let mut lines = Vec::new();
            lines.push(HelpLine::Item {
                key: String::new(),
                action: no_matches_msg.to_string(),
            });
            let block = HelpBlock::new(lines, key_col_width);
            max_line_len = max_line_len.max(block.max_len);
            blocks.push(block);
        } else {
            for (section_idx, section) in sections.iter().enumerate() {
                let mut lines = Vec::new();
                max_line_len = max_line_len.max(section.title.len());
                lines.push(HelpLine::Header(section.title.clone()));

                for (key, action) in &section.bindings {
                    lines.push(HelpLine::Item {
                        key: key.clone(),
                        action: action.to_string(),
                    });
                }

                if section_idx + 1 < sections.len() {
                    lines.push(HelpLine::Spacer);
                }

                let block = HelpBlock::new(lines, key_col_width);
                max_line_len = max_line_len.max(block.max_len);
                blocks.push(block);
            }
        }

        let content_max_len = blocks.iter().map(|block| block.max_len).max().unwrap_or(0);
        let gutter_cells = 4usize;
        let max_scale = self.font_renderer.quantize_scale(style.help_font_scale_max);
        let mut selected_layout: Option<HelpLayout> = None;
        let mut fallback_layout: Option<HelpLayout> = None;

        for scale in (1..=max_scale).rev() {
            let mut probe_font = font_config.clone();
            probe_font.scale = scale as f32;
            let (cell_w, cell_h) = self.font_renderer.cell_size(&probe_font);
            let available_cols = buffer_width / cell_w;
            let available_rows = buffer_height / cell_h;
            if available_cols < 10 || available_rows < 5 {
                continue;
            }

            let content_rows_capacity = available_rows
                .saturating_sub(fixed_lines.len() + footer_lines.len() + padding_cells_y * 2);
            if content_rows_capacity == 0 {
                continue;
            }

            let two_col_cols = content_max_len * 2 + gutter_cells;
            let can_use_two_columns =
                available_cols >= two_col_cols + padding_cells_x * 2 && !blocks.is_empty();
            let use_two_columns = can_use_two_columns;
            let required_cols = if use_two_columns {
                max_line_len.max(two_col_cols)
            } else {
                max_line_len
            };
            if required_cols + padding_cells_x * 2 > available_cols {
                continue;
            }

            let layout = HelpLayout {
                scale,
                cell_w,
                cell_h,
                available_cols,
                available_rows,
                use_two_columns,
                content_rows_capacity,
                required_cols,
            };

            if use_two_columns && can_fit_blocks(&blocks, content_rows_capacity) {
                selected_layout = Some(layout);
                break;
            }

            if fallback_layout.is_none() {
                fallback_layout = Some(layout);
            }
        }

        let layout = match selected_layout.or(fallback_layout) {
            Some(layout) => layout,
            None => return (0, 0),
        };

        let mut help_font = font_config.clone();
        help_font.scale = layout.scale as f32;

        let cell_w = layout.cell_w;
        let cell_h = layout.cell_h;
        let use_two_columns = layout.use_two_columns;
        let font_scale = layout.scale as f32;

        let total_content_rows: usize = blocks.iter().map(|block| block.height).sum();
        let content_rows_needed = if use_two_columns {
            (total_content_rows + 1) / 2
        } else {
            total_content_rows
        };

        let box_cols = layout.available_cols * 9 / 10;
        let total_rows =
            fixed_lines.len() + content_rows_needed + footer_lines.len() + padding_cells_y * 2;
        let box_rows = total_rows.min(layout.available_rows * 9 / 10);

        let box_w = box_cols * cell_w;
        let box_h = box_rows * cell_h;
        let origin_x = buffer_width.saturating_sub(box_w) / 2;
        let origin_y = buffer_height.saturating_sub(box_h) / 2;

        let bg = style.help_bg;
        let border = accent_color;

        for y in origin_y..(origin_y + box_h).min(buffer_height) {
            let row = y * buffer_width;
            for x in origin_x..(origin_x + box_w).min(buffer_width) {
                backbuffer[row + x] = bg;
            }
        }

        for x in origin_x..(origin_x + box_w).min(buffer_width) {
            backbuffer[origin_y * buffer_width + x] = border;
            let yb = (origin_y + box_h - 1).min(buffer_height - 1);
            backbuffer[yb * buffer_width + x] = border;
        }
        for y in origin_y..(origin_y + box_h).min(buffer_height) {
            backbuffer[y * buffer_width + origin_x] = border;
            let xb = (origin_x + box_w - 1).min(buffer_width - 1);
            backbuffer[y * buffer_width + xb] = border;
        }

        let content_rows = box_rows
            .saturating_sub(padding_cells_y * 2)
            .saturating_sub(fixed_lines.len())
            .saturating_sub(footer_lines.len());
        let block_starts = block_start_rows(&blocks);
        let (scroll, start_idx, scroll_total, scroll_visible, max_scroll) = if use_two_columns {
            let mut max_scroll_idx = 0usize;
            for start_idx in 0..blocks.len() {
                let placements = layout_blocks(&blocks, start_idx, 2, content_rows);
                if !placements.is_empty() {
                    if placements
                        .last()
                        .map(|p| p.index == blocks.len() - 1)
                        .unwrap_or(false)
                    {
                        max_scroll_idx = start_idx;
                        break;
                    }
                }
            }
            let scroll = scroll_offset.min(max_scroll_idx);
            let placements = layout_blocks(&blocks, scroll, 2, content_rows);
            (
                scroll,
                scroll,
                blocks.len().max(1),
                placements.len().max(1),
                max_scroll_idx,
            )
        } else {
            let mut max_scroll = 0usize;
            for start_idx in 0..blocks.len() {
                let placements = layout_blocks(&blocks, start_idx, 1, content_rows);
                if !placements.is_empty() {
                    max_scroll = block_starts[start_idx];
                }
            }
            let scroll = scroll_offset.min(max_scroll);
            let start_idx = find_start_block(&blocks, &block_starts, scroll);
            let scroll = block_starts.get(start_idx).copied().unwrap_or(0);
            (
                scroll,
                start_idx,
                total_content_rows.max(1),
                content_rows.max(1),
                total_content_rows.saturating_sub(content_rows),
            )
        };
        let scroll = scroll.min(max_scroll);

        if max_scroll > 0 && content_rows > 0 {
            self.draw_help_scrollbar(
                backbuffer,
                buffer_width,
                buffer_height,
                origin_x,
                origin_y,
                box_w,
                box_h,
                cell_h,
                padding_cells_y,
                fixed_lines.len(),
                scroll_total,
                scroll_visible,
                scroll,
                max_scroll,
                accent_color,
                style.help_footer_text,
            );
        }

        let text_color = style.help_text;
        let header_color = accent_color;
        let key_color = accent_color;
        let mut y = origin_y + padding_cells_y * cell_h;
        let text_x0 = origin_x + padding_cells_x * cell_w;
        let text_x1 = (origin_x + box_w).saturating_sub(padding_cells_x * cell_w);

        for (idx, line) in fixed_lines.iter().enumerate() {
            if idx + padding_cells_y >= box_rows {
                break;
            }
            let line_kind = if idx == 0 {
                HelpLine::Header(line.clone())
            } else if line.is_empty() {
                HelpLine::Spacer
            } else {
                // Non-empty lines (like filter text) as items with empty key
                HelpLine::Item {
                    key: String::new(),
                    action: line.clone(),
                }
            };
            self.draw_help_line(
                backbuffer,
                buffer_width,
                buffer_height,
                y,
                text_x0,
                text_x1,
                &line_kind,
                text_color,
                header_color,
                key_color,
                key_col_width,
                &help_font,
                cell_w,
                cell_h,
                font_scale,
            );
            y += cell_h;
        }

        let content_origin_y = origin_y + padding_cells_y * cell_h + fixed_lines.len() * cell_h;
        let placements = layout_blocks(
            &blocks,
            start_idx,
            if use_two_columns { 2 } else { 1 },
            content_rows,
        );

        let content_x0 = origin_x + padding_cells_x * cell_w;
        let content_x1 = (origin_x + box_w).saturating_sub(padding_cells_x * cell_w);
        let gutter_px = gutter_cells * cell_w;
        let col_width = if use_two_columns {
            content_x1
                .saturating_sub(content_x0)
                .saturating_sub(gutter_px)
                / 2
        } else {
            content_x1.saturating_sub(content_x0)
        };
        let col_x0 = content_x0;
        let col_x1 = col_x0 + col_width;
        let col2_x0 = col_x1 + gutter_px;
        let col2_x1 = (col2_x0 + col_width).min(content_x1);

        for placement in placements {
            let (x0, x1) = if placement.col == 0 {
                (col_x0, if use_two_columns { col_x1 } else { col2_x1 })
            } else {
                (col2_x0, col2_x1)
            };
            let mut line_y = content_origin_y + placement.row * cell_h;
            for line in &blocks[placement.index].lines {
                self.draw_help_line(
                    backbuffer,
                    buffer_width,
                    buffer_height,
                    line_y,
                    x0,
                    x1,
                    line,
                    text_color,
                    header_color,
                    key_color,
                    key_col_width,
                    &help_font,
                    cell_w,
                    cell_h,
                    font_scale,
                );
                line_y += cell_h;
            }
        }

        let footer_color = style.help_footer_text;
        for line in &footer_lines {
            self.draw_help_text_at(
                backbuffer,
                buffer_width,
                buffer_height,
                text_x0,
                text_x1,
                y,
                line,
                footer_color,
                &help_font,
                cell_w,
                font_scale,
            );
            y += cell_h;
        }

        (scroll, max_scroll)
    }

    fn draw_help_scrollbar(
        &self,
        backbuffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        origin_x: usize,
        origin_y: usize,
        box_w: usize,
        box_h: usize,
        cell_h: usize,
        padding_cells_y: usize,
        fixed_lines_len: usize,
        content_lines_len: usize,
        content_rows: usize,
        scroll: usize,
        max_scroll: usize,
        accent_color: u32,
        footer_color: u32,
    ) {
        let track_y0 = origin_y + padding_cells_y * cell_h + fixed_lines_len * cell_h;
        let track_y1 = origin_y + box_h - padding_cells_y * cell_h;
        let track_y0 = track_y0.min(buffer_height.saturating_sub(1));
        let track_y1 = track_y1.min(buffer_height);

        if track_y1 <= track_y0 {
            return;
        }

        let track_h = track_y1 - track_y0;
        let total = content_lines_len.max(1);
        let visible = content_rows.min(total);

        let mut thumb_h = (track_h * visible) / total;
        thumb_h = thumb_h.max(cell_h).min(track_h);

        let travel = track_h.saturating_sub(thumb_h);
        let thumb_y0 = if max_scroll == 0 {
            track_y0
        } else {
            track_y0 + (travel * scroll) / max_scroll
        };
        let thumb_y1 = (thumb_y0 + thumb_h).min(track_y1);

        let bar_w = 2usize;
        let bar_x1 = (origin_x + box_w).min(buffer_width).saturating_sub(2);
        let bar_x0 = bar_x1.saturating_sub(bar_w);

        let track_color = footer_color;
        let thumb_color = accent_color;

        for y in track_y0..track_y1 {
            let row = y * buffer_width;
            for x in bar_x0..bar_x1 {
                backbuffer[row + x] = track_color;
            }
        }

        for y in thumb_y0..thumb_y1 {
            let row = y * buffer_width;
            for x in bar_x0..bar_x1 {
                backbuffer[row + x] = thumb_color;
            }
        }
    }

    fn draw_help_line(
        &mut self,
        backbuffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        y: usize,
        x0: usize,
        x1: usize,
        line: &HelpLine,
        text_color: u32,
        header_color: u32,
        key_color: u32,
        key_col_width: usize,
        font_config: &FontConfig,
        cell_w: usize,
        cell_h: usize,
        font_scale: f32,
    ) {
        match line {
            HelpLine::Spacer => {}
            HelpLine::Header(text) => {
                self.draw_help_text_at(
                    backbuffer,
                    buffer_width,
                    buffer_height,
                    x0,
                    x1,
                    y,
                    text,
                    header_color,
                    font_config,
                    cell_w,
                    font_scale,
                );

                let underline_y = y.saturating_add(cell_h.saturating_sub(2));
                if underline_y < buffer_height {
                    let end_x = (x0 + text.len() * cell_w).min(x1);
                    let row = underline_y * buffer_width;
                    for ux in x0..end_x {
                        if ux < buffer_width {
                            backbuffer[row + ux] = header_color;
                        }
                    }
                }
            }
            HelpLine::Item { key, action } => {
                let key_text = format!("{:<width$}", key, width = key_col_width);
                let key_x0 = x0.saturating_add(HELP_LINE_INDENT * cell_w);
                let action_x0 = key_x0
                    .saturating_add(key_col_width * cell_w)
                    .saturating_add(HELP_KEY_GAP * cell_w);

                self.draw_help_text_at(
                    backbuffer,
                    buffer_width,
                    buffer_height,
                    key_x0,
                    x1,
                    y,
                    &key_text,
                    key_color,
                    font_config,
                    cell_w,
                    font_scale,
                );
                self.draw_help_text_at(
                    backbuffer,
                    buffer_width,
                    buffer_height,
                    action_x0,
                    x1,
                    y,
                    action,
                    text_color,
                    font_config,
                    cell_w,
                    font_scale,
                );
            }
        }
    }

    fn draw_help_text_at(
        &mut self,
        backbuffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        x0: usize,
        x1: usize,
        y: usize,
        text: &str,
        text_color: u32,
        font_config: &FontConfig,
        cell_w: usize,
        font_scale: f32,
    ) {
        let mut x = x0;
        for ch in text.chars() {
            if x + cell_w > x1 {
                break;
            }
            if let Ok(Some(tt_glyph)) = self.font_renderer.get_glyph(ch, font_config) {
                let baseline_offset = self.font_renderer.baseline_offset(font_config);
                let glyph_y = y
                    .saturating_add(baseline_offset as usize)
                    .saturating_sub(tt_glyph.bearing_y as usize);
                self.draw_native_glyph(
                    backbuffer,
                    buffer_width,
                    buffer_height,
                    x,
                    glyph_y,
                    &tt_glyph.pixels,
                    tt_glyph.width,
                    tt_glyph.height,
                    text_color,
                );
            } else {
                let glyph = font::get_bitmap_glyph(ch);
                self.draw_glyph_help(
                    backbuffer,
                    buffer_width,
                    buffer_height,
                    x0,
                    0,
                    x1.saturating_sub(x0),
                    buffer_height,
                    font_scale,
                    x,
                    y,
                    glyph,
                    text_color,
                );
            }
            x += cell_w;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_glyph_help(
        &self,
        backbuffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        origin_x: usize,
        origin_y: usize,
        box_w: usize,
        box_h: usize,
        font_scale: f32,
        x0: usize,
        y0: usize,
        glyph: [u8; 8],
        color: u32,
    ) {
        let clip_right = (origin_x + box_w).min(buffer_width);
        let clip_bottom = (origin_y + box_h).min(buffer_height);

        let font_scale = self.font_renderer.quantize_scale(font_scale);

        for gy in 0..8 {
            let bits = glyph[gy];
            for gx in 0..8 {
                let on = (bits >> gx) & 1 == 1;
                if !on {
                    continue;
                }

                for sy in 0..font_scale {
                    for sx in 0..font_scale {
                        let x = x0 + gx * font_scale + sx;
                        let y = y0 + gy * font_scale + sy;
                        if x < clip_right && y < clip_bottom {
                            backbuffer[y * buffer_width + x] = color;
                        }
                    }
                }
            }
        }
    }

    /// Paint a sticky prompt at the bottom of a pane region.
    #[allow(clippy::too_many_arguments)]
    pub fn paint_sticky_prompt(
        &mut self,
        backbuffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        origin_x: usize,
        origin_y: usize,
        region_w: usize,
        region_h: usize,
        cell_w: usize,
        cell_h: usize,
        font_scale: f32,
        prompt_rows: &[crate::core::grid::RowSnapshot],
        cursor: Option<(usize, usize)>,
        palette: &[u32; 256],
        style: &UiStyle,
        font_config: &FontConfig,
    ) {
        if prompt_rows.is_empty() {
            return;
        }

        let num_prompt_rows = prompt_rows.len();
        let padding_top = cell_h / 2;
        let padding_bottom = cell_h / 2;
        let prompt_height = num_prompt_rows * cell_h + padding_top + padding_bottom;

        if prompt_height > region_h / 2 {
            return;
        }

        let sticky_area_y = origin_y + region_h - prompt_height;
        let prompt_y = sticky_area_y + padding_top;

        let separator_y = sticky_area_y.saturating_sub(1);
        if separator_y >= origin_y && separator_y < origin_y + region_h {
            for x in origin_x..(origin_x + region_w).min(buffer_width) {
                if separator_y < buffer_height {
                    backbuffer[separator_y * buffer_width + x] = style.sticky_prompt_separator;
                }
            }
        }

        let sticky_bg = style.sticky_prompt_bg;
        for y in sticky_area_y..(origin_y + region_h).min(buffer_height) {
            for x in origin_x..(origin_x + region_w).min(buffer_width) {
                backbuffer[y * buffer_width + x] = sticky_bg;
            }
        }

        for (row_idx, row_data) in prompt_rows.iter().enumerate() {
            let cols = row_data.cells.len();
            for col in 0..cols {
                let (ch, fg_color, bg_color) = row_data.cells.get(col).copied().unwrap_or((
                    ' ',
                    Color::Default,
                    Color::Default,
                ));

                let is_cursor = cursor
                    .map(|(r, c)| r == row_idx && c == col)
                    .unwrap_or(false);

                let fg = color_to_u32(fg_color, style.base_fg, palette);
                let bg = color_to_u32(bg_color, sticky_bg, palette);

                let fill_color = if matches!(bg_color, Color::Default) {
                    sticky_bg
                } else {
                    bg
                };

                let (fg, fill_color) = if is_cursor {
                    (fill_color, fg)
                } else {
                    (fg, fill_color)
                };

                let x0 = origin_x + col * cell_w;
                let y0 = prompt_y + row_idx * cell_h;

                let clip_right = (origin_x + region_w).min(buffer_width);
                let clip_bottom = (origin_y + region_h).min(buffer_height);

                if x0 >= clip_right || y0 >= clip_bottom {
                    continue;
                }

                for y in y0..(y0 + cell_h).min(clip_bottom) {
                    for x in x0..(x0 + cell_w).min(clip_right) {
                        backbuffer[y * buffer_width + x] = fill_color;
                    }
                }

                if let Ok(Some(tt_glyph)) = self.font_renderer.get_glyph(ch, font_config) {
                    let baseline_offset = self.font_renderer.baseline_offset(font_config);
                    let glyph_y = y0
                        .saturating_add(baseline_offset as usize)
                        .saturating_sub(tt_glyph.bearing_y as usize);
                    self.draw_native_glyph(
                        backbuffer,
                        buffer_width,
                        buffer_height,
                        x0,
                        glyph_y,
                        &tt_glyph.pixels,
                        tt_glyph.width,
                        tt_glyph.height,
                        fg,
                    );
                } else {
                    let glyph = font::get_bitmap_glyph(ch);
                    self.draw_glyph(
                        backbuffer,
                        buffer_width,
                        buffer_height,
                        origin_x,
                        origin_y,
                        region_w,
                        region_h,
                        font_scale,
                        x0,
                        y0,
                        glyph,
                        fg,
                    );
                }
            }
        }
    }

    pub(super) fn draw_search_bar(
        &mut self,
        buffer: &mut [u32],
        width: usize,
        height: usize,
        origin_x: usize,
        origin_y: usize,
        region_w: usize,
        region_h: usize,
        cell_w: usize,
        cell_h: usize,
        font_scale: f32,
        view: &TerminalView,
        style: &UiStyle,
        font_config: &FontConfig,
    ) {
        let bar_height = cell_h;
        let bar_y = origin_y;
        let clip_right = (origin_x + region_w).min(width);
        let clip_bottom = (origin_y + region_h).min(height);
        let bar_bottom = (bar_y + bar_height).min(clip_bottom);

        let bar_bg = style.search_bar_bg;
        let text_color = style.search_bar_text;
        let match_info_color = style.search_bar_hint_text;

        for y in bar_y..bar_bottom {
            if y >= clip_bottom {
                break;
            }
            let row = y * width;
            for x in origin_x.min(width)..clip_right {
                buffer[row + x] = bar_bg;
            }
        }

        let prefix = "Find: ";
        let mut x_pos = origin_x + cell_w / 2;
        for ch in prefix.chars() {
            if x_pos + cell_w > clip_right {
                break;
            }
            if let Ok(Some(tt_glyph)) = self.font_renderer.get_glyph(ch, font_config) {
                let baseline_offset = self.font_renderer.baseline_offset(font_config);
                let fixed_bearing = self.font_renderer.max_bearing_y(font_config);
                let glyph_y = bar_y
                    .saturating_add(baseline_offset as usize)
                    .saturating_sub(fixed_bearing as usize);
                self.draw_rasterized_glyph(
                    buffer,
                    width,
                    height,
                    origin_x,
                    origin_y,
                    region_w,
                    region_h,
                    x_pos,
                    glyph_y,
                    &tt_glyph.pixels,
                    tt_glyph.width,
                    tt_glyph.height,
                    fixed_bearing,
                    text_color,
                );
            } else {
                let glyph = font::get_bitmap_glyph(ch);
                self.draw_glyph(
                    buffer,
                    width,
                    height,
                    origin_x,
                    origin_y,
                    region_w,
                    region_h,
                    font_scale,
                    x_pos, bar_y, glyph, text_color,
                );
            }
            x_pos += cell_w;
        }

        for ch in view.search.query().chars() {
            if x_pos + cell_w > clip_right.saturating_sub(style.search_right_reserved_px) {
                break;
            }
            if let Ok(Some(tt_glyph)) = self.font_renderer.get_glyph(ch, font_config) {
                let baseline_offset = self.font_renderer.baseline_offset(font_config);
                let fixed_bearing = self.font_renderer.max_bearing_y(font_config) as usize;
                let glyph_y = bar_y
                    .saturating_add(baseline_offset as usize)
                    .saturating_sub(fixed_bearing);
                self.draw_native_glyph(
                    buffer,
                    width,
                    height,
                    x_pos,
                    glyph_y,
                    &tt_glyph.pixels,
                    tt_glyph.width,
                    tt_glyph.height,
                    text_color,
                );
            } else {
                let glyph = font::get_bitmap_glyph(ch);
                self.draw_glyph(
                    buffer,
                    width,
                    height,
                    origin_x,
                    origin_y,
                    region_w,
                    region_h,
                    font_scale,
                    x_pos, bar_y, glyph, text_color,
                );
            }
            x_pos += cell_w;
        }

        let cursor_x = x_pos;
        for y in bar_y..bar_bottom {
            if cursor_x < clip_right {
                buffer[y * width + cursor_x] = text_color;
            }
        }

        let match_count = view.search.match_count();
        let current_idx = view.search.current_match_index();
        let case_indicator = if view.search.is_case_sensitive() {
            "Aa"
        } else {
            "aa"
        };
        let regex_indicator = if view.is_search_regex() { ".*" } else { "" };

        let match_info = if !view.is_search_regex_valid() {
            format!("[{}{}] invalid regex", case_indicator, regex_indicator)
        } else if match_count > 0 {
            format!(
                "{}/{} [{}{}]",
                current_idx + 1,
                match_count,
                case_indicator,
                regex_indicator
            )
        } else if !view.search.query().is_empty() {
            format!("0/0 [{}{}]", case_indicator, regex_indicator)
        } else {
            format!("[{}{}]", case_indicator, regex_indicator)
        };

        let info_color = if !view.is_search_regex_valid() {
            style.search_invalid_regex_text
        } else {
            match_info_color
        };

        let info_width = match_info.len() * cell_w;
        let info_x = clip_right.saturating_sub(info_width + cell_w);

        let mut x_pos = info_x;
        for ch in match_info.chars() {
            if x_pos + cell_w > clip_right {
                break;
            }
            if let Ok(Some(tt_glyph)) = self.font_renderer.get_glyph(ch, font_config) {
                let baseline_offset = self.font_renderer.baseline_offset(font_config);
                let fixed_bearing = self.font_renderer.max_bearing_y(font_config) as usize;
                let glyph_y = bar_y
                    .saturating_add(baseline_offset as usize)
                    .saturating_sub(fixed_bearing);
                self.draw_native_glyph(
                    buffer,
                    width,
                    height,
                    x_pos,
                    glyph_y,
                    &tt_glyph.pixels,
                    tt_glyph.width,
                    tt_glyph.height,
                    info_color,
                );
            } else {
                let glyph = font::get_bitmap_glyph(ch);
                self.draw_glyph(
                    buffer,
                    width,
                    height,
                    origin_x,
                    origin_y,
                    region_w,
                    region_h,
                    font_scale,
                    x_pos, bar_y, glyph, info_color,
                );
            }
            x_pos += cell_w;
        }
    }
}

const HELP_LINE_INDENT: usize = 2;
const HELP_KEY_COL_WIDTH: usize = 16;
const HELP_KEY_GAP: usize = 2;

#[derive(Debug)]
enum HelpLine {
    Header(String),
    Item { key: String, action: String },
    Spacer,
}

fn help_line_len(line: &HelpLine, key_col_width: usize) -> usize {
    match line {
        HelpLine::Header(text) => text.len(),
        HelpLine::Item { key, action } => {
            let key_width = key.len().max(key_col_width);
            HELP_LINE_INDENT + key_width + HELP_KEY_GAP + action.len()
        }
        HelpLine::Spacer => 0,
    }
}

struct HelpBlock {
    lines: Vec<HelpLine>,
    max_len: usize,
    height: usize,
}

impl HelpBlock {
    fn new(lines: Vec<HelpLine>, key_col_width: usize) -> Self {
        let max_len = lines
            .iter()
            .map(|line| help_line_len(line, key_col_width))
            .max()
            .unwrap_or(0);
        let height = lines.len();
        Self {
            lines,
            max_len,
            height,
        }
    }
}

struct HelpLayout {
    scale: usize,
    cell_w: usize,
    cell_h: usize,
    available_cols: usize,
    available_rows: usize,
    use_two_columns: bool,
    content_rows_capacity: usize,
    required_cols: usize,
}

#[derive(Clone, Copy)]
struct BlockPlacement {
    index: usize,
    col: usize,
    row: usize,
}

fn can_fit_blocks(blocks: &[HelpBlock], content_rows_capacity: usize) -> bool {
    if blocks.is_empty() {
        return true;
    }
    if content_rows_capacity == 0 {
        return false;
    }
    blocks
        .iter()
        .all(|block| block.height <= content_rows_capacity)
}

fn block_start_rows(blocks: &[HelpBlock]) -> Vec<usize> {
    let mut starts = Vec::with_capacity(blocks.len());
    let mut row = 0usize;
    for block in blocks {
        starts.push(row);
        row += block.height;
    }
    starts
}

fn find_start_block(blocks: &[HelpBlock], starts: &[usize], scroll: usize) -> usize {
    for (idx, start) in starts.iter().enumerate() {
        let end = start.saturating_add(blocks[idx].height);
        if scroll < end {
            return idx;
        }
    }
    blocks.len().saturating_sub(1)
}

fn layout_blocks(
    blocks: &[HelpBlock],
    start_idx: usize,
    columns: usize,
    content_rows: usize,
) -> Vec<BlockPlacement> {
    if blocks.is_empty() || content_rows == 0 {
        return Vec::new();
    }

    let mut placements = Vec::new();
    let mut col = 0usize;
    let mut row = 0usize;

    for (idx, block) in blocks.iter().enumerate().skip(start_idx) {
        if block.height > content_rows {
            if placements.is_empty() {
                placements.push(BlockPlacement {
                    index: idx,
                    col,
                    row: 0,
                });
            }
            break;
        }
        if row + block.height > content_rows {
            col += 1;
            row = 0;
        }
        if col >= columns {
            break;
        }
        placements.push(BlockPlacement {
            index: idx,
            col,
            row,
        });
        row += block.height;
    }

    placements
}
