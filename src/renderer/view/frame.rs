use anyhow::Result;

use crate::core::grid::RowSnapshot;
use crate::terminal::Terminal;

use super::TerminalView;

use super::super::RenderFrame;

impl TerminalView {
    pub(crate) fn build_frame(
        &mut self,
        terminal: &Terminal,
        rows: usize,
        cols: usize,
    ) -> Result<RenderFrame> {
        let buffer_len = terminal.buffer_len();
        let terminal_generation = terminal.generation();

        // Maintain scroll position when new output arrives.
        if self.scroll_offset > 0 {
            let new_lines = buffer_len.saturating_sub(self.last_buffer_len);
            if new_lines > 0 {
                self.scroll_offset = (self.scroll_offset + new_lines).min(self.max_scroll_offset());
            }
        }
        self.last_buffer_len = buffer_len;

        // Clamp scroll offset to valid range.
        self.scroll_offset = self
            .scroll_offset
            .min(self.max_scroll_offset_with_len(buffer_len));

        // Visible window start in absolute coordinates.
        let window_start = buffer_len.saturating_sub(rows + self.scroll_offset);

        let mut display_rows = terminal.rows_in_range(window_start, rows, cols);
        if display_rows.len() < rows {
            let mut padded = Vec::with_capacity(rows);
            for _ in 0..(rows - display_rows.len()) {
                padded.push(RowSnapshot::blank(cols));
            }
            padded.extend(display_rows);
            display_rows = padded;
        }

        self.selection
            .set_scroll_state(self.scroll_offset, buffer_len);

        // Cache display rows for copy operations.
        self.last_display_rows = display_rows.clone();

        // Update search matches when search is active and inputs changed.
        if self.search.is_active() {
            let query = self.search.query().to_string();
            let case_sensitive = self.search.is_case_sensitive();
            let regex_mode = self.search.mode() == crate::core::search::SearchMode::Regex;

            if query != self.last_search_query
                || terminal_generation != self.last_search_terminal_generation
                || case_sensitive != self.last_search_case_sensitive
                || regex_mode != self.last_search_regex_mode
            {
                let all_rows = terminal.rows_in_range(0, buffer_len, cols);
                self.search.update_matches(&all_rows);
                self.last_search_query = query;
                self.last_search_terminal_generation = terminal_generation;
                self.last_search_case_sensitive = case_sensitive;
                self.last_search_regex_mode = regex_mode;
            }
        }

        // Detect URLs and hex color codes in each visible row.
        for (row_idx, row_data) in display_rows.iter().enumerate() {
            let text: String = row_data.cells.iter().map(|(ch, _, _)| ch).collect();
            self.urls
                .update_row_with_hyperlinks(row_idx, &text, &row_data.hyperlinks);
            self.color_codes.update_row(row_idx, &text);
        }

        let (cursor, cursor_visible) = terminal.cursor();

        Ok(RenderFrame {
            cursor,
            display_rows,
            rows,
            cols,
            view_start: window_start,
            show_cursor: self.scroll_offset == 0 && cursor_visible,
        })
    }
}
