use winit::keyboard::{Key, ModifiersState, NamedKey};

pub fn key_to_pty_bytes(key: &Key, mods: ModifiersState) -> Option<Vec<u8>> {
    let ctrl = mods.control_key();

    match key {
        Key::Named(NamedKey::Enter) => Some(b"\r".to_vec()),
        Key::Named(NamedKey::Tab) => Some(b"\t".to_vec()),
        Key::Named(NamedKey::Space) => {
            if ctrl {
                Some(vec![super::NULL_BYTE])
            } else {
                Some(b" ".to_vec())
            }
        }
        Key::Named(NamedKey::Backspace) => Some(vec![super::BACKSPACE_BYTE]),
        Key::Named(NamedKey::Escape) => Some(vec![super::ESCAPE_BYTE]),
        Key::Named(NamedKey::ArrowUp) => Some([super::ESCAPE_BYTE, b'[', b'A'].to_vec()),
        Key::Named(NamedKey::ArrowDown) => Some([super::ESCAPE_BYTE, b'[', b'B'].to_vec()),
        Key::Named(NamedKey::ArrowRight) => Some([super::ESCAPE_BYTE, b'[', b'C'].to_vec()),
        Key::Named(NamedKey::ArrowLeft) => Some([super::ESCAPE_BYTE, b'[', b'D'].to_vec()),
        Key::Named(NamedKey::Home) => Some([super::ESCAPE_BYTE, b'H'].to_vec()),
        Key::Named(NamedKey::End) => Some([super::ESCAPE_BYTE, b'F'].to_vec()),
        Key::Named(NamedKey::PageUp) => Some([super::ESCAPE_BYTE, b'[', b'5', b'~'].to_vec()),
        Key::Named(NamedKey::PageDown) => Some([super::ESCAPE_BYTE, b'[', b'6', b'~'].to_vec()),
        Key::Named(NamedKey::Delete) => Some([super::ESCAPE_BYTE, b'[', b'3', b'~'].to_vec()),
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
