use super::super::layout::{
    block_start_rows, can_fit_blocks, find_start_block, layout_blocks, BlockPlacement, HelpBlock,
    HelpLayout, HelpLine, HELP_KEY_COL_WIDTH_CELLS,
};
use crate::config::FontConfig;
use crate::renderer::{HelpSection, UiStyle};
use crate::renderer::Renderer;

impl Renderer {
    #[allow(clippy::too_many_arguments)]
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
        shell_warning_dismissed: bool,
        style: &UiStyle,
        font_config: &FontConfig,
    ) -> (usize, usize) {
        let padding_cells_x = style.help_padding_x_cells;
        let padding_cells_y = style.help_padding_y_cells;

        let fixed_lines = if let Some(ref query) = filter_query {
            vec![
                format!("{} — Filter: {}▎ ({} matches)", title, query, match_count.unwrap_or(0)),
                String::new(),
            ]
        } else {
            vec![title.to_string(), String::new()]
        };
        let mut blocks: Vec<HelpBlock> = Vec::new();

        let footer_lines: Vec<String> = if shell_integration_detected || shell_warning_dismissed {
            Vec::new()
        } else {
            vec![
                String::new(),
                "Shell integration not detected (press 'd' to dismiss)".to_string(),
                "Source scripts/shell/yatmux.bash in your shell".to_string(),
            ]
        };

        let max_key_len = sections
            .iter()
            .flat_map(|section| section.bindings.iter().map(|(key, _)| key.len()))
            .max()
            .unwrap_or(0);
        let key_col_width = HELP_KEY_COL_WIDTH_CELLS.max(max_key_len);

        let mut max_line_len = title.len();
        for line in &fixed_lines {
            max_line_len = max_line_len.max(line.len());
        }
        for line in &footer_lines {
            max_line_len = max_line_len.max(line.len());
        }

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

                if filter_query.is_none() {
                    max_line_len = max_line_len.max(section.title.len());
                    lines.push(HelpLine::Header(section.title.clone()));
                }

                for (key, action) in &section.bindings {
                    let action_text = if filter_query.is_some() {
                        format!("[{}] {}", section.title, action)
                    } else {
                        action.to_string()
                    };
                    lines.push(HelpLine::Item {
                        key: key.clone(),
                        action: action_text,
                    });
                }

                if filter_query.is_none() && section_idx + 1 < sections.len() {
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
            let use_two_columns = can_use_two_columns && filter_query.is_none();
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

        let box_cols = layout.available_cols * 95 / 100;
        let total_rows =
            fixed_lines.len() + content_rows_needed + footer_lines.len() + padding_cells_y * 2;
        let box_rows = total_rows.min(layout.available_rows * 95 / 100);

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
                blocks.len(),
                placements.len(),
                max_scroll_idx,
            )
        } else {
            let total_rows = block_starts
                .last()
                .map(|start| start + blocks.last().map(|b| b.height).unwrap_or(0))
                .unwrap_or(0);
            let max_scroll = total_rows.saturating_sub(content_rows);
            let scroll = scroll_offset.min(max_scroll);
            let start_idx = find_start_block(&blocks, &block_starts, scroll);
            let visible_rows = content_rows;
            let scroll_total = total_rows;
            let scroll_visible = visible_rows;
            (scroll, start_idx, scroll_total, scroll_visible, max_scroll)
        };

        let content_x = origin_x + padding_cells_x * cell_w;
        let content_y = origin_y + padding_cells_y * cell_h + fixed_lines.len() * cell_h;
        let content_w = box_w.saturating_sub(padding_cells_x * 2 * cell_w);
        let content_h = content_rows * cell_h;

        for (idx, line) in fixed_lines.iter().enumerate() {
            let y = origin_y + padding_cells_y * cell_h + idx * cell_h;
            self.draw_help_text_at(
                backbuffer,
                buffer_width,
                buffer_height,
                content_x,
                origin_x + box_w - padding_cells_x * cell_w,
                y,
                line,
                style.help_text,
                &help_font,
                cell_w,
                font_scale,
            );
        }

        let placements: Vec<BlockPlacement> = if use_two_columns {
            layout_blocks(&blocks, start_idx, 2, content_rows)
        } else {
            let mut placements = Vec::new();
            let mut row = 0usize;
            for (idx, block) in blocks.iter().enumerate().skip(start_idx) {
                if row >= content_rows {
                    break;
                }
                placements.push(BlockPlacement {
                    index: idx,
                    col: 0,
                    row,
                });
                row += block.height;
            }
            placements
        };

        for placement in placements {
            let block = &blocks[placement.index];
            let col_w = if use_two_columns {
                (content_w.saturating_sub(gutter_cells * cell_w)) / 2
            } else {
                content_w
            };
            let x0 = content_x + placement.col * (col_w + gutter_cells * cell_w);
            let y0 = content_y + placement.row.saturating_mul(cell_h);

            for (line_idx, line) in block.lines.iter().enumerate() {
                let y = y0 + line_idx * cell_h;
                if y >= content_y + content_h {
                    break;
                }
                let line_y = y;
                self.draw_help_line(
                    backbuffer,
                    buffer_width,
                    buffer_height,
                    x0,
                    x0 + col_w,
                    line_y,
                    line,
                    style.help_text,
                    style.help_text,
                    &help_font,
                    cell_w,
                    font_scale,
                    key_col_width,
                );
            }
        }

        let footer_start = origin_y
            + padding_cells_y * cell_h
            + fixed_lines.len() * cell_h
            + content_rows * cell_h;
        for (idx, line) in footer_lines.iter().enumerate() {
            let y = footer_start + idx * cell_h;
            self.draw_help_text_at(
                backbuffer,
                buffer_width,
                buffer_height,
                content_x,
                origin_x + box_w - padding_cells_x * cell_w,
                y,
                line,
                style.help_footer_text,
                &help_font,
                cell_w,
                font_scale,
            );
        }

        if max_scroll > 0 {
            self.draw_help_scrollbar(
                backbuffer,
                buffer_width,
                buffer_height,
                origin_x,
                origin_y,
                box_w,
                box_h,
                scroll,
                scroll_total,
                scroll_visible,
                style,
            );
        }

        (scroll, max_scroll)
    }
}
