use winit::keyboard::{Key, ModifiersState, NamedKey};

use yatmux::constants::{BACKSPACE_BYTE, ESCAPE_BYTE, NULL_BYTE};
use yatmux::keys::key_to_pty_bytes;

#[test]
fn test_enter_key() {
    let result = key_to_pty_bytes(&Key::Named(NamedKey::Enter), ModifiersState::empty());
    assert_eq!(result, Some(b"\r".to_vec()));
}

#[test]
fn test_tab_key() {
    let result = key_to_pty_bytes(&Key::Named(NamedKey::Tab), ModifiersState::empty());
    assert_eq!(result, Some(b"\t".to_vec()));
}

#[test]
fn test_backspace_key() {
    let result = key_to_pty_bytes(&Key::Named(NamedKey::Backspace), ModifiersState::empty());
    assert_eq!(result, Some(vec![BACKSPACE_BYTE]));
}

#[test]
fn test_escape_key() {
    let result = key_to_pty_bytes(&Key::Named(NamedKey::Escape), ModifiersState::empty());
    assert_eq!(result, Some(vec![ESCAPE_BYTE]));
}

#[test]
fn test_arrow_keys() {
    let up = key_to_pty_bytes(&Key::Named(NamedKey::ArrowUp), ModifiersState::empty());
    assert_eq!(up, Some(vec![ESCAPE_BYTE, b'[', b'A']));

    let down = key_to_pty_bytes(&Key::Named(NamedKey::ArrowDown), ModifiersState::empty());
    assert_eq!(down, Some(vec![ESCAPE_BYTE, b'[', b'B']));
}

#[test]
fn test_character_key() {
    let result = key_to_pty_bytes(&Key::Character("a".into()), ModifiersState::empty());
    assert_eq!(result, Some(b"a".to_vec()));
}

#[test]
fn test_ctrl_c() {
    let result = key_to_pty_bytes(&Key::Character("c".into()), ModifiersState::CONTROL);
    assert_eq!(result, Some(vec![3]));
}

#[test]
fn test_ctrl_space() {
    let result = key_to_pty_bytes(&Key::Named(NamedKey::Space), ModifiersState::CONTROL);
    assert_eq!(result, Some(vec![NULL_BYTE]));
}
