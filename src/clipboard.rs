//! Clipboard operations for the terminal.

use arboard::Clipboard;

/// Trait for clipboard operations.
///
/// This abstraction allows for mocking the clipboard in tests.
pub trait ClipboardProvider: Send + Sync {
    /// Reads text from the clipboard.
    fn read(&self) -> Option<String>;

    /// Writes text to the clipboard.
    fn write(&self, text: &str) -> bool;
}

/// System clipboard implementation using arboard.
#[derive(Default)]
pub struct SystemClipboard;

impl SystemClipboard {
    /// Creates a new system clipboard provider.
    pub fn new() -> Self {
        Self
    }
}

impl ClipboardProvider for SystemClipboard {
    fn read(&self) -> Option<String> {
        read_clipboard_text()
    }

    fn write(&self, text: &str) -> bool {
        write_clipboard_text(text)
    }
}

/// Reads text from the system clipboard.
pub fn read_clipboard_text() -> Option<String> {
    match Clipboard::new() {
        Ok(mut clipboard) => match clipboard.get_text() {
            Ok(text) => Some(text),
            Err(err) => {
                eprintln!("clipboard read failed: {err:#}");
                None
            }
        },
        Err(err) => {
            eprintln!("clipboard init failed: {err:#}");
            None
        }
    }
}

/// Writes text to the system clipboard.
pub fn write_clipboard_text(text: &str) -> bool {
    match Clipboard::new() {
        Ok(mut clipboard) => match clipboard.set_text(text.to_string()) {
            Ok(()) => true,
            Err(err) => {
                eprintln!("clipboard write failed: {err:#}");
                false
            }
        },
        Err(err) => {
            eprintln!("clipboard init failed: {err:#}");
            false
        }
    }
}

/// Mock clipboard for testing.
pub mod mock {
    use super::ClipboardProvider;
    use std::sync::Mutex;

    /// A mock clipboard that stores text in memory.
    #[derive(Default)]
    pub struct MockClipboard {
        content: Mutex<Option<String>>,
    }

    impl MockClipboard {
        /// Creates a new empty mock clipboard.
        pub fn new() -> Self {
            Self::default()
        }

        /// Creates a mock clipboard with initial content.
        pub fn with_content(text: &str) -> Self {
            Self {
                content: Mutex::new(Some(text.to_string())),
            }
        }

        /// Gets the current clipboard content.
        pub fn content(&self) -> Option<String> {
            self.content.lock().unwrap().clone()
        }
    }

    impl ClipboardProvider for MockClipboard {
        fn read(&self) -> Option<String> {
            self.content.lock().unwrap().clone()
        }

        fn write(&self, text: &str) -> bool {
            *self.content.lock().unwrap() = Some(text.to_string());
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Clipboard tests are difficult to run in CI environments
    // as they require a display server. These tests are intended
    // for local development.

    #[test]
    fn test_clipboard_module_compiles() {
        // Ensure the module compiles correctly
        assert!(true);
    }

    #[test]
    fn test_mock_clipboard_read_write() {
        let clipboard = mock::MockClipboard::new();
        assert!(clipboard.read().is_none());

        assert!(clipboard.write("hello"));
        assert_eq!(clipboard.read(), Some("hello".to_string()));
        assert_eq!(clipboard.content(), Some("hello".to_string()));
    }

    #[test]
    fn test_mock_clipboard_with_content() {
        let clipboard = mock::MockClipboard::with_content("initial");
        assert_eq!(clipboard.read(), Some("initial".to_string()));

        clipboard.write("updated");
        assert_eq!(clipboard.read(), Some("updated".to_string()));
    }

    #[test]
    fn test_mock_clipboard_overwrites() {
        let clipboard = mock::MockClipboard::new();

        clipboard.write("first");
        clipboard.write("second");
        clipboard.write("third");

        assert_eq!(clipboard.read(), Some("third".to_string()));
    }

    #[test]
    fn test_mock_clipboard_empty_string() {
        let clipboard = mock::MockClipboard::new();

        clipboard.write("");
        assert_eq!(clipboard.read(), Some("".to_string()));
    }

    #[test]
    fn test_mock_clipboard_unicode() {
        let clipboard = mock::MockClipboard::new();

        clipboard.write("Hello 世界 🦀");
        assert_eq!(clipboard.read(), Some("Hello 世界 🦀".to_string()));
    }

    #[test]
    fn test_mock_clipboard_multiline() {
        let clipboard = mock::MockClipboard::new();

        let multiline = "line 1\nline 2\nline 3";
        clipboard.write(multiline);
        assert_eq!(clipboard.read(), Some(multiline.to_string()));
    }

    /// Integration test: simulates copy from terminal to clipboard
    #[test]
    fn test_copy_workflow_with_terminal() {
        use crate::pty::mock::MockPty;
        use crate::terminal::Terminal;
        use std::sync::Arc;

        // Set up terminal with mock PTY
        let mock_pty = Arc::new(MockPty::new());
        let terminal = Terminal::new(mock_pty);
        let clipboard = mock::MockClipboard::new();

        // Simulate text appearing in terminal (from PTY output)
        terminal.process(b"Hello, World!");

        // Simulate selecting "Hello" (row 0, cols 0-4)
        let selection = Some(((0, 0), (0, 4)));

        // Get selected text from terminal
        if let Some(text) = terminal.get_selected_text(selection) {
            clipboard.write(&text);
        }

        // Verify clipboard contains the selected text
        assert_eq!(clipboard.read(), Some("Hello".to_string()));
    }

    /// Integration test: simulates paste from clipboard to terminal
    #[test]
    fn test_paste_workflow_with_terminal() {
        use crate::pty::mock::MockPty;
        use crate::terminal::Terminal;
        use std::sync::Arc;

        // Set up terminal with mock PTY
        let mock_pty = Arc::new(MockPty::new());
        let terminal = Terminal::new(mock_pty.clone());
        let clipboard = mock::MockClipboard::with_content("pasted text");

        // Simulate paste action
        if let Some(text) = clipboard.read() {
            terminal.write(text.as_bytes());
        }

        // Verify the text was written to the PTY
        assert_eq!(mock_pty.written_string(), "pasted text");
    }

    /// Integration test: copy then paste cycle
    #[test]
    fn test_copy_paste_cycle() {
        use crate::pty::mock::MockPty;
        use crate::terminal::Terminal;
        use std::sync::Arc;

        // Set up terminal with mock PTY
        let mock_pty = Arc::new(MockPty::new());
        let terminal = Terminal::new(mock_pty.clone());
        let clipboard = mock::MockClipboard::new();

        // 1. Simulate terminal receiving output
        terminal.process(b"Important data: 12345");

        // 2. Copy selection from terminal to clipboard
        let selection = Some(((0, 16), (0, 20))); // "12345"
        if let Some(text) = terminal.get_selected_text(selection) {
            clipboard.write(&text);
        }

        // 3. Verify clipboard has the copied text
        assert_eq!(clipboard.read(), Some("12345".to_string()));

        // 4. Simulate paste back to terminal
        if let Some(text) = clipboard.read() {
            terminal.write(text.as_bytes());
        }

        // 5. Verify the text was written to PTY
        assert_eq!(mock_pty.written_string(), "12345");
    }
}
