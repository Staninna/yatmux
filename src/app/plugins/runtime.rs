use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

#[cfg(not(test))]
use wait_timeout::ChildExt;

use super::command::PluginCommand;
use super::manager::Plugin;
use super::utils::{plugin_timeout_secs, sanitize_plugin_name, validate_plugin_root};

pub(super) fn run_plugin(
    plugin: &Plugin,
    payload: &str,
    config_path: Option<&Path>,
) -> Option<Vec<PluginCommand>> {
    // Validate plugin root path
    if !validate_plugin_root(&plugin.root) {
        eprintln!(
            "Plugin {} has invalid root path: {}",
            plugin.name,
            plugin.root.display()
        );
        return None;
    }

    let mut child = match Command::new("bash")
        .arg(&plugin.script)
        .env("YATMUX_PLUGIN_EVENT", &payload)
        .env("YATMUX_PLUGIN_NAME", &sanitize_plugin_name(&plugin.name))
        .env("YATMUX_PLUGIN_ROOT", &plugin.root)
        .env(
            "YATMUX_CONFIG_PATH",
            config_path.unwrap_or_else(|| Path::new("")).as_os_str(),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            eprintln!(
                "Failed to start plugin {} ({}): {e}",
                plugin.name,
                plugin.script.display()
            );
            return None;
        }
    };

    // Take stdout before waiting
    let stdout_handle = child.stdout.take();

    if let Some(mut stdin) = child.stdin.take() {
        if let Err(e) = stdin.write_all(payload.as_bytes()) {
            eprintln!(
                "Failed to write plugin event to {}: {e}",
                plugin.script.display()
            );
        }
    }

    // Wait for plugin with timeout
    let timeout = Duration::from_secs(plugin_timeout_secs());
    let status = match wait_for_child(&mut child, timeout) {
        Ok(Some(status)) => status,
        Ok(None) => {
            // Timeout occurred
            eprintln!(
                "Plugin {} timed out after {}s, killing process",
                plugin.name,
                timeout.as_secs()
            );
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        Err(e) => {
            eprintln!("Failed to wait for plugin {}: {e}", plugin.name);
            return None;
        }
    };

    // Read output after process has finished
    let stdout = if let Some(mut handle) = stdout_handle {
        let mut output = Vec::new();
        if let Err(e) = std::io::Read::read_to_end(&mut handle, &mut output) {
            eprintln!("Failed to read plugin output from {}: {e}", plugin.name);
            return None;
        }
        String::from_utf8_lossy(&output).to_string()
    } else {
        String::new()
    };

    if !status.success() {
        eprintln!("Plugin {} exited with status {}", plugin.name, status);
    }

    Some(parse_plugin_commands(&stdout))
}

fn wait_for_child(
    child: &mut std::process::Child,
    timeout: Duration,
) -> std::io::Result<Option<std::process::ExitStatus>> {
    #[cfg(test)]
    {
        let start = std::time::Instant::now();
        loop {
            match child.try_wait()? {
                Some(status) => return Ok(Some(status)),
                None => {
                    if start.elapsed() >= timeout {
                        return Ok(None);
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        }
    }
    #[cfg(not(test))]
    {
        child.wait_timeout(timeout)
    }
}

pub(super) fn parse_plugin_commands(stdout: &str) -> Vec<PluginCommand> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    if trimmed.starts_with('[') {
        match serde_json::from_str::<Vec<PluginCommand>>(trimmed) {
            Ok(commands) => return commands,
            Err(e) => {
                eprintln!("Plugin output JSON array invalid: {e}");
                return Vec::new();
            }
        }
    }

    let mut commands = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<PluginCommand>(line) {
            Ok(command) => commands.push(command),
            Err(e) => eprintln!("Plugin output JSON invalid: {e}"),
        }
    }
    commands
}
