use serde::Serialize;

#[derive(Debug, Clone, Copy)]
pub enum ActionSource {
    User,
    Plugin,
}

#[derive(Debug, Serialize)]
pub struct PluginEvent {
    pub event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pane_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}
