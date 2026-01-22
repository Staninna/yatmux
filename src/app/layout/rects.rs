/// A rectangle representing a region in screen space.
#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

impl Rect {
    /// Returns true if the point (x, y) is contained within this rectangle.
    pub fn contains(&self, x: f64, y: f64) -> bool {
        let x = x as isize;
        let y = y as isize;
        if x < 0 || y < 0 {
            return false;
        }
        let x = x as usize;
        let y = y as usize;
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// Computes the 1D overlap between two ranges.
pub fn overlap_1d(a0: usize, a_len: usize, b0: usize, b_len: usize) -> usize {
    let a1 = a0 + a_len;
    let b1 = b0 + b_len;
    let start = a0.max(b0);
    let end = a1.min(b1);
    end.saturating_sub(start)
}

/// Fills a rectangle with a solid color.
pub fn fill_rect(
    buffer: &mut [u32],
    buffer_width: usize,
    buffer_height: usize,
    rect: Rect,
    color: u32,
) {
    let x1 = (rect.x + rect.w).min(buffer_width);
    let y1 = (rect.y + rect.h).min(buffer_height);

    for y in rect.y.min(buffer_height)..y1 {
        let row = y * buffer_width;
        for x in rect.x.min(buffer_width)..x1 {
            buffer[row + x] = color;
        }
    }
}

/// Draws a 1px border around a rectangle.
pub fn draw_border(
    buffer: &mut [u32],
    buffer_width: usize,
    buffer_height: usize,
    rect: Rect,
    color: u32,
) {
    if rect.w == 0 || rect.h == 0 {
        return;
    }

    let x0 = rect.x.min(buffer_width);
    let y0 = rect.y.min(buffer_height);
    let x1 = (rect.x + rect.w).min(buffer_width);
    let y1 = (rect.y + rect.h).min(buffer_height);

    if x1 <= x0 || y1 <= y0 {
        return;
    }

    // Top and bottom
    for x in x0..x1 {
        buffer[y0 * buffer_width + x] = color;
        buffer[(y1 - 1) * buffer_width + x] = color;
    }

    // Left and right
    for y in y0..y1 {
        buffer[y * buffer_width + x0] = color;
        buffer[y * buffer_width + (x1 - 1)] = color;
    }
}
