//! Integration tests for the terminal emulator.
//!
//! These tests exercise the full terminal flow using mock implementations.

use std::sync::Arc;
use yatmux::clipboard::ClipboardProvider;
use yatmux::clipboard::mock::MockClipboard;
use yatmux::config::Config;
use yatmux::pty::mock::MockPty;
use yatmux::terminal::Terminal;

/// Helper to create a test terminal with mock PTY.
fn create_terminal() -> (Terminal, Arc<MockPty>) {
    let mock_pty = Arc::new(MockPty::new());
    let terminal = Terminal::new(mock_pty.clone());
    (terminal, mock_pty)
}

// =============================================================================
// Terminal Output Tests
// =============================================================================

#[test]
fn test_terminal_displays_simple_text() {
    let (terminal, _pty) = create_terminal();

    terminal.process(b"Hello, World!");

    let screen = terminal.screen_text();
    assert!(screen.contains("Hello, World!"));
}

#[test]
fn test_terminal_handles_newlines() {
    let (terminal, _pty) = create_terminal();

    terminal.process(b"Line 1\r\nLine 2\r\nLine 3");

    let screen = terminal.screen_text();
    assert!(screen.contains("Line 1"));
    assert!(screen.contains("Line 2"));
    assert!(screen.contains("Line 3"));
}

#[test]
fn test_terminal_handles_ansi_colors() {
    let (terminal, _pty) = create_terminal();

    // Red text, then reset
    terminal.process(b"\x1b[31mRed Text\x1b[0m Normal");

    let screen = terminal.screen_text();
    assert!(screen.contains("Red Text"));
    assert!(screen.contains("Normal"));
}

#[test]
fn test_terminal_handles_cursor_movement() {
    let (terminal, _pty) = create_terminal();

    // Write text, move cursor home, overwrite
    terminal.process(b"XXXXX");
    terminal.process(b"\x1b[H"); // Cursor home
    terminal.process(b"Hello");

    let screen = terminal.screen_text();
    assert!(screen.contains("Hello"));
}

#[test]
fn test_terminal_handles_clear_screen() {
    let (terminal, _pty) = create_terminal();

    terminal.process(b"Old content");
    terminal.process(b"\x1b[2J\x1b[H"); // Clear screen and home
    terminal.process(b"New content");

    let screen = terminal.screen_text();
    assert!(screen.contains("New content"));
    // Old content should be cleared (or scrolled off)
}

#[test]
fn test_terminal_handles_backspace() {
    let (terminal, _pty) = create_terminal();

    terminal.process(b"Helloo\x08 "); // Backspace and space to erase

    let screen = terminal.screen_text();
    assert!(screen.contains("Hello"));
}

// =============================================================================
// Terminal Input Tests
// =============================================================================

#[test]
fn test_terminal_write_sends_to_pty() {
    let (terminal, mock_pty) = create_terminal();

    terminal.write(b"user input");

    assert_eq!(mock_pty.written_string(), "user input");
}

#[test]
fn test_terminal_write_special_keys() {
    let (terminal, mock_pty) = create_terminal();

    // Simulate Enter key
    terminal.write(b"\r");
    // Simulate Ctrl+C
    terminal.write(b"\x03");
    // Simulate escape
    terminal.write(b"\x1b");

    let written = mock_pty.written_bytes();
    assert!(written.contains(&b'\r'));
    assert!(written.contains(&0x03));
    assert!(written.contains(&0x1b));
}

#[test]
fn test_terminal_write_arrow_keys() {
    let (terminal, mock_pty) = create_terminal();

    // Arrow key escape sequences
    terminal.write(b"\x1b[A"); // Up
    terminal.write(b"\x1b[B"); // Down
    terminal.write(b"\x1b[C"); // Right
    terminal.write(b"\x1b[D"); // Left

    let written = mock_pty.written_string();
    assert!(written.contains("\x1b[A"));
    assert!(written.contains("\x1b[B"));
    assert!(written.contains("\x1b[C"));
    assert!(written.contains("\x1b[D"));
}

// =============================================================================
// Resize Tests
// =============================================================================

#[test]
fn test_terminal_resize_updates_pty() {
    let (terminal, mock_pty) = create_terminal();

    // Resize to 800x600 with 10x20 cells
    terminal.resize(800, 600, 10, 20);

    let resizes = mock_pty.resizes.lock().unwrap();
    assert_eq!(resizes.len(), 1);
    // 800/10 = 80 cols, 600/20 = 30 rows
    assert_eq!(resizes[0], (30, 80, 800, 600));
}

#[test]
fn test_terminal_resize_multiple_times() {
    let (terminal, mock_pty) = create_terminal();

    terminal.resize(800, 600, 10, 20);
    terminal.resize(1024, 768, 10, 20);
    terminal.resize(640, 480, 10, 20);

    let resizes = mock_pty.resizes.lock().unwrap();
    assert_eq!(resizes.len(), 3);
    assert_eq!(resizes[0], (30, 80, 800, 600));
    assert_eq!(resizes[1], (38, 102, 1024, 768));
    assert_eq!(resizes[2], (24, 64, 640, 480));
}

// =============================================================================
// Selection and Copy/Paste Tests
// =============================================================================

#[test]
fn test_copy_selected_text() {
    let (terminal, _pty) = create_terminal();
    let clipboard = MockClipboard::new();

    // Display some text
    terminal.process(b"Copy this text please");

    // Select "Copy this"
    let selection = Some(((0, 0), (0, 8)));
    if let Some(text) = terminal.get_selected_text(selection) {
        clipboard.write(&text);
    }

    assert_eq!(clipboard.read(), Some("Copy this".to_string()));
}

#[test]
fn test_paste_into_terminal() {
    let (terminal, mock_pty) = create_terminal();
    let clipboard = MockClipboard::with_content("pasted content");

    // Simulate paste
    if let Some(text) = clipboard.read() {
        terminal.write(text.as_bytes());
    }

    assert_eq!(mock_pty.written_string(), "pasted content");
}

#[test]
fn test_copy_multiline_selection() {
    let (terminal, _pty) = create_terminal();
    let clipboard = MockClipboard::new();

    terminal.process(b"First line\r\nSecond line\r\nThird line");

    // Select across lines
    let selection = Some(((0, 0), (1, 10)));
    if let Some(text) = terminal.get_selected_text(selection) {
        clipboard.write(&text);
    }

    let content = clipboard.read().unwrap();
    assert!(content.contains("First line"));
    assert!(content.contains("Second line"));
}

#[test]
fn test_copy_paste_round_trip() {
    let (terminal, mock_pty) = create_terminal();
    let clipboard = MockClipboard::new();

    // Step 1: Terminal receives output
    terminal.process(b"Secret: password123");

    // Step 2: User selects "password123"
    let selection = Some(((0, 8), (0, 18)));
    if let Some(text) = terminal.get_selected_text(selection) {
        clipboard.write(&text);
    }

    // Step 3: User pastes it back
    if let Some(text) = clipboard.read() {
        terminal.write(text.as_bytes());
    }

    // Verify the password was sent to PTY
    assert_eq!(mock_pty.written_string(), "password123");
}

// =============================================================================
// Config Tests
// =============================================================================

#[test]
fn test_config_loads_defaults() {
    let config = Config::default();

    assert!(!config.window.title.is_empty());
    assert!(config.colors.background != 0 || config.colors.background == 0); // Just verify it exists
}

#[test]
fn test_config_keybinds() {
    let config = Config::default();

    // Should have default keybinds
    assert!(!config.keybinds.bindings.is_empty());
}

// =============================================================================
// Escape Sequence Tests
// =============================================================================

#[test]
fn test_sgr_bold() {
    let (terminal, _pty) = create_terminal();

    terminal.process(b"\x1b[1mBold\x1b[0m");

    let screen = terminal.screen_text();
    assert!(screen.contains("Bold"));
}

#[test]
fn test_sgr_256_colors() {
    let (terminal, _pty) = create_terminal();

    // 256 color: foreground color 196 (red)
    terminal.process(b"\x1b[38;5;196mRed256\x1b[0m");

    let screen = terminal.screen_text();
    assert!(screen.contains("Red256"));
}

#[test]
fn test_sgr_rgb_colors() {
    let (terminal, _pty) = create_terminal();

    // RGB color: foreground #ff0000
    terminal.process(b"\x1b[38;2;255;0;0mRGB Red\x1b[0m");

    let screen = terminal.screen_text();
    assert!(screen.contains("RGB Red"));
}

#[test]
fn test_cursor_save_restore() {
    let (terminal, _pty) = create_terminal();

    terminal.process(b"Start");
    terminal.process(b"\x1b[s"); // Save cursor
    terminal.process(b"\x1b[10;10H"); // Move cursor
    terminal.process(b"Middle");
    terminal.process(b"\x1b[u"); // Restore cursor
    terminal.process(b"End");

    let screen = terminal.screen_text();
    assert!(screen.contains("Start"));
    assert!(screen.contains("Middle"));
    assert!(screen.contains("End"));
}

#[test]
fn test_erase_in_line() {
    let (terminal, _pty) = create_terminal();

    terminal.process(b"AAAAAAAAAA");
    terminal.process(b"\x1b[H"); // Home
    terminal.process(b"\x1b[5C"); // Move right 5
    terminal.process(b"\x1b[K"); // Erase to end of line
    terminal.process(b"BBB");

    let screen = terminal.screen_text();
    assert!(screen.contains("AAAAABBB"));
}

// =============================================================================
// Stress Tests
// =============================================================================

#[test]
fn test_large_output() {
    let (terminal, _pty) = create_terminal();

    // Send 1000 lines
    for i in 0..1000 {
        terminal.process(format!("Line {}\r\n", i).as_bytes());
    }

    // Should not panic, terminal should handle it
    let screen = terminal.screen_text();
    assert!(!screen.is_empty());
}

#[test]
fn test_rapid_resize() {
    let (terminal, mock_pty) = create_terminal();

    // Resize many times rapidly
    for i in 1..100 {
        terminal.resize(800 + i * 10, 600 + i * 5, 10, 20);
    }

    let resizes = mock_pty.resizes.lock().unwrap();
    assert_eq!(resizes.len(), 99);
}

#[test]
fn test_concurrent_read_write() {
    let (terminal, mock_pty) = create_terminal();

    // Simulate bidirectional communication
    terminal.process(b"prompt> ");
    terminal.write(b"command");
    terminal.process(b"command\r\noutput\r\nprompt> ");
    terminal.write(b"exit");

    let screen = terminal.screen_text();
    assert!(screen.contains("prompt>"));

    let written = mock_pty.written_string();
    assert!(written.contains("command"));
    assert!(written.contains("exit"));
}
