//! Help overlay rendering.
//!
//! This module contains the rendering logic for the help overlay popup
//! that displays keyboard shortcuts and other help information.

use super::HelpSection;
use super::font;

/// Paint a help overlay popup centered on the screen.
///
/// Returns `(current_scroll, max_scroll)` for tracking scroll state.
pub fn paint_help_overlay(
    backbuffer: &mut [u32],
    buffer_width: usize,
    buffer_height: usize,
    title: &str,
    sections: &[HelpSection],
    scroll_offset: usize,
    accent_color: u32,
    font_scale: usize,
    shell_integration_detected: bool,
    bg: u32,
    text_color: u32,
    footer_color: u32,
    padding_cells_x: usize,
    padding_cells_y: usize,
) -> (usize, usize) {
    let font_scale = font_scale.clamp(1, 8);
    let cell_w = 8 * font_scale;
    let cell_h = 8 * font_scale;

    if buffer_width < cell_w * 10 || buffer_height < cell_h * 5 {
        return (0, 0);
    }

    let padding_cells_x = padding_cells_x;
    let padding_cells_y = padding_cells_y;

    let fixed_lines = vec![title.to_string(), String::new()];
    let mut content_lines: Vec<String> = Vec::new();

    // Footer lines (shown at the bottom, outside scrollable area)
    let footer_lines: Vec<String> = if shell_integration_detected {
        Vec::new()
    } else {
        vec![
            String::new(),
            "Shell integration not detected".to_string(),
            "Source scripts/shell/yatmux.bash in your shell".to_string(),
        ]
    };

    let mut max_line_len = title.len();
    for line in &footer_lines {
        max_line_len = max_line_len.max(line.len());
    }
    for (section_idx, section) in sections.iter().enumerate() {
        if section_idx > 0 {
            content_lines.push(String::new());
        }

        max_line_len = max_line_len.max(section.title.len());
        content_lines.push(section.title.clone());

        // Render like: "  ctrl+shift+/  Toggle help"
        for (key, action) in &section.bindings {
            let line = format!("  {:<16} {}", key, action);
            max_line_len = max_line_len.max(line.len());
            content_lines.push(line);
        }
    }

    let box_cols = (max_line_len + padding_cells_x * 2).min(buffer_width / cell_w);
    let total_rows =
        fixed_lines.len() + content_lines.len() + footer_lines.len() + padding_cells_y * 2;
    let box_rows = total_rows.min(buffer_height / cell_h);

    let box_w = box_cols * cell_w;
    let box_h = box_rows * cell_h;
    let origin_x = buffer_width.saturating_sub(box_w) / 2;
    let origin_y = buffer_height.saturating_sub(box_h) / 2;

    // Background
    let bg = bg;
    let border = accent_color;

    for y in origin_y..(origin_y + box_h).min(buffer_height) {
        let row = y * buffer_width;
        for x in origin_x..(origin_x + box_w).min(buffer_width) {
            backbuffer[row + x] = bg;
        }
    }

    // Border
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
    let max_scroll = content_lines.len().saturating_sub(content_rows);
    let scroll = scroll_offset.min(max_scroll);

    // Scrollbar (only when there is overflow)
    if max_scroll > 0 && content_rows > 0 {
        draw_scrollbar(
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
            content_lines.len(),
            content_rows,
            scroll,
            max_scroll,
            accent_color,
            footer_color,
        );
    }

    // Text
    let text_color = text_color;
    let mut y = origin_y + padding_cells_y * cell_h;

    for (idx, line) in fixed_lines.iter().enumerate() {
        if idx + padding_cells_y >= box_rows {
            break;
        }
        draw_text_line(
            backbuffer,
            buffer_width,
            buffer_height,
            origin_x,
            origin_y,
            box_w,
            box_h,
            cell_w,
            padding_cells_x,
            font_scale,
            y,
            line,
            text_color,
        );
        y += cell_h;
    }

    for (idx, line) in content_lines
        .iter()
        .skip(scroll)
        .take(content_rows)
        .enumerate()
    {
        let overall_idx = idx + fixed_lines.len();
        if overall_idx + padding_cells_y >= box_rows {
            break;
        }
        draw_text_line(
            backbuffer,
            buffer_width,
            buffer_height,
            origin_x,
            origin_y,
            box_w,
            box_h,
            cell_w,
            padding_cells_x,
            font_scale,
            y,
            line,
            text_color,
        );
        y += cell_h;
    }

    // Footer (shell integration hint)
    let footer_color = footer_color;
    for line in &footer_lines {
        draw_text_line(
            backbuffer,
            buffer_width,
            buffer_height,
            origin_x,
            origin_y,
            box_w,
            box_h,
            cell_w,
            padding_cells_x,
            font_scale,
            y,
            line,
            footer_color,
        );
        y += cell_h;
    }

    (scroll, max_scroll)
}

#[allow(clippy::too_many_arguments)]
fn draw_scrollbar(
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

    // Draw inside the right padding area.
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

#[allow(clippy::too_many_arguments)]
fn draw_text_line(
    backbuffer: &mut [u32],
    buffer_width: usize,
    buffer_height: usize,
    origin_x: usize,
    origin_y: usize,
    box_w: usize,
    box_h: usize,
    cell_w: usize,
    padding_cells_x: usize,
    font_scale: usize,
    y: usize,
    line: &str,
    text_color: u32,
) {
    let mut x = origin_x + padding_cells_x * cell_w;
    for ch in line.chars() {
        if x + cell_w > origin_x + box_w - padding_cells_x * cell_w {
            break;
        }
        let glyph = font::get_glyph(ch);
        draw_glyph_help(
            backbuffer,
            buffer_width,
            buffer_height,
            origin_x,
            origin_y,
            box_w,
            box_h,
            font_scale,
            x,
            y,
            glyph,
            text_color,
        );
        x += cell_w;
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_glyph_help(
    backbuffer: &mut [u32],
    buffer_width: usize,
    buffer_height: usize,
    origin_x: usize,
    origin_y: usize,
    box_w: usize,
    box_h: usize,
    font_scale: usize,
    x0: usize,
    y0: usize,
    glyph: [u8; 8],
    color: u32,
) {
    let clip_right = (origin_x + box_w).min(buffer_width);
    let clip_bottom = (origin_y + box_h).min(buffer_height);

    let font_scale = font_scale.clamp(1, 8);

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
