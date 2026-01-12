//! Search functionality tests.
//!
//! These tests verify search within terminal scrollback buffer:
//! - Match finding (case-sensitive and case-insensitive)
//! - Navigation between matches
//! - Unicode handling
//! - History + live row searching

mod common;

use common::make_row;
use yatmux::core::search::SearchState;

#[test]
fn test_search_state_new() {
    let state = SearchState::new();
    assert!(!state.is_active());
    assert!(state.query().is_empty());
    assert_eq!(state.match_count(), 0);
}

#[test]
fn test_search_activate_deactivate() {
    let mut state = SearchState::new();

    state.activate();
    assert!(state.is_active());

    state.deactivate();
    assert!(!state.is_active());
}

#[test]
fn test_search_find_matches() {
    let mut state = SearchState::new();
    state.activate();
    state.push_char('h');
    state.push_char('e');
    state.push_char('l');
    state.push_char('l');
    state.push_char('o');

    let display_rows = vec![
        make_row("hello world"),
        make_row("hello again"),
        make_row("goodbye"),
    ];

    state.update_matches(&display_rows);

    assert_eq!(state.match_count(), 2);
    assert_eq!(state.matches()[0].row, 0);
    assert_eq!(state.matches()[0].start_col, 0);
    assert_eq!(state.matches()[1].row, 1);
}

#[test]
fn test_search_columns_with_unicode_prefix() {
    // Box drawing characters are multibyte in UTF-8.
    // Ensure match columns are based on *chars*, not byte offsets.
    let row = make_row("│   base-mail-track-digest256");

    let mut state = SearchState::new();
    state.activate();
    for ch in "mail".chars() {
        state.push_char(ch);
    }

    state.update_matches(&[row]);

    assert_eq!(state.match_count(), 1);
    assert_eq!(state.matches()[0].start_col, 9);
    assert_eq!(state.matches()[0].end_col, 13);
}

#[test]
fn test_search_case_insensitive() {
    let mut state = SearchState::new();
    state.activate();
    state.push_char('h');
    state.push_char('e');
    state.push_char('l');
    state.push_char('l');
    state.push_char('o');

    let display_rows = vec![make_row("Hello World"), make_row("HELLO WORLD")];

    state.update_matches(&display_rows);

    assert_eq!(state.match_count(), 2);
}

#[test]
fn test_search_case_sensitive() {
    let mut state = SearchState::new();
    state.activate();
    state.toggle_case_sensitive();
    state.push_char('h');
    state.push_char('e');
    state.push_char('l');
    state.push_char('l');
    state.push_char('o');

    let display_rows = vec![
        make_row("Hello World"),
        make_row("HELLO WORLD"),
        make_row("hello world"),
    ];

    state.update_matches(&display_rows);

    assert_eq!(state.match_count(), 1);
    assert_eq!(state.matches()[0].row, 2);
}

#[test]
fn test_search_navigation() {
    let mut state = SearchState::new();
    state.activate();
    state.push_char('t');
    state.push_char('e');
    state.push_char('s');
    state.push_char('t');

    let display_rows = vec![make_row("test one test two test three")];

    state.update_matches(&display_rows);

    assert_eq!(state.match_count(), 3);
    assert_eq!(state.current_match_index(), 0);

    state.next_match();
    assert_eq!(state.current_match_index(), 1);

    state.next_match();
    assert_eq!(state.current_match_index(), 2);

    state.next_match();
    assert_eq!(state.current_match_index(), 0); // Wraps around

    state.prev_match();
    assert_eq!(state.current_match_index(), 2); // Wraps backward
}

#[test]
fn test_search_is_match() {
    let mut state = SearchState::new();
    state.activate();
    state.push_char('w');
    state.push_char('o');
    state.push_char('r');
    state.push_char('l');
    state.push_char('d');

    let display_rows = vec![make_row("hello world")];

    state.update_matches(&display_rows);

    // "world" starts at col 6
    // view_start=0 means display row 0 = absolute row 0
    assert!(state.is_match(0, 5, 0).is_none()); // 'o' before world
    assert_eq!(state.is_match(0, 6, 0), Some(true)); // 'w' - current match
    assert_eq!(state.is_match(0, 10, 0), Some(true)); // 'd' - still in match
    assert!(state.is_match(0, 11, 0).is_none()); // Past match
}

#[test]
fn test_search_multiple_matches_same_line() {
    let mut state = SearchState::new();
    state.activate();
    state.push_char('a');
    state.push_char('b');
    state.push_char('c');

    let display_rows = vec![make_row("abcabcabc")];

    state.update_matches(&display_rows);

    assert_eq!(state.match_count(), 3);
    assert_eq!(state.matches()[0].start_col, 0);
    assert_eq!(state.matches()[1].start_col, 3);
    assert_eq!(state.matches()[2].start_col, 6);
}

#[test]
fn test_search_push_pop_char() {
    let mut state = SearchState::new();
    state.activate();

    state.push_char('t');
    state.push_char('e');
    state.push_char('s');

    assert_eq!(state.query(), "tes");

    state.pop_char();
    assert_eq!(state.query(), "te");
}

#[test]
fn test_search_with_history() {
    // Simulate searching through history + live rows
    let mut state = SearchState::new();
    state.activate();
    state.push_char('t');
    state.push_char('e');
    state.push_char('s');
    state.push_char('t');

    // Simulate: 2 history rows + 2 live rows
    let all_rows = vec![
        make_row("history test 1"), // row 0 (history)
        make_row("history row 2"),  // row 1 (history)
        make_row("live test 2"),    // row 2 (live)
        make_row("live row"),       // row 3 (live)
    ];

    state.update_matches(&all_rows);

    // Should find "test" in rows 0 and 2
    assert_eq!(state.match_count(), 2);
    assert_eq!(state.matches()[0].row, 0); // history row
    assert_eq!(state.matches()[1].row, 2); // live row

    // Test is_match with view_start
    // If view shows rows 1-3 (view_start=1), display row 0 = absolute row 1
    assert!(state.is_match(0, 8, 1).is_none()); // display row 0 = abs row 1, no match
    assert_eq!(state.is_match(1, 5, 1), Some(false)); // display row 1 = abs row 2, has "test"

    // Navigate to next match
    state.next_match();
    assert_eq!(state.current_match_index(), 1);
    assert_eq!(state.current_match_row(), Some(2));
}

#[test]
fn test_search_current_match_row() {
    let mut state = SearchState::new();
    state.activate();
    state.push_char('a');

    let all_rows = vec![
        make_row("row a"), // row 0
        make_row("row b"), // row 1
        make_row("row a"), // row 2
    ];

    state.update_matches(&all_rows);
    assert_eq!(state.current_match_row(), Some(0));

    state.next_match();
    assert_eq!(state.current_match_row(), Some(2));
}
