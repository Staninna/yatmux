use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
enum PluginCommand {
    Toast { message: String },
    ReloadConfig,
    #[serde(other)]
    Other,
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new() -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        path.push(format!(
            "yatmux-it-{nanos}-{}-{counter}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(unix)]
fn write_script(dir: &Path, name: &str, contents: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join(name);
    let mut file = fs::File::create(&path).unwrap();
    file.write_all(contents.as_bytes()).unwrap();
    let mut perms = file.metadata().unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

fn parse_commands(stdout: &str) -> Vec<PluginCommand> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if trimmed.starts_with('[') {
        return serde_json::from_str::<Vec<PluginCommand>>(trimmed).unwrap_or_default();
    }
    let mut commands = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(cmd) = serde_json::from_str::<PluginCommand>(line) {
            commands.push(cmd);
        }
    }
    commands
}

#[cfg(unix)]
#[test]
fn plugin_script_runs_with_env_payload() {
    let temp = TempDir::new();
    let plugin_dir = temp.path.join("plugin");
    fs::create_dir_all(&plugin_dir).unwrap();
    let script = write_script(
        &plugin_dir,
        "plugin.sh",
        r#"#!/usr/bin/env bash
set -euo pipefail
printf '%s' "$YATMUX_PLUGIN_EVENT" > "$YATMUX_PLUGIN_ROOT/seen.json"
echo '{"command":"toast","message":"ok"}'
"#,
    );

    let payload = r#"{"event":"startup"}"#;
    let output = Command::new("bash")
        .arg(&script)
        .env("YATMUX_PLUGIN_EVENT", payload)
        .env("YATMUX_PLUGIN_ROOT", &plugin_dir)
        .output()
        .expect("run plugin script");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let commands = parse_commands(&stdout);
    assert_eq!(commands.len(), 1);
    match &commands[0] {
        PluginCommand::Toast { message } => assert_eq!(message, "ok"),
        _ => panic!("unexpected command"),
    }
    let seen = fs::read_to_string(plugin_dir.join("seen.json")).unwrap();
    assert_eq!(seen, payload);
}

#[cfg(unix)]
#[test]
fn plugin_script_parses_array_output() {
    let temp = TempDir::new();
    let plugin_dir = temp.path.join("plugin");
    fs::create_dir_all(&plugin_dir).unwrap();
    let script = write_script(
        &plugin_dir,
        "plugin.sh",
        r#"#!/usr/bin/env bash
set -euo pipefail
echo '[{"command":"toast","message":"one"},{"command":"reload_config"}]'
"#,
    );

    let output = Command::new("bash")
        .arg(&script)
        .env("YATMUX_PLUGIN_EVENT", r#"{"event":"startup"}"#)
        .env("YATMUX_PLUGIN_ROOT", &plugin_dir)
        .output()
        .expect("run plugin script");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let commands = parse_commands(&stdout);
    assert_eq!(commands.len(), 2);
}
