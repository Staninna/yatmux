//! Keyboard input translation for PTY communication.

use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::constants::{BACKSPACE_BYTE, ESCAPE_BYTE, NULL_BYTE};

/// Converts a keyboard event to the bytes that should be sent to the PTY.
pub fn key_to_pty_bytes(key: &Key, mods: ModifiersState) -> Option<Vec<u8>> {
    let ctrl = mods.control_key();

    match key {
        Key::Named(NamedKey::Enter) => Some(b"\r".to_vec()),
        Key::Named(NamedKey::Tab) => Some(b"\t".to_vec()),
        Key::Named(NamedKey::Space) => {
            if ctrl {
                Some(vec![NULL_BYTE])
            } else {
                Some(b" ".to_vec())
            }
        }
        Key::Named(NamedKey::Backspace) => Some(vec![BACKSPACE_BYTE]),
        Key::Named(NamedKey::Escape) => Some(vec![ESCAPE_BYTE]),
        Key::Named(NamedKey::ArrowUp) => Some([ESCAPE_BYTE, b'[', b'A'].to_vec()),
        Key::Named(NamedKey::ArrowDown) => Some([ESCAPE_BYTE, b'[', b'B'].to_vec()),
        Key::Named(NamedKey::ArrowRight) => Some([ESCAPE_BYTE, b'[', b'C'].to_vec()),
        Key::Named(NamedKey::ArrowLeft) => Some([ESCAPE_BYTE, b'[', b'D'].to_vec()),
        Key::Named(NamedKey::Home) => Some([ESCAPE_BYTE, b'H'].to_vec()),
        Key::Named(NamedKey::End) => Some([ESCAPE_BYTE, b'F'].to_vec()),
        Key::Named(NamedKey::PageUp) => Some([ESCAPE_BYTE, b'[', b'5', b'~'].to_vec()),
        Key::Named(NamedKey::PageDown) => Some([ESCAPE_BYTE, b'[', b'6', b'~'].to_vec()),
        Key::Named(NamedKey::Delete) => Some([ESCAPE_BYTE, b'[', b'3', b'~'].to_vec()),
        Key::Character(s) => {
            let mut chars = s.chars();
            let ch = chars.next()?;
            if chars.next().is_some() {
                return Some(s.as_bytes().to_vec());
            }
            if ctrl {
                let c = ch.to_ascii_lowercase() as u8;
                if (b'a'..=b'z').contains(&c) {
                    return Some(vec![c - b'a' + 1]);
                }
            }
            Some(ch.to_string().into_bytes())
        }
        _ => None,
    }
}
