use yatmux::renderer::TerminalView;

#[test]
fn test_view_default() {
    let _view = TerminalView::new();
}

#[test]
fn test_window_to_cell() {
    let mut view = TerminalView::new();
    view.set_dimensions(24, 80);

    let cell = 8 * 2;
    assert_eq!(view.window_to_cell(0.0, 0.0, cell, cell), Some((0, 0)));
    assert_eq!(view.window_to_cell(8.0, 8.0, cell, cell), Some((0, 0)));
    assert_eq!(view.window_to_cell(17.0, 17.0, cell, cell), Some((1, 1)));
}

#[test]
fn test_window_to_cell_out_of_bounds() {
    let mut view = TerminalView::new();
    view.set_dimensions(24, 80);
    let cell = 8 * 2;

    assert_eq!(view.window_to_cell(10000.0, 10000.0, cell, cell), None);
}
