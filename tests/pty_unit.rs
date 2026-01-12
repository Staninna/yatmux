use yatmux::pty::PtyWriter;
use yatmux::pty::mock::MockPty;

#[test]
fn test_mock_pty_write() {
    let pty = MockPty::new();
    pty.write(b"hello");
    pty.write(b" world");
    assert_eq!(pty.written_string(), "hello world");
}

#[test]
fn test_mock_pty_resize() {
    let pty = MockPty::new();
    pty.resize(24, 80, 640, 480);
    pty.resize(30, 100, 800, 600);

    let resizes = pty.resizes.lock().unwrap();
    assert_eq!(resizes.len(), 2);
    assert_eq!(resizes[0], (24, 80, 640, 480));
    assert_eq!(resizes[1], (30, 100, 800, 600));
}
