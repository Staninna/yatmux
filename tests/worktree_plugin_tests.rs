use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

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
fn write_script(dir: &PathBuf, name: &str, contents: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join(name);
    let mut file = fs::File::create(&path).unwrap();
    file.write_all(contents.as_bytes()).unwrap();
    let mut perms = file.metadata().unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

#[cfg(unix)]
fn has_python3() -> bool {
    Command::new("python3")
        .arg("--version")
        .output()
        .is_ok()
}

#[cfg(unix)]
fn parse_json_lines(stdout: &str) -> Vec<Value> {
    stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
        .collect()
}

#[cfg(unix)]
fn find_command<'a>(commands: &'a [Value], name: &str) -> Option<&'a Value> {
    commands
        .iter()
        .find(|cmd| cmd.get("command").and_then(Value::as_str) == Some(name))
}

#[cfg(unix)]
fn install_fake_git(bin_dir: &PathBuf) -> PathBuf {
    let script = r#"#!/usr/bin/env bash
set -euo pipefail
args="$*"
if [[ "$args" == *"rev-parse --show-toplevel"* ]]; then
  printf '%s' "${YATMUX_TEST_GIT_REPO:-}"
  exit 0
fi
if [[ "$args" == *"show-ref --verify --quiet"* ]]; then
  branch="${@: -1}"
  if [[ "$branch" == "refs/heads/main" || "$branch" == "refs/heads/existing" ]]; then
    exit 0
  fi
  exit 1
fi
if [[ "$args" == *"worktree list --porcelain"* ]]; then
  cat <<EOF
worktree ${YATMUX_TEST_GIT_REPO}
branch refs/heads/main
worktree ${YATMUX_TEST_GIT_REPO}/.worktrees/feature
branch refs/heads/feature
EOF
  exit 0
fi
if [[ "$args" == *"worktree add"* ]]; then
  exit 0
fi
if [[ "$args" == *"worktree remove"* ]]; then
  exit 0
fi
exit 0
"#;
    write_script(bin_dir, "git", &script)
}

#[cfg(unix)]
fn run_worktree_plugin(
    event_json: &str,
    plugin_root: &PathBuf,
    repo: &PathBuf,
    bin_dir: &PathBuf,
) -> std::process::Output {
    Command::new("bash")
        .arg("examples/plugins/worktree/plugin.sh")
        .env("YATMUX_PLUGIN_EVENT", event_json)
        .env("YATMUX_PLUGIN_ROOT", plugin_root)
        .env("YATMUX_TEST_GIT_REPO", repo)
        .env(
            "PATH",
            format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap_or_default()),
        )
        .output()
        .expect("run worktree plugin")
}

#[test]
fn test_worktree_plugin_exists() {
    let plugin_path = PathBuf::from("examples/plugins/worktree/plugin.sh");
    assert!(
        plugin_path.exists(),
        "Worktree plugin should exist at examples/plugins/worktree/plugin.sh"
    );
}

#[test]
fn test_worktree_plugin_has_python3_check() {
    let plugin_src =
        fs::read_to_string("examples/plugins/worktree/plugin.sh").expect("Failed to read plugin");

    // Verify python3 check exists
    assert!(
        plugin_src.contains("command -v python3"),
        "Plugin should check for python3"
    );

    // Verify toast notification on missing python3
    assert!(
        plugin_src.contains("Worktree plugin disabled: python3 required"),
        "Plugin should show toast when python3 is missing"
    );

    // Verify check is in startup event
    assert!(
        plugin_src.contains(r#"if [ "$event" = "startup" ]"#),
        "Plugin should handle startup event"
    );
}

#[test]
fn test_worktree_plugin_has_state_cleanup() {
    let plugin_src =
        fs::read_to_string("examples/plugins/worktree/plugin.sh").expect("Failed to read plugin");

    // Verify state cleanup exists
    assert!(
        plugin_src.contains("find \"$state_dir\""),
        "Plugin should have state cleanup logic"
    );

    // Verify it uses the configurable variable
    assert!(
        plugin_src.contains("STATE_CLEANUP_DAYS"),
        "Plugin should use configurable STATE_CLEANUP_DAYS"
    );

    assert!(
        plugin_src.contains("-mtime +"),
        "Plugin should clean up files by mtime"
    );

    assert!(
        plugin_src.contains("-delete"),
        "Plugin should delete old state files"
    );

    // Verify it can be disabled
    assert!(
        plugin_src.contains("-gt 0"),
        "Plugin should allow disabling cleanup with 0"
    );
}

#[test]
fn test_worktree_plugin_has_improved_slugify() {
    let plugin_src =
        fs::read_to_string("examples/plugins/worktree/plugin.sh").expect("Failed to read plugin");

    // Verify slugify function exists and is improved
    assert!(plugin_src.contains("slugify()"), "Plugin should have slugify function");

    // Check for improvements: removing consecutive dashes
    assert!(
        plugin_src.contains("sed 's#-\\+#-#g'"),
        "Slugify should remove consecutive dashes"
    );

    // Check for improvements: removing leading dashes
    assert!(
        plugin_src.contains("sed 's#^-##'"),
        "Slugify should remove leading dashes"
    );

    // Check for improvements: removing trailing dashes
    assert!(
        plugin_src.contains("sed 's#-$##'"),
        "Slugify should remove trailing dashes"
    );
}

#[test]
fn test_worktree_plugin_has_confirm_dialog() {
    let plugin_src =
        fs::read_to_string("examples/plugins/worktree/plugin.sh").expect("Failed to read plugin");

    // Verify emit_confirm function exists
    assert!(
        plugin_src.contains("emit_confirm()"),
        "Plugin should have emit_confirm function"
    );

    // Verify confirm command structure
    assert!(
        plugin_src.contains(r#""command":"confirm""#),
        "emit_confirm should emit confirm command"
    );

    // Verify close operation uses confirmation
    assert!(
        plugin_src.contains("close_confirm"),
        "Close operation should use confirmation"
    );

    // Verify confirmation message
    assert!(
        plugin_src.contains("This cannot be undone"),
        "Confirmation should warn about destructive action"
    );
}

#[test]
fn test_worktree_plugin_has_close_confirm_handler() {
    let plugin_src =
        fs::read_to_string("examples/plugins/worktree/plugin.sh").expect("Failed to read plugin");

    // Verify close_confirm action handler exists
    assert!(
        plugin_src.contains("close_confirm)"),
        "Plugin should handle close_confirm action"
    );

    // Verify it proceeds to show picker after confirmation
    assert!(
        plugin_src.contains(r#"\"action\":\"close\""#),
        "close_confirm should lead to close action"
    );
}

#[test]
fn test_worktree_plugin_json_helpers() {
    let plugin_src =
        fs::read_to_string("examples/plugins/worktree/plugin.sh").expect("Failed to read plugin");

    // Verify json helper functions exist
    assert!(plugin_src.contains("json_get()"), "Plugin should have json_get helper");
    assert!(
        plugin_src.contains("json_escape()"),
        "Plugin should have json_escape helper"
    );

    // Verify python3 is used for JSON parsing
    assert!(
        plugin_src.contains("python3 -"),
        "Plugin should use python3 for JSON operations"
    );
}

#[test]
fn test_worktree_plugin_has_all_commands() {
    let plugin_src =
        fs::read_to_string("examples/plugins/worktree/plugin.sh").expect("Failed to read plugin");

    // Verify all main commands exist
    assert!(plugin_src.contains("new)"), "Plugin should support 'new' command");
    assert!(
        plugin_src.contains("switch)"),
        "Plugin should support 'switch' command"
    );
    assert!(plugin_src.contains("close)"), "Plugin should support 'close' command");
    assert!(plugin_src.contains("sync)"), "Plugin should support 'sync' command");
}

#[test]
fn test_worktree_plugin_has_emit_functions() {
    let plugin_src =
        fs::read_to_string("examples/plugins/worktree/plugin.sh").expect("Failed to read plugin");

    // Verify all emit functions exist
    assert!(
        plugin_src.contains("emit_toast()"),
        "Plugin should have emit_toast function"
    );
    assert!(
        plugin_src.contains("emit_new_tab()"),
        "Plugin should have emit_new_tab function"
    );
    assert!(
        plugin_src.contains("emit_pick()"),
        "Plugin should have emit_pick function"
    );
    assert!(
        plugin_src.contains("emit_prompt()"),
        "Plugin should have emit_prompt function"
    );
    assert!(
        plugin_src.contains("emit_confirm()"),
        "Plugin should have emit_confirm function"
    );
    assert!(
        plugin_src.contains("emit_request_state()"),
        "Plugin should have emit_request_state function"
    );
    assert!(
        plugin_src.contains("emit_focus_tab()"),
        "Plugin should have emit_focus_tab function"
    );
    assert!(
        plugin_src.contains("emit_close_tab()"),
        "Plugin should have emit_close_tab function"
    );
    assert!(
        plugin_src.contains("emit_close_pane()"),
        "Plugin should have emit_close_pane function"
    );
}

#[test]
fn test_worktree_plugin_state_management() {
    let plugin_src =
        fs::read_to_string("examples/plugins/worktree/plugin.sh").expect("Failed to read plugin");

    // Verify state management functions
    assert!(
        plugin_src.contains("save_request()"),
        "Plugin should have save_request function"
    );
    assert!(
        plugin_src.contains("load_request()"),
        "Plugin should have load_request function"
    );
    assert!(
        plugin_src.contains("rm_request()"),
        "Plugin should have rm_request function"
    );

    // Verify state directory usage
    assert!(
        plugin_src.contains("state_dir="),
        "Plugin should define state_dir variable"
    );
    assert!(
        plugin_src.contains("mkdir -p \"$state_dir\""),
        "Plugin should create state directory"
    );
}

#[test]
fn test_worktree_plugin_git_operations() {
    let plugin_src =
        fs::read_to_string("examples/plugins/worktree/plugin.sh").expect("Failed to read plugin");

    // Verify git worktree operations
    assert!(
        plugin_src.contains("worktree add"),
        "Plugin should use git worktree add"
    );
    assert!(
        plugin_src.contains("worktree remove"),
        "Plugin should use git worktree remove"
    );
    assert!(
        plugin_src.contains("worktree list --porcelain"),
        "Plugin should list worktrees"
    );

    // Verify repo root detection
    assert!(
        plugin_src.contains("rev-parse --show-toplevel"),
        "Plugin should detect repo root"
    );
}

#[test]
fn test_worktree_plugin_subscribes_to_events() {
    let plugin_src =
        fs::read_to_string("examples/plugins/worktree/plugin.sh").expect("Failed to read plugin");

    // Verify subscription to required events
    assert!(
        plugin_src.contains("plugin_command"),
        "Plugin should subscribe to plugin_command events"
    );
    assert!(
        plugin_src.contains("prompt_response"),
        "Plugin should subscribe to prompt_response events"
    );
    assert!(
        plugin_src.contains("state_response"),
        "Plugin should subscribe to state_response events"
    );
}

#[test]
fn test_worktree_plugin_handles_events() {
    let plugin_src =
        fs::read_to_string("examples/plugins/worktree/plugin.sh").expect("Failed to read plugin");

    // Verify event handlers
    assert!(
        plugin_src.contains(r#"if [ "$event" = "startup" ]"#),
        "Plugin should handle startup event"
    );
    assert!(
        plugin_src.contains(r#"if [ "$event" = "plugin_command" ]"#),
        "Plugin should handle plugin_command event"
    );
    assert!(
        plugin_src.contains(r#"if [ "$event" = "prompt_response" ]"#),
        "Plugin should handle prompt_response event"
    );
    assert!(
        plugin_src.contains(r#"if [ "$event" = "state_response" ]"#),
        "Plugin should handle state_response event"
    );
}

#[test]
fn test_worktree_plugin_has_readme() {
    let readme_path = PathBuf::from("examples/plugins/worktree/README.md");
    assert!(
        readme_path.exists(),
        "Worktree plugin should have a README.md"
    );

    let readme = fs::read_to_string(&readme_path).expect("Failed to read README");

    // Verify key sections exist
    assert!(readme.contains("# Worktree Plugin"), "README should have title");
    assert!(readme.contains("## Features"), "README should describe features");
    assert!(readme.contains("## Requirements"), "README should list requirements");
    assert!(readme.contains("## Commands"), "README should document commands");
    assert!(
        readme.contains("## Troubleshooting"),
        "README should have troubleshooting section"
    );
}

#[cfg(unix)]
#[test]
fn test_worktree_plugin_runtime_new_command() {
    if !has_python3() {
        return;
    }
    let temp = TempDir::new();
    let repo = temp.path.join("repo");
    fs::create_dir_all(&repo).unwrap();
    let plugin_root = temp.path.join("plugin-root");
    fs::create_dir_all(&plugin_root).unwrap();
    let bin_dir = temp.path.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    install_fake_git(&bin_dir);

    let worktree_path = repo.join(".worktrees").join("feature");
    let event_json = format!(
        r#"{{"event":"plugin_command","data":{{"plugin":"worktree","command":"new","cwd":"{}","args":{{"branch":"feature","path":"{}"}}}}}}"#,
        repo.display(),
        worktree_path.display()
    );
    let worktree_path_str = worktree_path.to_string_lossy().to_string();
    let output = Command::new("bash")
        .arg("examples/plugins/worktree/plugin.sh")
        .env("YATMUX_PLUGIN_EVENT", event_json)
        .env("YATMUX_PLUGIN_ROOT", &plugin_root)
        .env("YATMUX_TEST_GIT_REPO", &repo)
        .env("PATH", format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap_or_default()))
        .output()
        .expect("run worktree plugin");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let commands = parse_json_lines(&stdout);
    assert!(commands.iter().any(|cmd| {
        cmd.get("command").and_then(Value::as_str) == Some("new_tab")
            && cmd.get("cwd").and_then(Value::as_str) == Some(worktree_path_str.as_str())
            && cmd.get("title").and_then(Value::as_str) == Some("feature")
    }));
}

#[cfg(unix)]
#[test]
fn test_worktree_plugin_runtime_switch_command() {
    if !has_python3() {
        return;
    }
    let temp = TempDir::new();
    let repo = temp.path.join("repo");
    fs::create_dir_all(&repo).unwrap();
    let plugin_root = temp.path.join("plugin-root");
    fs::create_dir_all(&plugin_root).unwrap();
    let bin_dir = temp.path.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    install_fake_git(&bin_dir);

    let event_json = format!(
        r#"{{"event":"plugin_command","data":{{"plugin":"worktree","command":"switch","cwd":"{}"}}}}"#,
        repo.display()
    );
    let output = Command::new("bash")
        .arg("examples/plugins/worktree/plugin.sh")
        .env("YATMUX_PLUGIN_EVENT", event_json)
        .env("YATMUX_PLUGIN_ROOT", &plugin_root)
        .env("YATMUX_TEST_GIT_REPO", &repo)
        .env("PATH", format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap_or_default()))
        .output()
        .expect("run worktree plugin");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let commands = parse_json_lines(&stdout);
    let request = commands.iter().find(|cmd| cmd.get("command").and_then(Value::as_str) == Some("request_state"));
    assert!(request.is_some());
    let id = request.and_then(|cmd| cmd.get("id").and_then(Value::as_str)).unwrap_or("");
    assert!(id.starts_with("wt-switch-"));
}

#[cfg(unix)]
#[test]
fn test_worktree_plugin_runtime_sync_flow() {
    if !has_python3() {
        return;
    }
    let temp = TempDir::new();
    let repo = temp.path.join("repo");
    fs::create_dir_all(&repo).unwrap();
    let plugin_root = temp.path.join("plugin-root");
    fs::create_dir_all(&plugin_root).unwrap();
    let bin_dir = temp.path.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    install_fake_git(&bin_dir);

    let sync_event = format!(
        r#"{{"event":"plugin_command","data":{{"plugin":"worktree","command":"sync","cwd":"{}","args":{{"close_orphans":true}}}}}}"#,
        repo.display()
    );
    let output = run_worktree_plugin(&sync_event, &plugin_root, &repo, &bin_dir);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let commands = parse_json_lines(&stdout);
    let request = commands.iter().find(|cmd| cmd.get("command").and_then(Value::as_str) == Some("request_state"));
    let id = request.and_then(|cmd| cmd.get("id").and_then(Value::as_str)).unwrap_or("");
    assert!(id.starts_with("wt-sync-"));

    let tabs_json = serde_json::json!([
        {
            "id": 1,
            "cwd": repo.display().to_string(),
            "panes": [1],
            "pane_cwds": { "1": repo.display().to_string() }
        },
        {
            "id": 2,
            "cwd": repo.join("other").display().to_string(),
            "panes": [2],
            "pane_cwds": { "2": repo.join("other").display().to_string() }
        }
    ]);
    let state_event = serde_json::json!({
        "event": "state_response",
        "data": {
            "id": id,
            "tabs": tabs_json
        }
    });

    let output = run_worktree_plugin(&state_event.to_string(), &plugin_root, &repo, &bin_dir);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let commands = parse_json_lines(&stdout);
    assert!(commands.iter().any(|cmd| cmd.get("command").and_then(Value::as_str) == Some("set_tab_title")));
    assert!(commands.iter().any(|cmd| cmd.get("command").and_then(Value::as_str) == Some("new_tab")));
    assert!(commands.iter().any(|cmd| cmd.get("command").and_then(Value::as_str) == Some("close_pane")));
}

#[cfg(unix)]
#[test]
fn test_worktree_plugin_runtime_close_flow_e2e() {
    if !has_python3() {
        return;
    }
    let temp = TempDir::new();
    let repo = temp.path.join("repo");
    fs::create_dir_all(&repo).unwrap();
    let plugin_root = temp.path.join("plugin-root");
    fs::create_dir_all(&plugin_root).unwrap();
    let bin_dir = temp.path.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    install_fake_git(&bin_dir);

    let close_event = format!(
        r#"{{"event":"plugin_command","data":{{"plugin":"worktree","command":"close","cwd":"{}"}}}}"#,
        repo.display()
    );
    let output = run_worktree_plugin(&close_event, &plugin_root, &repo, &bin_dir);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let commands = parse_json_lines(&String::from_utf8_lossy(&output.stdout));
    let confirm = find_command(&commands, "confirm").expect("expected confirm command");
    let confirm_id = confirm
        .get("id")
        .and_then(Value::as_str)
        .expect("confirm id");

    let confirm_response = serde_json::json!({
        "event": "prompt_response",
        "data": {
            "id": confirm_id,
            "ok": true
        }
    });
    let output = run_worktree_plugin(
        &confirm_response.to_string(),
        &plugin_root,
        &repo,
        &bin_dir,
    );
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let commands = parse_json_lines(&String::from_utf8_lossy(&output.stdout));
    let request = find_command(&commands, "request_state").expect("expected request_state");
    let request_id = request
        .get("id")
        .and_then(Value::as_str)
        .expect("request_state id");
    assert!(request_id.starts_with("wt-close-"));

    let tabs_json = serde_json::json!([
        {
            "id": 1,
            "cwd": repo.display().to_string(),
            "panes": [1],
            "pane_cwds": { "1": repo.display().to_string() }
        },
        {
            "id": 2,
            "cwd": format!("{}/.worktrees/feature", repo.display()),
            "panes": [2],
            "pane_cwds": { "2": format!("{}/.worktrees/feature", repo.display()) }
        }
    ]);
    let state_event = serde_json::json!({
        "event": "state_response",
        "data": {
            "id": request_id,
            "tabs": tabs_json
        }
    });
    let output = run_worktree_plugin(&state_event.to_string(), &plugin_root, &repo, &bin_dir);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let commands = parse_json_lines(&String::from_utf8_lossy(&output.stdout));
    let pick = find_command(&commands, "pick").expect("expected pick command");
    let pick_id = pick.get("id").and_then(Value::as_str).expect("pick id");

    let pick_response = serde_json::json!({
        "event": "prompt_response",
        "data": {
            "id": pick_id,
            "ok": true,
            "index": 1
        }
    });
    let output = run_worktree_plugin(&pick_response.to_string(), &plugin_root, &repo, &bin_dir);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let commands = parse_json_lines(&String::from_utf8_lossy(&output.stdout));
    let confirm = find_command(&commands, "confirm").expect("expected confirm after pick");
    let confirm_id = confirm
        .get("id")
        .and_then(Value::as_str)
        .expect("confirm id");

    let confirm_response = serde_json::json!({
        "event": "prompt_response",
        "data": {
            "id": confirm_id,
            "ok": true
        }
    });
    let output = run_worktree_plugin(
        &confirm_response.to_string(),
        &plugin_root,
        &repo,
        &bin_dir,
    );
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let commands = parse_json_lines(&String::from_utf8_lossy(&output.stdout));
    let request = find_command(&commands, "request_state")
        .expect("expected request_state after confirm");
    let request_id = request
        .get("id")
        .and_then(Value::as_str)
        .expect("request_state id");
    assert!(request_id.starts_with("wt-close-path-"));

    let state_event = serde_json::json!({
        "event": "state_response",
        "data": {
            "id": request_id,
            "tabs": tabs_json
        }
    });
    let output = run_worktree_plugin(&state_event.to_string(), &plugin_root, &repo, &bin_dir);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let commands = parse_json_lines(&String::from_utf8_lossy(&output.stdout));
    assert!(commands
        .iter()
        .any(|cmd| cmd.get("command").and_then(Value::as_str) == Some("close_pane")));
}
