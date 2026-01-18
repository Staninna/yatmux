use super::*;

/// Main configuration structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: ThemeConfig,

    pub window: WindowConfig,
    pub colors: ColorConfig,
    pub terminal: TerminalConfig,
    pub shell_integration: ShellIntegrationConfig,
    pub font: FontConfig,
    pub pane: PaneConfig,
    pub keybinds: KeybindConfig,

    pub ui: UiConfig,
    pub interaction: InteractionConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            theme: ThemeConfig::default(),
            window: WindowConfig::default(),
            colors: ColorConfig::default(),
            terminal: TerminalConfig::default(),
            shell_integration: ShellIntegrationConfig::default(),
            font: FontConfig::default(),
            pane: PaneConfig::default(),
            keybinds: KeybindConfig::default(),
            ui: UiConfig::default(),
            interaction: InteractionConfig::default(),
        }
    }
}

impl Config {
    pub fn apply_defaults(&mut self) {
        self.keybinds.apply_defaults();

        // Keep rendering/input assumptions intact.
        self.font.scale = self.font.scale.clamp(1.0, 8.0);

        self.terminal.rows = self.terminal.rows.max(1);
        self.terminal.cols = self.terminal.cols.max(1);
        self.terminal.scrollback_lines = self.terminal.scrollback_lines.max(1);
        self.terminal.tab_width = self.terminal.tab_width.max(1);

        if !self.terminal.scroll_speed.is_finite() || self.terminal.scroll_speed <= 0.0 {
            self.terminal.scroll_speed = SCROLL_SPEED_MULTIPLIER;
        }

        // UI safety clamps.
        self.ui.toast.duration_ms = self.ui.toast.duration_ms.min(60_000);
        self.ui.search.right_reserved_px = self.ui.search.right_reserved_px.min(2_000);
        self.ui.tab_bar.gap_px = self.ui.tab_bar.gap_px.min(128);
        self.ui.tab_bar.side_padding_px = self.ui.tab_bar.side_padding_px.min(256);
        self.ui.tab_bar.max_width_cells = self.ui.tab_bar.max_width_cells.clamp(4, 200);
        self.ui.tab_bar.max_width_px_extra = self.ui.tab_bar.max_width_px_extra.min(512);

        self.interaction.click_move_max_steps =
            self.interaction.click_move_max_steps.clamp(1, 10_000);
        if !self.interaction.pane_resize_step.is_finite()
            || self.interaction.pane_resize_step <= 0.0
        {
            self.interaction.pane_resize_step = InteractionConfig::default().pane_resize_step;
        }
        self.interaction.pane_resize_step = self.interaction.pane_resize_step.clamp(0.005, 0.5);
        self.interaction.focus_move_overlap_weight = self
            .interaction
            .focus_move_overlap_weight
            .clamp(1, 1_000_000);
    }
}
