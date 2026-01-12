//! Configuration tests for the terminal emulator.
//!
//! These tests verify config parsing, serialization, and keybind functionality.

use term::config::{Action, Config, Keybind, KeybindConfig};
use term::constants::{DEFAULT_BG_COLOR, DEFAULT_ROWS};

#[test]
fn test_default_config() {
    let config = Config::default();
    assert_eq!(config.window.title, "term");
    assert_eq!(config.colors.background, DEFAULT_BG_COLOR);
    assert_eq!(config.colors.accent, 0x66AAFF);
    assert_eq!(config.terminal.rows, DEFAULT_ROWS);
}

#[test]
fn test_config_parse() {
    let toml = r#"
        [window]
        title = "my-term"

        [colors]
        background = 0x000000
        foreground = 0xFFFFFF
        accent = 0x123456

        [terminal]
        scrollback_lines = 10000
    "#;

    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.window.title, "my-term");
    assert_eq!(config.colors.background, 0x000000);
    assert_eq!(config.colors.foreground, 0xFFFFFF);
    assert_eq!(config.colors.accent, 0x123456);
    assert_eq!(config.terminal.scrollback_lines, 10000);
}

#[test]
fn test_config_serialize() {
    let config = Config::default();
    let toml = toml::to_string(&config).unwrap();
    assert!(toml.contains("[window]"));
    assert!(toml.contains("[colors]"));
    // Verify colors are serialized as hex strings
    assert!(toml.contains("background = \"#"));
    assert!(toml.contains("foreground = \"#"));
    assert!(toml.contains("accent = \"#"));
}

#[test]
fn test_keybind_parse() {
    let kb = Keybind::parse("ctrl+shift+c").unwrap();
    assert_eq!(kb.key, "c");
    assert!(kb.ctrl);
    assert!(kb.shift);
    assert!(!kb.alt);

    let kb = Keybind::parse("f12").unwrap();
    assert_eq!(kb.key, "f12");
    assert!(!kb.ctrl);
    assert!(!kb.shift);
    assert!(!kb.alt);

    let kb = Keybind::parse("alt+enter").unwrap();
    assert_eq!(kb.key, "enter");
    assert!(!kb.ctrl);
    assert!(!kb.shift);
    assert!(kb.alt);
}

#[test]
fn test_keybind_matches() {
    let kb = Keybind::parse("ctrl+shift+c").unwrap();
    assert!(kb.matches("c", true, true, false));
    assert!(!kb.matches("c", true, false, false));
    assert!(!kb.matches("v", true, true, false));
}

#[test]
fn test_keybind_config_get_action() {
    let config = KeybindConfig::default();
    assert_eq!(
        config.get_action("c", true, true, false),
        Some(Action::Copy)
    );
    assert_eq!(
        config.get_action("v", true, false, false),
        Some(Action::Paste)
    );
    assert_eq!(
        config.get_action("pageup", false, true, false),
        Some(Action::ScrollPageUp)
    );
    assert_eq!(config.get_action("x", true, true, false), None);
}

#[test]
fn test_keybind_config_parse() {
    let toml = r#"
        [keybinds]
        "ctrl+c" = "copy"
        "ctrl+v" = "paste"
        "f1" = "scroll_to_top"
    "#;

    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(
        config.keybinds.get_action("c", true, false, false),
        Some(Action::Copy)
    );
    assert_eq!(
        config.keybinds.get_action("f1", false, false, false),
        Some(Action::ScrollToTop)
    );
}

#[test]
fn test_search_keybinds() {
    let config = KeybindConfig::default();

    // Search mode keybinds
    assert_eq!(
        config.get_action("escape", false, false, false),
        Some(Action::SearchClose)
    );
    assert_eq!(
        config.get_action("enter", false, false, false),
        Some(Action::SearchConfirm)
    );
    assert_eq!(
        config.get_action("n", true, false, false),
        Some(Action::SearchNext)
    );
    assert_eq!(
        config.get_action("p", true, false, false),
        Some(Action::SearchPrev)
    );
    assert_eq!(
        config.get_action("c", true, false, false),
        Some(Action::SearchToggleCase)
    );
}

#[test]
fn test_search_keybinds_configurable() {
    let toml = r#"
        [keybinds]
        "ctrl+g" = "search_close"
        "ctrl+j" = "search_next"
        "ctrl+k" = "search_prev"
    "#;

    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(
        config.keybinds.get_action("g", true, false, false),
        Some(Action::SearchClose)
    );
    assert_eq!(
        config.keybinds.get_action("j", true, false, false),
        Some(Action::SearchNext)
    );
    assert_eq!(
        config.keybinds.get_action("k", true, false, false),
        Some(Action::SearchPrev)
    );
}

#[test]
fn test_search_arrow_keybinds() {
    let config = KeybindConfig::default();

    // Arrow keys should work for search navigation
    assert_eq!(
        config.get_action("down", false, false, false),
        Some(Action::SearchNext)
    );
    assert_eq!(
        config.get_action("up", false, false, false),
        Some(Action::SearchPrev)
    );
}

#[test]
fn test_keybind_disable_with_null() {
    let toml = r#"
[keybinds]
"ctrl+shift+-" = "none"
"ctrl+shift+c" = "copy"
"#;

    let config: Config = toml::from_str(toml).unwrap();

    // Disabled bindings should return None
    assert_eq!(config.keybinds.get_action("-", true, true, false), None);

    // Enabled binding should still work
    assert_eq!(
        config.keybinds.get_action("c", true, true, false),
        Some(Action::Copy)
    );

    // Check is_disabled helper
    assert!(config.keybinds.is_disabled("-", true, true, false));
    assert!(!config.keybinds.is_disabled("c", true, true, false));
}

#[test]
fn test_keybind_disable_overrides_default() {
    // Default config has split_horizontal bound to ctrl+shift+-
    let default_config = KeybindConfig::default();
    assert_eq!(
        default_config.get_action("-", true, true, false),
        Some(Action::SplitHorizontal)
    );

    // User config with "none" should disable it
    let toml = r#"
[keybinds]
"ctrl+shift+-" = "none"
"#;

    let mut config: Config = toml::from_str(toml).unwrap();
    config.keybinds.apply_defaults();

    // Should still be disabled even after applying defaults
    assert_eq!(config.keybinds.get_action("-", true, true, false), None);
    assert!(config.keybinds.is_disabled("-", true, true, false));
}
