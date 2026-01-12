use yatmux::core::selection::{CellPos, Selection, SelectionManager};

#[test]
fn test_cell_pos_new() {
    let pos = CellPos::new(5, 10);
    assert_eq!(pos.row, 5);
    assert_eq!(pos.col, 10);
}

#[test]
fn test_selection_new() {
    let sel = Selection::new(CellPos::new(1, 2));
    assert_eq!(sel.start(), sel.end());
}

#[test]
fn test_selection_normalized() {
    let mut sel = Selection::new(CellPos::new(5, 10));
    sel.update_end(CellPos::new(2, 5));
    let (start, end) = sel.normalized();
    assert_eq!(start.row, 2);
    assert_eq!(end.row, 5);
}

#[test]
fn test_selection_contains_single_line() {
    let mut sel = Selection::new(CellPos::new(3, 5));
    sel.update_end(CellPos::new(3, 10));

    assert!(sel.contains(3, 5));
    assert!(sel.contains(3, 7));
    assert!(sel.contains(3, 10));
    assert!(!sel.contains(3, 4));
    assert!(!sel.contains(3, 11));
    assert!(!sel.contains(2, 7));
}

#[test]
fn test_selection_contains_multi_line() {
    let mut sel = Selection::new(CellPos::new(2, 5));
    sel.update_end(CellPos::new(5, 10));

    assert!(sel.contains(2, 5));
    assert!(sel.contains(2, 80));
    assert!(!sel.contains(2, 4));

    assert!(sel.contains(3, 0));
    assert!(sel.contains(4, 50));

    assert!(sel.contains(5, 10));
    assert!(sel.contains(5, 0));
    assert!(!sel.contains(5, 11));

    assert!(!sel.contains(1, 5));
    assert!(!sel.contains(6, 5));
}

#[test]
fn test_selection_manager() {
    let mut mgr = SelectionManager::new();
    mgr.set_dimensions(24, 80);
    mgr.set_scroll_state(0, 24);

    assert!(!mgr.is_selected(5, 5));

    mgr.start(5, 5);
    assert!(mgr.is_selected(5, 5));

    mgr.update(5, 10);
    assert!(mgr.is_selected(5, 7));

    mgr.clear();
    assert!(!mgr.is_selected(5, 7));
}

#[test]
fn test_selection_manager_clamps() {
    let mut mgr = SelectionManager::new();
    mgr.set_dimensions(24, 80);
    mgr.set_scroll_state(0, 24);

    mgr.start(100, 100);
    assert!(mgr.is_selected(23, 79));
}

#[test]
fn test_selection_scrolls_with_content() {
    let mut mgr = SelectionManager::new();
    mgr.set_dimensions(24, 80);
    mgr.set_scroll_state(0, 48);

    mgr.start(10, 5);
    mgr.update(10, 15);

    assert!(mgr.is_selected(10, 10));
    assert!(!mgr.is_selected(9, 10));

    mgr.set_scroll_state(5, 48);

    assert!(!mgr.is_selected(10, 10));
    assert!(mgr.is_selected(15, 10));
}
