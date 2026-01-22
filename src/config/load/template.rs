use std::fs;
use std::path::Path;

use super::super::*;

impl Config {
    pub(super) fn write_default_template(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, Self::default_config_template_for_path(path))?;
        Ok(())
    }

    fn join_theme_names() -> String {
        let mut s = String::new();
        for (i, name) in Self::builtin_theme_names().iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            if *name == "dracula" {
                s.push_str("dracula (default)");
            } else {
                s.push_str(name);
            }
        }
        s
    }

    pub(super) fn default_config_template_for_path(path: &Path) -> String {
        let mut out = String::new();

        out.push_str("# yatmux configuration\n");
        out.push_str("#\n");
        out.push_str("# Location: ");
        out.push_str(&format!("{}", path.display()));
        out.push_str("\n");
        out.push_str("#\n");
        out.push_str("# Apply changes by restarting yatmux or using Reload config (default: ctrl+shift+r).\n");
        out.push_str("#\n");
        out.push_str("# UI color precedence: theme > [ui] > [colors]\n");
        out.push_str("#\n");
        out.push_str("# Color formats:\n");
        out.push_str("# - \"#RGB\" / \"#RRGGBB\" strings\n");
        out.push_str("# - 0xRRGGBB integers\n");
        out.push_str("#\n");
        out.push_str("# Built-in themes: ");
        out.push_str(&Self::join_theme_names());
        out.push_str("\n\n");

        out.push_str("[theme]\n");
        out.push_str("# Pick a theme by name.\n");
        out.push_str("#\n");
        out.push_str("# Examples:\n");
        out.push_str("# name = \"dracula\"\n");
        out.push_str("# name = \"light\"\n");
        out.push_str("# name = \"gruvbox-dark\"\n");
        out.push_str("#\n");
        out.push_str("# Set to \"\" (empty) or \"off\" to disable theme loading.\n");
        out.push_str("name = \"dracula\"\n");
        out.push_str("# Merge extra files before this config.toml.\n");
        out.push_str(
            "# imports = [\"./local-overrides.toml\", \"~/dotfiles/yatmux/common.toml\"]\n",
        );
        out.push_str("imports = []\n\n");

        out.push_str("[window]\n");
        out.push_str("# title = \"yatmux\"\n\n");

        out.push_str("[terminal]\n");
        out.push_str("# rows = 24\n");
        out.push_str("# cols = 80\n");
        out.push_str("# scrollback_lines = 4096\n");
        out.push_str("# scroll_speed = 3.0\n");
        out.push_str("# tab_width = 8\n\n");

        out.push_str("[font]\n");
        out.push_str("# scale = 1.0  # Font scale (1.0-8.0, accepts decimals like 1.5, 2.25)\n\n");

        out.push_str("[pane]\n");
        out.push_str("# Pane layout tweaks. Values are in pixels.\n");
        out.push_str("# Padding adds space between the pane border and the terminal grid.\n");
        out.push_str("#\n");
        out.push_str("# padding = 8\n");
        out.push_str("# padding_left = 8\n");
        out.push_str("# padding_right = 8\n");
        out.push_str("# padding_top = 8\n");
        out.push_str("# padding_bottom = 8\n");
        out.push_str("#\n");
        out.push_str("# Prevent splitting panes too small:\n");
        out.push_str("# min_size = 100\n\n");
        out.push_str("# Inherit focused pane cwd on split:\n");
        out.push_str("# inherit_cwd_on_split = true\n\n");

        out.push_str("[shell_integration]\n");
        out.push_str("# cwd_from_osc7 = true\n");
        out.push_str("# semantic_zones_from_osc133 = true\n");
        out.push_str("# title_from_osc = true\n");
        out.push_str("# tab_title_source = \"cwd\" # none|cwd|title\n");
        out.push_str("# window_title_follows_active_tab = true\n");
        out.push_str("# sticky_prompt = true\n");
        out.push_str("# shadow_prompt = \"on_typing\" # off|always|on_typing\n");
        out.push_str("# shadow_prompt_enabled_by_default = false\n");
        out.push_str("# debug_log = false\n\n");

        out.push_str("[colors]\n");
        out.push_str("# Base colors for the terminal and as a fallback for UI chrome.\n");
        out.push_str("#\n");
        out.push_str("# With a theme enabled, themes take precedence over [colors] for UI (and often terminal).\n");
        out.push_str("# To fully control colors, disable themes and set these explicitly.\n");
        out.push_str("#\n");
        out.push_str("# background = \"#101010\"\n");
        out.push_str("# foreground = \"#D0D0D0\"\n");
        out.push_str("# accent = \"#66AAFF\"\n");
        out.push_str("#\n");
        out.push_str(
            "# Optional 16-color ANSI palette (colors 0-15). Must have exactly 16 entries:\n",
        );
        out.push_str("# palette = [ \"#000000\", \"#800000\", ... ]\n\n");

        out.push_str("[ui]\n");
        out.push_str("# UI chrome settings (tab bar, overlays, borders).\n");
        out.push_str("# These override [colors] but are overridden by theme files.\n\n");

        out.push_str("[ui.tab_bar]\n");
        out.push_str("# gap_px = 4\n");
        out.push_str("# side_padding_px = 8\n");
        out.push_str("# max_width_cells = 12\n");
        out.push_str("# max_width_px_extra = 16\n");
        out.push_str("# vertical_padding_px = 8\n");
        out.push_str("# tab_internal_padding_px = 16\n");
        out.push_str("# min_height_cells = 2  # Optional: minimum height in character cells\n");
        out.push_str("# font_scale = 2.0  # Tab text scale (1.0-8.0, accepts decimals)\n\n");

        out.push_str("[ui.search]\n");
        out.push_str("# right_reserved_px = 100\n");
        out.push_str("# match_bg = \"#4A4A00\"\n");
        out.push_str("# current_match_bg = \"#806000\"\n\n");

        out.push_str("[ui.toast]\n");
        out.push_str("# duration_ms = 1500\n");
        out.push_str("# bottom_margin_cells = 2\n");
        out.push_str(
            "# font_scale = 2.0 # Toast font scale (1.0-8.0, omit for auto, accepts decimals)\n\n",
        );

        out.push_str("[ui.help]\n");
        out.push_str("# padding_x_cells = 2\n");
        out.push_str("# padding_y_cells = 0\n");
        out.push_str("# font_scale = 8.0 # Help overlay font scale (1.0-8.0, accepts decimals)\n\n");

        out.push_str("[ui.dividers]\n");
        out.push_str("# Pane borders + split lines.\n");
        out.push_str("# - Inactive panes use this color for a thin outline.\n");
        out.push_str("# - The focused pane gets an additional accent border.\n");
        out.push_str("#\n");
        out.push_str("# color = \"#222\"\n\n");

        out.push_str("[interaction]\n");
        out.push_str("# click_move_max_steps = 512\n");
        out.push_str("# pane_resize_step = 0.05\n");
        out.push_str("# focus_move_overlap_weight = 1000\n\n");

        out.push_str("[plugins]\n");
        out.push_str("# enabled = true\n");
        out.push_str("# enable_default_dir = true\n");
        out.push_str("# paths = [\"~/dotfiles/yatmux/plugins\", \"./local-plugins\"]\n\n");
        out.push_str("[keybinds]\n");
        out.push_str("# \"ctrl+shift+r\" = \"reload_config\"\n");
        out.push_str("# \"ctrl+shift+-\" = \"none\"\n");

        out.push_str("\n# Troubleshooting\n");
        out.push_str("# - Theme not changing? You may have [colors]/[ui] overrides; disable theme or remove overrides.\n");
        out.push_str("# - ANSI colors wrong? [colors].palette must have exactly 16 entries.\n");

        out
    }
}
