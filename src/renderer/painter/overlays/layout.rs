const HELP_LINE_INDENT: usize = 2;
const HELP_KEY_COL_WIDTH: usize = 16;
const HELP_KEY_GAP: usize = 2;

#[derive(Debug)]
pub(super) enum HelpLine {
    Header(String),
    Item { key: String, action: String },
    Spacer,
}

pub(super) fn help_line_len(line: &HelpLine, key_col_width: usize) -> usize {
    match line {
        HelpLine::Header(text) => text.len(),
        HelpLine::Item { key, action } => {
            let key_width = key.len().max(key_col_width);
            HELP_LINE_INDENT + key_width + HELP_KEY_GAP + action.len()
        }
        HelpLine::Spacer => 0,
    }
}

pub(super) struct HelpBlock {
    pub(super) lines: Vec<HelpLine>,
    pub(super) max_len: usize,
    pub(super) height: usize,
}

impl HelpBlock {
    pub(super) fn new(lines: Vec<HelpLine>, key_col_width: usize) -> Self {
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

pub(super) struct HelpLayout {
    pub(super) scale: usize,
    pub(super) cell_w: usize,
    pub(super) cell_h: usize,
    pub(super) available_cols: usize,
    pub(super) available_rows: usize,
    pub(super) use_two_columns: bool,
}

#[derive(Clone, Copy)]
pub(super) struct BlockPlacement {
    pub(super) index: usize,
    pub(super) col: usize,
    pub(super) row: usize,
}

pub(super) fn can_fit_blocks(blocks: &[HelpBlock], content_rows_capacity: usize) -> bool {
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

pub(super) fn block_start_rows(blocks: &[HelpBlock]) -> Vec<usize> {
    let mut starts = Vec::with_capacity(blocks.len());
    let mut row = 0usize;
    for block in blocks {
        starts.push(row);
        row += block.height;
    }
    starts
}

pub(super) fn find_start_block(blocks: &[HelpBlock], starts: &[usize], scroll: usize) -> usize {
    for (idx, start) in starts.iter().enumerate() {
        let end = start.saturating_add(blocks[idx].height);
        if scroll < end {
            return idx;
        }
    }
    blocks.len().saturating_sub(1)
}

pub(super) fn layout_blocks(
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

pub(super) const HELP_LINE_INDENT_CELLS: usize = HELP_LINE_INDENT;
pub(super) const HELP_KEY_COL_WIDTH_CELLS: usize = HELP_KEY_COL_WIDTH;
pub(super) const HELP_KEY_GAP_CELLS: usize = HELP_KEY_GAP;
