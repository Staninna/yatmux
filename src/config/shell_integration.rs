use super::*;

/// Shell integration configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ShellIntegrationConfig {
    /// Track current working directory via OSC 7.
    pub cwd_from_osc7: bool,

    /// Track prompt/input/output boundaries via OSC 133.
    pub semantic_zones_from_osc133: bool,

    /// Track title changes via OSC 0/1/2.
    pub title_from_osc: bool,

    /// Controls what we show in the tab bar.
    pub tab_title_source: TabTitleSource,

    /// Updates the OS window title to match the active tab.
    pub window_title_follows_active_tab: bool,

    /// Show the current prompt at the bottom when scrolled up.
    pub sticky_prompt: bool,

    /// Shadow prompt mode - type-ahead during command execution.
    pub shadow_prompt: ShadowPromptMode,

    /// Whether shadow prompt is enabled by default for new panes.
    pub shadow_prompt_enabled_by_default: bool,

    /// Prints debug logs when shell integration signals change.
    pub debug_log: bool,
}

impl Default for ShellIntegrationConfig {
    fn default() -> Self {
        Self {
            cwd_from_osc7: true,
            semantic_zones_from_osc133: true,
            title_from_osc: true,
            tab_title_source: TabTitleSource::Cwd,
            window_title_follows_active_tab: true,
            sticky_prompt: true,
            shadow_prompt: ShadowPromptMode::default(),
            shadow_prompt_enabled_by_default: false,
            debug_log: false,
        }
    }
}

/// Source for tab bar titles.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TabTitleSource {
    None,
    Cwd,
    Title,
}

/// Shadow prompt mode - when to show type-ahead prompt during command execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ShadowPromptMode {
    /// Never show shadow prompt
    Off,
    /// Show shadow prompt immediately when command starts
    Always,
    /// Show shadow prompt only when user starts typing (default)
    #[default]
    OnTyping,
}

impl Default for TabTitleSource {
    fn default() -> Self {
        Self::Cwd
    }
}
