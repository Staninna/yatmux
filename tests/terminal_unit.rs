use std::sync::Arc;

use tattoy_wezterm_term::SemanticType;

use yatmux::pty::mock::MockPty;
use yatmux::terminal::Terminal;

fn create_test_terminal() -> (Terminal, Arc<MockPty>) {
    let mock_pty = Arc::new(MockPty::new());
    let terminal = Terminal::new(mock_pty.clone());
    (terminal, mock_pty)
}

#[test]
fn test_terminal_new() {
    let (terminal, _mock_pty) = create_test_terminal();
    let screen = terminal.screen_text();
    assert!(!screen.is_empty());
}

#[test]
fn test_terminal_write_forwards_to_pty() {
    let (terminal, mock_pty) = create_test_terminal();

    terminal.write(b"hello");
    terminal.write(b" world");

    assert_eq!(mock_pty.written_string(), "hello world");
}

#[test]
fn test_terminal_resize_updates_pty() {
    let (terminal, mock_pty) = create_test_terminal();
    terminal.resize(800, 600, 10, 20);

    let resizes = mock_pty.resizes.lock().unwrap();
    assert_eq!(resizes.len(), 1);
    assert_eq!(resizes[0], (30, 80, 800, 600));
}

#[test]
fn test_terminal_handles_output_and_resize_reflow() {
    let (terminal, _mock_pty) = create_test_terminal();
    terminal.process(b"hello world this is a long line");

    terminal.resize(80, 200, 10, 20);
    terminal.resize(800, 200, 10, 20);

    let screen = terminal.screen_text();
    assert!(screen.contains("hello"));
    assert!(screen.contains("world"));
}

#[test]
fn test_shell_title_from_osc() {
    let (terminal, _mock_pty) = create_test_terminal();

    terminal.process(b"\x1b]2;my title\x1b\\");
    assert_eq!(terminal.shell_title().as_deref(), Some("my title"));
}

#[test]
fn test_shell_cwd_from_osc7() {
    let (terminal, _mock_pty) = create_test_terminal();

    terminal.process(b"\x1b]7;file://host/home/alice\x1b\\");
    assert_eq!(
        terminal.shell_cwd().as_deref(),
        Some("file://host/home/alice")
    );
}

#[test]
fn test_semantic_zones_from_osc133() {
    let (terminal, _mock_pty) = create_test_terminal();

    terminal.process(b"\x1b]133;A\x1b\\");
    terminal.process(b"$ ");

    terminal.process(b"\x1b]133;B\x1b\\");
    terminal.process(b"echo hi");

    terminal.process(b"\x1b]133;C\x1b\\");
    terminal.process(b"\r\nhi\r\n");

    let zones = terminal.semantic_zones().unwrap();
    assert!(
        zones
            .iter()
            .any(|z| z.semantic_type == SemanticType::Prompt)
    );
    assert!(zones.iter().any(|z| z.semantic_type == SemanticType::Input));
}

#[test]
fn test_shell_integration_status_detection() {
    let (terminal, _mock_pty) = create_test_terminal();

    let status = terminal.shell_integration_status();
    assert!(!status.any());

    terminal.process(b"\x1b]7;file://host/home/alice\x1b\\");
    assert!(terminal.shell_integration_status().osc7_cwd);

    terminal.process(b"\x1b]2;my title\x1b\\");
    assert!(terminal.shell_integration_status().osc_title);

    terminal.process(b"\x1b]133;A\x1b\\");
    terminal.process(b"$ ");
    terminal.process(b"\x1b]133;B\x1b\\");
    terminal.process(b"echo hi");

    let _ = terminal.semantic_zones().unwrap();
    assert!(terminal.shell_integration_status().osc133_semantic);
}
