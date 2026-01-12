use yatmux::core::color_codes::ColorCodeManager;

#[test]
fn parses_rgb() {
    let mut m = ColorCodeManager::new();
    m.set_dimensions(1);

    m.update_row(0, "#fff");
    assert_eq!(m.color_at(0, 0), Some(0xFFFFFF));

    m.update_row(0, "#0aF");
    assert_eq!(m.color_at(0, 0), Some(0x00AAFF));
}

#[test]
fn parses_rrggbb() {
    let mut m = ColorCodeManager::new();
    m.set_dimensions(1);

    m.update_row(0, "#ff0000");
    assert_eq!(m.color_at(0, 0), Some(0xFF0000));

    m.update_row(0, "#00ff88");
    assert_eq!(m.color_at(0, 0), Some(0x00FF88));
}

#[test]
fn requires_boundary_after_color() {
    let mut m = ColorCodeManager::new();
    m.set_dimensions(1);

    m.update_row(0, "#ffffff0");
    assert_eq!(m.color_at(0, 0), None);

    m.update_row(0, "#fff0");
    assert_eq!(m.color_at(0, 0), None);

    m.update_row(0, "#fff ");
    assert_eq!(m.color_at(0, 0), Some(0xFFFFFF));

    m.update_row(0, "#ffffff,");
    assert_eq!(m.color_at(0, 0), Some(0xFFFFFF));
}

#[test]
fn manager_tracks_spans() {
    let mut m = ColorCodeManager::new();
    m.set_dimensions(2);
    m.update_row(0, "bg = #101010 fg=#D0D0D0");

    assert_eq!(m.color_at(0, 5), Some(0x101010));
    assert_eq!(m.color_at(0, 10), Some(0x101010));
    assert_eq!(m.color_at(0, 16), Some(0xD0D0D0));
    assert_eq!(m.color_at(0, 0), None);
    assert_eq!(m.color_at(1, 0), None);
}
