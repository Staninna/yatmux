//! Hex color code detection for UI highlighting.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Span {
    start: usize,
    end: usize,
    color: u32,
}

fn is_hex_digit(b: u8) -> bool {
    matches!(b, b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F')
}

fn hex_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn parse_hex_color(bytes: &[u8]) -> Option<(usize, u32)> {
    if bytes.first().copied() != Some(b'#') {
        return None;
    }

    // #RGB
    if bytes.len() >= 4
        && bytes[1..4].iter().all(|&b| is_hex_digit(b))
        && (bytes.len() == 4 || !is_hex_digit(bytes[4]))
    {
        let r = hex_value(bytes[1])?;
        let g = hex_value(bytes[2])?;
        let b = hex_value(bytes[3])?;
        let rr = (r << 4) | r;
        let gg = (g << 4) | g;
        let bb = (b << 4) | b;
        let color = ((rr as u32) << 16) | ((gg as u32) << 8) | (bb as u32);
        return Some((4, color));
    }

    // #RRGGBB
    if bytes.len() >= 7
        && bytes[1..7].iter().all(|&b| is_hex_digit(b))
        && (bytes.len() == 7 || !is_hex_digit(bytes[7]))
    {
        let r = (hex_value(bytes[1])? << 4) | hex_value(bytes[2])?;
        let g = (hex_value(bytes[3])? << 4) | hex_value(bytes[4])?;
        let b = (hex_value(bytes[5])? << 4) | hex_value(bytes[6])?;
        let color = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
        return Some((7, color));
    }

    None
}

/// Tracks hex color codes in the visible rows.
#[derive(Default)]
pub struct ColorCodeManager {
    rows: Vec<Vec<Span>>,
}

impl ColorCodeManager {
    pub fn new() -> Self {
        ColorCodeManager::default()
    }

    pub fn set_dimensions(&mut self, rows: usize) {
        if self.rows.len() != rows {
            self.rows = vec![Vec::new(); rows];
        } else {
            for row in &mut self.rows {
                row.clear();
            }
        }
    }

    pub fn update_row(&mut self, row_idx: usize, text: &str) {
        if row_idx >= self.rows.len() {
            return;
        }

        let bytes = text.as_bytes();
        let mut spans = Vec::new();

        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'#' {
                if let Some((len, color)) = parse_hex_color(&bytes[i..]) {
                    spans.push(Span {
                        start: i,
                        end: i + len,
                        color,
                    });
                    i += len;
                    continue;
                }
            }
            i += 1;
        }

        self.rows[row_idx] = spans;
    }

    pub fn color_at(&self, row_idx: usize, col: usize) -> Option<u32> {
        let spans = self.rows.get(row_idx)?;
        for span in spans {
            if col >= span.start && col < span.end {
                return Some(span.color);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rgb() {
        assert_eq!(parse_hex_color(b"#fff"), Some((4, 0xFFFFFF)));
        assert_eq!(parse_hex_color(b"#0aF"), Some((4, 0x00AAFF)));
    }

    #[test]
    fn parses_rrggbb() {
        assert_eq!(parse_hex_color(b"#ff0000"), Some((7, 0xFF0000)));
        assert_eq!(parse_hex_color(b"#00ff88"), Some((7, 0x00FF88)));
    }

    #[test]
    fn requires_boundary_after_color() {
        assert_eq!(parse_hex_color(b"#ffffff0"), None);
        assert_eq!(parse_hex_color(b"#fff0"), None);
        assert_eq!(parse_hex_color(b"#fff "), Some((4, 0xFFFFFF)));
        assert_eq!(parse_hex_color(b"#ffffff,"), Some((7, 0xFFFFFF)));
    }

    #[test]
    fn manager_tracks_spans() {
        let mut m = ColorCodeManager::new();
        m.set_dimensions(2);
        m.update_row(0, "bg = #101010 fg=#D0D0D0");

        assert_eq!(m.color_at(0, 5), Some(0x101010)); // '#'
        assert_eq!(m.color_at(0, 10), Some(0x101010));
        assert_eq!(m.color_at(0, 16), Some(0xD0D0D0));
        assert_eq!(m.color_at(0, 0), None);
        assert_eq!(m.color_at(1, 0), None);
    }
}
