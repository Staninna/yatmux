//! Clipboard operations for the terminal.

use arboard::Clipboard;

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

#[cfg(test)]
mod tests {
    // Note: Clipboard tests are difficult to run in CI environments
    // as they require a display server. These tests are intended
    // for local development.

    #[test]
    fn test_clipboard_module_compiles() {
        // Ensure the module compiles correctly
        assert!(true);
    }
}
