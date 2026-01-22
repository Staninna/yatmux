use super::discovery::{discover_from_path, resolve_path};
use super::runtime::{parse_plugin_commands, run_plugin};
use super::utils::{escape_shell_single_quotes, sanitize_plugin_name, validate_plugin_root, should_deliver_event};
use super::manager::Plugin;
use super::command::PluginCommand;

use proptest::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

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
            "yatmux-test-{nanos}-{}-{counter}",
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

#[test]
fn parse_plugin_commands_handles_lines_and_array() {
    let stdout = "{\"command\":\"toast\",\"message\":\"hi\"}\n{\"command\":\"reload_config\"}\n";
    let commands = parse_plugin_commands(stdout);
    assert_eq!(commands.len(), 2);

    let stdout = r#"[{"command":"toast","message":"hi"}]"#;
    let commands = parse_plugin_commands(stdout);
    assert_eq!(commands.len(), 1);
}

#[test]
fn parse_plugin_commands_ignores_invalid() {
    let commands = parse_plugin_commands("not-json");
    assert!(commands.is_empty());
}

#[test]
fn parse_plugin_commands_skips_invalid_lines() {
    let stdout = "{\"command\":\"toast\",\"message\":\"ok\"}\nnot-json\n{\"command\":\"reload_config\"}";
    let commands = parse_plugin_commands(stdout);
    assert_eq!(commands.len(), 2);
}

#[test]
fn parse_plugin_commands_invalid_array_returns_empty() {
    let stdout = r#"[{"command":"toast","message":"ok"}, {"command":"nope"}]"#;
    let commands = parse_plugin_commands(stdout);
    assert!(commands.is_empty());
}

#[test]
fn sanitize_plugin_name_strips_invalid_chars() {
    let name = "my plugin!@#_name-1";
    let sanitized = sanitize_plugin_name(name);
    assert_eq!(sanitized, "myplugin_name-1");
}

#[test]
fn validate_plugin_root_requires_absolute_and_exists() {
    let temp = TempDir::new();
    let absolute = temp.path.join("plugin");
    fs::create_dir_all(&absolute).unwrap();
    assert!(validate_plugin_root(&absolute));

    let relative = PathBuf::from("relative-plugin");
    assert!(!validate_plugin_root(&relative));

    let missing = temp.path.join("missing");
    assert!(!validate_plugin_root(&missing));
}

proptest! {
    #[test]
    fn sanitize_plugin_name_filters_disallowed(input in ".*") {
        let sanitized = sanitize_plugin_name(&input);
        prop_assert!(sanitized.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-'));
        prop_assert!(sanitized.len() <= input.len());
    }
}

proptest! {
    #[test]
    fn escape_shell_single_quotes_matches_replace(input in ".*") {
        let escaped = escape_shell_single_quotes(&input);
        prop_assert_eq!(escaped, input.replace('\'', "'\\''"));
    }
}

#[test]
fn subscriptions_filter_events() {
    let mut subscriptions = std::collections::HashMap::new();
    let mut set = HashSet::new();
    set.insert("action".to_string());
    subscriptions.insert("alpha".to_string(), set);

    assert!(should_deliver_event(&subscriptions, "alpha", "ACTION"));
    assert!(!should_deliver_event(&subscriptions, "alpha", "plugin_command"));
    assert!(should_deliver_event(&subscriptions, "beta", "startup"));
    assert!(should_deliver_event(&subscriptions, "beta", "shutdown"));
}

#[test]
fn resolve_path_expands_relative_and_home() {
    let temp = TempDir::new();
    let base = temp.path.join("base");
    fs::create_dir_all(&base).unwrap();

    let rel = resolve_path(&base, "plugins");
    assert_eq!(rel, base.join("plugins"));

    // Home expansion depends on `dirs::home_dir()` which is environment-dependent.
    let expanded = resolve_path(&base, "~/plug");
    if let Some(home) = dirs::home_dir() {
        assert_eq!(expanded, home.join("plug"));
    } else {
        assert_eq!(expanded, base.join("~/plug"));
    }
}

#[test]
fn cwd_url_to_path_strips_host() {
    let path = super::utils::cwd_url_to_path("file://example.com/tmp/testing").unwrap();
    assert_eq!(path, PathBuf::from("/tmp/testing"));
    let path = super::utils::cwd_url_to_path("file:///tmp/testing").unwrap();
    assert_eq!(path, PathBuf::from("/tmp/testing"));
}

#[test]
fn discover_from_path_finds_plugin_sh() {
    let temp = TempDir::new();
    let plugin_dir = temp.path.join("example");
    fs::create_dir_all(&plugin_dir).unwrap();
    let script = plugin_dir.join("plugin.sh");
    fs::write(&script, "#!/usr/bin/env bash\n").unwrap();

    let found = discover_from_path(&plugin_dir).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].0, plugin_dir);
    assert_eq!(found[0].1, script);
}

#[cfg(unix)]
#[test]
fn run_plugin_executes_script_and_parses() {
    let temp = TempDir::new();
    let plugin_dir = temp.path.join("plugin");
    fs::create_dir_all(&plugin_dir).unwrap();
    let script = write_script(
        &plugin_dir,
        "plugin.sh",
        r#"#!/usr/bin/env bash
set -euo pipefail
echo '{"command":"toast","message":"ok"}'
"#,
    );

    let plugin = Plugin {
        name: "test".to_string(),
        root: plugin_dir.clone(),
        script,
    };
    let commands = run_plugin(&plugin, r#"{"event":"startup"}"#, None).unwrap();
    assert_eq!(commands.len(), 1);
    match &commands[0] {
        PluginCommand::Toast { message } => assert_eq!(message, "ok"),
        _ => panic!("unexpected command"),
    }
}

#[cfg(unix)]
#[test]
fn run_plugin_writes_stdin_and_sets_config_path() {
    let temp = TempDir::new();
    let plugin_dir = temp.path.join("plugin");
    fs::create_dir_all(&plugin_dir).unwrap();
    let script = write_script(
        &plugin_dir,
        "plugin.sh",
        r#"#!/usr/bin/env bash
set -euo pipefail
cat > "$YATMUX_PLUGIN_ROOT/payload.json"
printf '%s' "$YATMUX_CONFIG_PATH" > "$YATMUX_PLUGIN_ROOT/config_path.txt"
echo '{"command":"toast","message":"ok"}'
"#,
    );

    let plugin = Plugin {
        name: "test".to_string(),
        root: plugin_dir.clone(),
        script,
    };
    let config_path = temp.path.join("config.toml");
    let payload = r#"{"event":"startup"}"#;
    let commands = run_plugin(&plugin, payload, Some(&config_path)).unwrap();
    assert_eq!(commands.len(), 1);
    let seen_payload = fs::read_to_string(plugin_dir.join("payload.json")).unwrap();
    assert_eq!(seen_payload, payload);
    let seen_config = fs::read_to_string(plugin_dir.join("config_path.txt")).unwrap();
    assert_eq!(seen_config, config_path.to_string_lossy());
}

#[cfg(unix)]
#[test]
fn run_plugin_parses_output_even_on_failure_status() {
    let temp = TempDir::new();
    let plugin_dir = temp.path.join("plugin");
    fs::create_dir_all(&plugin_dir).unwrap();
    let script = write_script(
        &plugin_dir,
        "plugin.sh",
        r#"#!/usr/bin/env bash
set -euo pipefail
echo '{"command":"toast","message":"ok"}'
exit 1
"#,
    );

    let plugin = Plugin {
        name: "test".to_string(),
        root: plugin_dir.clone(),
        script,
    };
    let commands = run_plugin(&plugin, r#"{"event":"startup"}"#, None).unwrap();
    assert_eq!(commands.len(), 1);
}

#[cfg(unix)]
#[test]
fn run_plugin_times_out_with_test_override() {
    let temp = TempDir::new();
    let plugin_dir = temp.path.join("plugin");
    fs::create_dir_all(&plugin_dir).unwrap();
    let script = write_script(
        &plugin_dir,
        "plugin.sh",
        r#"#!/usr/bin/env bash
set -euo pipefail
sleep 2
echo '{"command":"toast","message":"late"}'
"#,
    );

    let plugin = Plugin {
        name: "test".to_string(),
        root: plugin_dir.clone(),
        script,
    };

    let old_timeout = std::env::var("YATMUX_TEST_PLUGIN_TIMEOUT_SECS").ok();
    unsafe {
        std::env::set_var("YATMUX_TEST_PLUGIN_TIMEOUT_SECS", "1");
    }
    let start = std::time::Instant::now();
    let commands = run_plugin(&plugin, r#"{"event":"startup"}"#, None);
    let elapsed = start.elapsed();
    if let Some(old_timeout) = old_timeout {
        unsafe {
            std::env::set_var("YATMUX_TEST_PLUGIN_TIMEOUT_SECS", old_timeout);
        }
    } else {
        unsafe {
            std::env::remove_var("YATMUX_TEST_PLUGIN_TIMEOUT_SECS");
        }
    }

    assert!(commands.is_none());
    assert!(elapsed < std::time::Duration::from_secs(3));
}
