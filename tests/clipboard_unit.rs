use std::sync::Arc;

use yatmux::clipboard::ClipboardProvider;
use yatmux::clipboard::mock::MockClipboard;
use yatmux::pty::mock::MockPty;
use yatmux::terminal::Terminal;

#[test]
fn test_clipboard_module_compiles() {
    assert!(true);
}

#[test]
fn test_mock_clipboard_read_write() {
    let clipboard = MockClipboard::new();
    assert!(clipboard.read().is_none());

    assert!(clipboard.write("hello"));
    assert_eq!(clipboard.read(), Some("hello".to_string()));
    assert_eq!(clipboard.content(), Some("hello".to_string()));
}

#[test]
fn test_mock_clipboard_with_content() {
    let clipboard = MockClipboard::with_content("initial");
    assert_eq!(clipboard.read(), Some("initial".to_string()));

    clipboard.write("updated");
    assert_eq!(clipboard.read(), Some("updated".to_string()));
}

#[test]
fn test_mock_clipboard_overwrites() {
    let clipboard = MockClipboard::new();

    clipboard.write("first");
    clipboard.write("second");
    clipboard.write("third");

    assert_eq!(clipboard.read(), Some("third".to_string()));
}

#[test]
fn test_mock_clipboard_empty_string() {
    let clipboard = MockClipboard::new();

    clipboard.write("");
    assert_eq!(clipboard.read(), Some("".to_string()));
}

#[test]
fn test_mock_clipboard_unicode() {
    let clipboard = MockClipboard::new();

    clipboard.write("Hello 世界 🦀");
    assert_eq!(clipboard.read(), Some("Hello 世界 🦀".to_string()));
}

#[test]
fn test_mock_clipboard_multiline() {
    let clipboard = MockClipboard::new();

    let multiline = "line 1\nline 2\nline 3";
    clipboard.write(multiline);
    assert_eq!(clipboard.read(), Some(multiline.to_string()));
}

#[test]
fn test_copy_workflow_with_terminal() {
    let mock_pty = Arc::new(MockPty::new());
    let terminal = Terminal::new(mock_pty);
    let clipboard = MockClipboard::new();

    terminal.process(b"Hello, World!");

    let selection = Some(((0, 0), (0, 4)));

    if let Some(text) = terminal.get_selected_text(selection) {
        clipboard.write(&text);
    }

    assert_eq!(clipboard.read(), Some("Hello".to_string()));
}

#[test]
fn test_paste_workflow_with_terminal() {
    let mock_pty = Arc::new(MockPty::new());
    let terminal = Terminal::new(mock_pty.clone());
    let clipboard = MockClipboard::with_content("pasted text");

    if let Some(text) = clipboard.read() {
        terminal.write(text.as_bytes());
    }

    assert_eq!(mock_pty.written_string(), "pasted text");
}

#[test]
fn test_copy_paste_cycle() {
    let mock_pty = Arc::new(MockPty::new());
    let terminal = Terminal::new(mock_pty.clone());
    let clipboard = MockClipboard::new();

    terminal.process(b"Important data: 12345");

    let selection = Some(((0, 16), (0, 20)));
    if let Some(text) = terminal.get_selected_text(selection) {
        clipboard.write(&text);
    }

    assert_eq!(clipboard.read(), Some("12345".to_string()));

    if let Some(text) = clipboard.read() {
        terminal.write(text.as_bytes());
    }

    assert_eq!(mock_pty.written_string(), "12345");
}
