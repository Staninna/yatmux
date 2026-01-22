use std::collections::HashSet;
use std::path::{Path, PathBuf};


/// Timeout for plugin execution in seconds
const PLUGIN_TIMEOUT_SECS: u64 = 30;

pub(super) fn plugin_timeout_secs() -> u64 {
    #[cfg(test)]
    {
        if let Ok(raw) = std::env::var("YATMUX_TEST_PLUGIN_TIMEOUT_SECS") {
            if let Ok(value) = raw.parse::<u64>() {
                return value;
            }
        }
    }
    PLUGIN_TIMEOUT_SECS
}

/// Sanitize plugin name for use in environment variables
/// Only allows alphanumeric characters, underscores, and hyphens
pub(super) fn sanitize_plugin_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect()
}

/// Validate that plugin root is an absolute path and exists
pub(super) fn validate_plugin_root(root: &Path) -> bool {
    root.is_absolute() && root.exists()
}

pub(super) fn cwd_url_to_path(cwd_url: &str) -> Option<PathBuf> {
    let mut s = cwd_url.trim();
    if let Some(stripped) = s.strip_prefix("file://") {
        s = stripped;
    }
    if !s.starts_with('/') {
        if let Some(idx) = s.find('/') {
            s = &s[idx..];
        }
    }
    s = s.split(['?', '#']).next().unwrap_or(s);
    while s.starts_with("//") {
        s = &s[1..];
    }
    if !s.starts_with('/') {
        return None;
    }
    if s.is_empty() {
        return None;
    }
    Some(PathBuf::from(s))
}

pub(super) fn escape_shell_single_quotes(input: &str) -> String {
    input.replace('\'', "'\\''")
}

pub(super) fn should_deliver_event(
    subscriptions: &std::collections::HashMap<String, HashSet<String>>,
    plugin: &str,
    event: &str,
) -> bool {
    if matches!(event, "startup" | "shutdown") {
        return true;
    }
    let Some(set) = subscriptions.get(plugin) else {
        return false;
    };
    if set.is_empty() {
        return false;
    }
    set.contains("all") || set.contains(&event.to_lowercase())
}
