use super::Renderer;

impl Renderer {
    pub(super) fn draw_url_hover_underline(
        &self,
        backbuffer: &mut [u32],
        width: usize,
        clip_right: usize,
        clip_bottom: usize,
        x0: usize,
        y0: usize,
        cell_w: usize,
        cell_h: usize,
        fg: u32,
    ) {
        let underline_y = y0 + cell_h - 2;
        if underline_y < clip_bottom {
            for x in x0..(x0 + cell_w).min(clip_right) {
                backbuffer[underline_y * width + x] = fg;
            }
        }
    }
}
