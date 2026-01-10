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
#[cfg(test)]
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
}
