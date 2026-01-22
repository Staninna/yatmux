use serde::Deserialize;

use yatmux::config::{Action, KeybindAction};

#[derive(Debug, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum PluginCommand {
    Action { action: Action },
    Toast { message: String },
    SetTabTitle { title: String, tab_id: Option<u64> },
    SetWindowTitle { title: String },
    NewTab { cwd: String, title: Option<String> },
    SetTabCwd { cwd: String, tab_id: Option<u64> },
    SetPaneCwd {
        cwd: String,
        tab_id: Option<u64>,
        pane_id: Option<u64>,
    },
    SetPaneProfile {
        profile: String,
        tab_id: Option<u64>,
        pane_id: Option<u64>,
    },
    Prompt {
        id: String,
        title: String,
        message: Option<String>,
        default: Option<String>,
    },
    Confirm {
        id: String,
        title: String,
        message: Option<String>,
        ok_label: Option<String>,
        cancel_label: Option<String>,
    },
    Pick {
        id: String,
        title: String,
        message: Option<String>,
        items: Vec<String>,
        selected: Option<usize>,
    },
    RequestState { id: String },
    ClipboardRead { id: String },
    ClipboardWrite { text: String },
    SendText {
        text: String,
        tab_id: Option<u64>,
        pane_id: Option<u64>,
    },
    FocusTab { tab_id: u64 },
    CloseTab { tab_id: u64 },
    ClosePane { tab_id: u64, pane_id: u64 },
    Subscribe { events: Vec<String> },
    ConfigPatch { toml: String, persist: Option<bool> },
    ReloadConfig,
    PluginCommand { name: String, args: Option<serde_json::Value> },
    RegisterKeybind {
        key: String,
        action: KeybindAction,
        persist: Option<bool>,
    },
}
