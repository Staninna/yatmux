use arboard::Clipboard;

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
