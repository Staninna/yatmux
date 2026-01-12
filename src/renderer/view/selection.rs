use super::TerminalView;

impl TerminalView {
    pub fn start_selection(&mut self, row: usize, col: usize) {
        self.selection.start(row, col);
    }

    pub fn update_selection(&mut self, row: usize, col: usize) {
        self.selection.update(row, col);
    }

    pub fn window_to_cell(
        &self,
        x: f64,
        y: f64,
        cell_w: usize,
        cell_h: usize,
    ) -> Option<(usize, usize)> {
        if self.view_rows == 0 || self.view_cols == 0 {
            return None;
        }

        let cell_w = cell_w.max(1);
        let cell_h = cell_h.max(1);

        let col = (x as usize) / cell_w;
        let row = (y as usize) / cell_h;

        if row >= self.view_rows || col >= self.view_cols {
            return None;
        }

        Some((row, col))
    }

    pub fn get_selection_bounds(&self) -> Option<((usize, usize), (usize, usize))> {
        self.selection.bounds()
    }

    pub fn get_selected_text(&self) -> Option<String> {
        let ((start_row, start_col), (end_row, end_col)) = self.selection.visible_bounds()?;

        if self.last_display_rows.is_empty() {
            return None;
        }

        let mut text = String::new();

        for row in start_row..=end_row {
            if row >= self.last_display_rows.len() {
                break;
            }

            let row_data = &self.last_display_rows[row];
            let row_start = if row == start_row { start_col } else { 0 };
            let row_end = if row == end_row {
                (end_col + 1).min(row_data.cells.len())
            } else {
                row_data.cells.len()
            };

            for col in row_start..row_end {
                if let Some((ch, _, _)) = row_data.cells.get(col) {
                    text.push(*ch);
                }
            }

            if row != end_row {
                text.push('\n');
            }
        }

        let trimmed: String = text
            .lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n");

        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }

    pub fn display_rows_len(&self) -> usize {
        self.last_display_rows.len()
    }

    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }

    pub fn has_selection(&self) -> bool {
        self.selection.has_selection()
    }

    pub fn select_all(&mut self) {
        self.selection.select_all();
    }
}
