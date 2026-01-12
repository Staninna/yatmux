//! Scrollback buffer tests.
//!
//! These tests verify the scrollback buffer functionality including:
//! - History management
//! - Scrolling behavior
//! - Display row generation
//! - Resize handling

mod common;

use common::{make_row, make_rows};
use term::core::grid::RowSnapshot;
use term::core::scrollback::ScrollbackBuffer;

#[test]
fn test_scrollback_buffer_new() {
    let buffer = ScrollbackBuffer::new();
    assert!(buffer.is_empty());
    assert_eq!(buffer.offset(), 0);
}

#[test]
fn test_add_history_rows() {
    let mut buffer = ScrollbackBuffer::with_capacity(100);
    buffer.set_dimensions(3, 80);

    // Simulate adding rows that scrolled off
    buffer.add_history_rows(make_rows(&["Line 1", "Line 2"]), 2);

    assert_eq!(buffer.len(), 2);
    assert_eq!(buffer.last_vt100_scrollback_len(), 2);
}

#[test]
fn test_scroll_by() {
    let mut buffer = ScrollbackBuffer::with_capacity(100);
    buffer.set_dimensions(3, 80);

    // Add some history
    buffer.add_history_rows(
        make_rows(&["Line 1", "Line 2", "Line 3", "Line 4", "Line 5"]),
        5,
    );

    assert_eq!(buffer.len(), 5);

    // Scroll up
    buffer.scroll_by(1);
    assert_eq!(buffer.offset(), 1);

    buffer.scroll_by(2);
    assert_eq!(buffer.offset(), 3);

    // Can't scroll past history
    buffer.scroll_by(100);
    assert_eq!(buffer.offset(), 5);

    // Scroll back down
    buffer.scroll_by(-2);
    assert_eq!(buffer.offset(), 3);

    buffer.scroll_by(-100);
    assert_eq!(buffer.offset(), 0);
}

#[test]
fn test_view_start() {
    let mut buffer = ScrollbackBuffer::with_capacity(100);
    buffer.set_dimensions(3, 80);

    // Add history
    buffer.add_history_rows(make_rows(&[""; 10]), 10);
    assert_eq!(buffer.len(), 10);

    // At live view
    assert_eq!(buffer.view_start(), 10);

    // Scrolled up by 3
    buffer.scroll_by(3);
    assert_eq!(buffer.view_start(), 7);

    // Scrolled to top
    buffer.scroll_by(100);
    assert_eq!(buffer.view_start(), 0);
}

#[test]
fn test_get_display_rows_live() {
    let mut buffer = ScrollbackBuffer::with_capacity(100);
    buffer.set_dimensions(3, 80);

    let live_rows = make_rows(&["Live 1", "Live 2", "Live 3"]);

    // No history, offset 0 - should return live rows (padded to display width)
    let display = buffer.get_display_rows(&live_rows, 80);
    assert_eq!(display.len(), 3);
    assert_eq!(display[0].text().trim_end(), "Live 1");
    assert_eq!(display[0].cells.len(), 80); // Padded to display width
}

#[test]
fn test_get_display_rows_scrolled() {
    let mut buffer = ScrollbackBuffer::with_capacity(100);
    buffer.set_dimensions(3, 80);

    // Add history
    buffer.add_history_rows(make_rows(&["Hist 1", "Hist 2"]), 2);

    // Scroll up
    buffer.scroll_by(1);

    let live_rows = make_rows(&["Live 1", "Live 2", "Live 3"]);
    let display = buffer.get_display_rows(&live_rows, 80);

    // Should show part history, part live
    assert_eq!(display.len(), 3);
    assert_eq!(display[0].text().trim_end(), "Hist 2");
    assert_eq!(display[1].text().trim_end(), "Live 1");
    assert_eq!(display[2].text().trim_end(), "Live 2");
}

#[test]
fn test_get_all_rows() {
    let mut buffer = ScrollbackBuffer::with_capacity(100);
    buffer.set_dimensions(3, 80);

    // Add history
    buffer.add_history_rows(make_rows(&["Hist 1", "Hist 2"]), 2);

    let live_rows = make_rows(&["Live 1", "Live 2"]);
    let all = buffer.get_all_rows(&live_rows);

    assert_eq!(all.len(), 4);
    assert_eq!(all[0].text().trim_end(), "Hist 1");
    assert_eq!(all[1].text().trim_end(), "Hist 2");
    assert_eq!(all[2].text().trim_end(), "Live 1");
    assert_eq!(all[3].text().trim_end(), "Live 2");
}

#[test]
fn test_clear() {
    let mut buffer = ScrollbackBuffer::with_capacity(100);
    buffer.add_history_rows(make_rows(&[""; 5]), 5);
    buffer.scroll_by(3);

    buffer.clear();
    assert!(buffer.is_empty());
    assert_eq!(buffer.offset(), 0);
}

#[test]
fn test_capacity_limit() {
    let mut buffer = ScrollbackBuffer::with_capacity(3);
    buffer.set_dimensions(3, 80);

    // Add more history than capacity
    buffer.add_history_rows(
        make_rows(&["Line 1", "Line 2", "Line 3", "Line 4", "Line 5"]),
        5,
    );

    // Only 3 lines kept
    assert_eq!(buffer.len(), 3);

    // Should have the last 3
    let live_rows = make_rows(&[""]);
    let all = buffer.get_all_rows(&live_rows);
    assert_eq!(all[0].text().trim_end(), "Line 3");
    assert_eq!(all[1].text().trim_end(), "Line 4");
    assert_eq!(all[2].text().trim_end(), "Line 5");
}

#[test]
fn test_offset_maintained_when_scrolled_up() {
    let mut buffer = ScrollbackBuffer::with_capacity(100);
    buffer.set_dimensions(3, 80);

    // Add initial history
    buffer.add_history_rows(make_rows(&["Line 1", "Line 2", "Line 3"]), 3);

    // User scrolls up
    buffer.scroll_by(2);
    assert_eq!(buffer.offset(), 2);

    // More content scrolls off
    buffer.add_history_rows(make_rows(&["Line 4", "Line 5"]), 5);

    // Offset should increase to maintain view position
    assert_eq!(buffer.offset(), 4);
}

#[test]
fn test_offset_not_changed_when_at_live() {
    let mut buffer = ScrollbackBuffer::with_capacity(100);
    buffer.set_dimensions(3, 80);

    // Add history
    buffer.add_history_rows(make_rows(&[""; 3]), 3);

    // At live view (offset = 0)
    assert_eq!(buffer.offset(), 0);

    // More history
    buffer.add_history_rows(make_rows(&[""; 2]), 5);

    // Offset should stay at 0
    assert_eq!(buffer.offset(), 0);
}

#[test]
fn test_row_snapshot() {
    let row = make_row("Hello");
    assert_eq!(row.text(), "Hello");

    let blank = RowSnapshot::blank(10);
    assert_eq!(blank.cells.len(), 10);
    assert_eq!(blank.text(), "          ");
}

#[test]
fn test_snap_to_bottom() {
    let mut buffer = ScrollbackBuffer::new();

    // Add some history
    buffer.add_history_rows(vec![make_row("Line 1"), make_row("Line 2")], 2);

    // Scroll up
    buffer.scroll_by(2);
    assert!(buffer.is_scrolled_up());
    assert_eq!(buffer.offset(), 2);

    // Snap to bottom
    buffer.snap_to_bottom();
    assert!(!buffer.is_scrolled_up());
    assert_eq!(buffer.offset(), 0);
}

#[test]
fn test_scroll_to_row() {
    let mut buffer = ScrollbackBuffer::new();
    buffer.set_dimensions(24, 80); // 24 rows visible

    // Add 100 history rows
    let history: Vec<_> = (0..100)
        .map(|i| make_row(&format!("history {}", i)))
        .collect();
    buffer.add_history_rows(history, 100);

    // 24 live rows (indices 100-123 in the combined buffer)
    let live_rows_len = 24;

    // Scroll to row 50 (in history)
    buffer.scroll_to_row(50, live_rows_len);

    // Row 50 should be visible (roughly in the middle)
    let view_start = buffer.view_start();
    let view_end = view_start + 24;

    assert!(
        view_start <= 50 && 50 < view_end,
        "Row 50 should be visible. view_start={}, view_end={}, offset={}",
        view_start,
        view_end,
        buffer.offset()
    );

    // Scroll to row 5 (near the beginning)
    buffer.scroll_to_row(5, live_rows_len);
    let view_start = buffer.view_start();
    let view_end = view_start + 24;
    assert!(
        view_start <= 5 && 5 < view_end,
        "Row 5 should be visible. view_start={}, view_end={}",
        view_start,
        view_end
    );

    // Scroll to row 120 (in live area)
    buffer.scroll_to_row(120, live_rows_len);
    let view_start = buffer.view_start();
    let view_end = view_start + 24;
    assert!(
        view_start <= 120 && 120 < view_end,
        "Row 120 should be visible. view_start={}, view_end={}",
        view_start,
        view_end
    );
}

#[test]
fn test_resize_preserves_history_data() {
    let mut buffer = ScrollbackBuffer::new();
    buffer.set_dimensions(24, 40); // Start with 40 columns

    // Add history with different widths
    buffer.add_history_rows(
        vec![
            make_row("Hello World"),
            make_row("This is a longer line that has more text"),
        ],
        2,
    );

    // Verify initial state - rows keep their original width
    assert_eq!(buffer.len(), 2);
    let live = make_rows(&["Live"]);
    let all = buffer.get_all_rows(&live);
    assert_eq!(all[0].cells.len(), 11); // "Hello World" = 11 chars
    assert_eq!(all[1].cells.len(), 40); // 40 chars

    // Resize to 80 columns - original data unchanged
    buffer.set_dimensions(24, 80);
    let all = buffer.get_all_rows(&live);
    assert_eq!(all[0].cells.len(), 11); // Still original width
    assert_eq!(all[1].cells.len(), 40);
    assert_eq!(all[0].text(), "Hello World");

    // But display rows are adjusted to display width
    buffer.scroll_by(2);
    let display = buffer.get_display_rows(&live, 80);
    assert_eq!(display[0].cells.len(), 80); // Padded for display
    assert_eq!(display[0].text().trim_end(), "Hello World");

    // Resize to 20 columns - original data STILL preserved
    buffer.set_dimensions(24, 20);
    let all = buffer.get_all_rows(&live);
    assert_eq!(all[0].cells.len(), 11); // Original preserved
    assert_eq!(all[1].cells.len(), 40); // Original preserved - NOT truncated!
    assert_eq!(all[1].text(), "This is a longer line that has more text");

    // But display is truncated
    let display = buffer.get_display_rows(&live, 20);
    assert_eq!(display[0].cells.len(), 20);
    assert_eq!(display[1].cells.len(), 20);
    assert_eq!(display[1].text(), "This is a longer lin"); // Truncated for display only

    // Resize back to 80 - data is RESTORED because we never truncated it
    buffer.set_dimensions(24, 80);
    let all = buffer.get_all_rows(&live);
    assert_eq!(all[1].text(), "This is a longer line that has more text");
}

#[test]
fn test_resize_preserves_scroll_offset() {
    let mut buffer = ScrollbackBuffer::new();
    buffer.set_dimensions(24, 80);

    // Add lots of history
    let history: Vec<_> = (0..100).map(|i| make_row(&format!("Line {}", i))).collect();
    buffer.add_history_rows(history, 100);

    // Scroll up
    buffer.scroll_by(50);
    assert_eq!(buffer.offset(), 50);

    // Resize - offset should be preserved
    buffer.set_dimensions(30, 100);
    assert_eq!(buffer.offset(), 50);

    // History should still be there
    assert_eq!(buffer.len(), 100);
}
