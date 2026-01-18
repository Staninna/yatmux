//! Terminal rendering and terminal view state.
//!
//! This module provides:
//! - `TerminalView`: UI state management (scrolling, selection, URLs, search)
//! - `Renderer`: Pixel painting for terminal frames
//!
//! Scrollback lives in the terminal model (`wezterm-term`). The view asks the
//! terminal for a snapshot of the *visible* rows. Only when search needs to
//! (re)index matches do we build a full snapshot of the scrollback.

mod color;
pub mod font;
mod help;
mod painter;
mod view;

pub use crate::core::search::{SearchMatch, SearchState};
pub use color::{create_palette, create_palette_with_ansi};
pub use painter::Renderer;
pub use view::TerminalView;

use crate::core::grid::RowSnapshot;

fn clamp_u8(v: i32) -> u8 {
    v.clamp(0, 255) as u8
}

fn rgb_split(rgb: u32) -> (u8, u8, u8) {
    (
        ((rgb >> 16) & 0xFF) as u8,
        ((rgb >> 8) & 0xFF) as u8,
        (rgb & 0xFF) as u8,
    )
}

fn rgb_join(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

fn mix(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let (ar, ag, ab) = rgb_split(a);
    let (br, bg, bb) = rgb_split(b);
    let r = (ar as f32 + (br as f32 - ar as f32) * t).round() as i32;
    let g = (ag as f32 + (bg as f32 - ag as f32) * t).round() as i32;
    let b = (ab as f32 + (bb as f32 - ab as f32) * t).round() as i32;
    rgb_join(clamp_u8(r), clamp_u8(g), clamp_u8(b))
}

fn darken(rgb: u32, amount: f32) -> u32 {
    mix(rgb, 0x000000, amount)
}

/// Concrete UI style used by the renderer.
#[derive(Clone, Debug)]
pub struct UiStyle {
    pub base_bg: u32,
    pub base_fg: u32,
    pub accent: u32,

    pub tab_bar_bg: u32,
    pub tab_bar_border: u32,
    pub tab_inactive_bg: u32,
    pub tab_inactive_text: u32,
    pub tab_gap_px: usize,
    pub tab_side_padding_px: usize,
    pub tab_max_width_cells: usize,
    pub tab_max_width_px_extra: usize,

    pub divider: u32,

    pub search_match_bg: u32,
    pub search_current_bg: u32,
    pub search_bar_bg: u32,
    pub search_bar_text: u32,
    pub search_bar_hint_text: u32,
    pub search_invalid_regex_text: u32,
    pub search_right_reserved_px: usize,

    pub help_bg: u32,
    pub help_text: u32,
    pub help_footer_text: u32,
    pub help_padding_x_cells: usize,
    pub help_padding_y_cells: usize,
    pub help_font_scale_max: usize,

    pub toast_bg: u32,
    pub toast_text: u32,
    pub toast_border: u32,
    pub toast_bottom_margin_cells: usize,
    pub toast_font_scale_override: Option<usize>,
    pub toast_font_scale_max: usize,

    pub sticky_prompt_bg: u32,
    pub sticky_prompt_separator: u32,

    pub context_menu_bg: u32,
    pub context_menu_hover_bg: u32,
    pub context_menu_text: u32,
    pub context_menu_border: u32,

    pub url_fg: u32,

    pub shadow_prompt_bg: u32,
    pub shadow_prompt_text: u32,
    pub shadow_prompt_cursor: u32,
    pub shadow_prompt_indicator: u32,
    pub shadow_prompt_border: u32,
}

impl UiStyle {
    pub fn from_config(config: &crate::config::Config) -> Self {
        let bg = config.colors.background;
        let fg = config.colors.foreground;
        let accent = config.colors.accent;

        let ui = &config.ui;

        // Tab bar
        let tab_bar_bg = ui.tab_bar.background.unwrap_or(darken(bg, 0.15));
        let tab_bar_border = ui.tab_bar.border.unwrap_or(darken(fg, 0.7));
        let tab_inactive_bg = ui
            .tab_bar
            .inactive_tab_background
            .unwrap_or(darken(bg, 0.08));
        let tab_inactive_text = ui.tab_bar.inactive_text.unwrap_or(darken(fg, 0.35));

        // Dividers / pane borders
        // Default is subtle but visible; override via `[ui.dividers].color`.
        let divider = ui.dividers.color.unwrap_or(mix(bg, fg, 0.08));

        // Search
        let search_match_bg = ui.search.match_bg.unwrap_or(mix(bg, accent, 0.28));
        let search_current_bg = ui.search.current_match_bg.unwrap_or(mix(bg, accent, 0.45));
        let search_bar_bg = ui.search.bar_bg.unwrap_or(darken(bg, 0.22));
        let search_bar_text = ui.search.bar_text.unwrap_or(fg);
        let search_bar_hint_text = ui.search.bar_hint_text.unwrap_or(darken(fg, 0.45));
        let search_invalid_regex_text = ui.search.invalid_regex_text.unwrap_or(0xFF6666);

        // Help
        let help_bg = ui.help.background.unwrap_or(darken(bg, 0.22));
        let help_text = ui.help.text.unwrap_or(fg);
        let help_footer_text = ui.help.footer_text.unwrap_or(darken(fg, 0.45));
        let help_font_scale_max = ui.help.font_scale.unwrap_or(8).clamp(1, 8);

        // Toast
        let toast_bg = ui.toast.background.unwrap_or(darken(bg, 0.22));
        let toast_text = ui.toast.text.unwrap_or(fg);
        let toast_border = ui.toast.border.unwrap_or(darken(fg, 0.70));
        let toast_font_scale_override = ui.toast.font_scale.map(|s| s.clamp(1, 8));
        let toast_font_scale_max = toast_font_scale_override.unwrap_or(help_font_scale_max);

        // Sticky prompt
        let sticky_prompt_bg = ui.sticky_prompt.background.unwrap_or(darken(bg, 0.22));
        let sticky_prompt_separator = ui.sticky_prompt.separator.unwrap_or(darken(fg, 0.70));

        // Context menu
        let context_menu_bg = ui.context_menu.background.unwrap_or(darken(bg, 0.22));
        let context_menu_hover_bg = ui
            .context_menu
            .hover_background
            .unwrap_or(mix(bg, accent, 0.22));
        let context_menu_text = ui.context_menu.text.unwrap_or(fg);
        let context_menu_border = ui.context_menu.border.unwrap_or(darken(fg, 0.70));

        let url_fg = accent;

        // Shadow prompt
        let shadow_prompt_bg = ui.shadow_prompt.background.unwrap_or(mix(bg, accent, 0.10));
        let shadow_prompt_text = ui.shadow_prompt.text.unwrap_or(fg);
        let shadow_prompt_cursor = ui.shadow_prompt.cursor.unwrap_or(accent);
        let shadow_prompt_indicator = ui
            .shadow_prompt
            .prompt_indicator
            .unwrap_or(mix(fg, accent, 0.55));
        let shadow_prompt_border = ui.shadow_prompt.border.unwrap_or(darken(fg, 0.70));

        Self {
            base_bg: bg,
            base_fg: fg,
            accent,

            tab_bar_bg,
            tab_bar_border,
            tab_inactive_bg,
            tab_inactive_text,
            tab_gap_px: ui.tab_bar.gap_px,
            tab_side_padding_px: ui.tab_bar.side_padding_px,
            tab_max_width_cells: ui.tab_bar.max_width_cells,
            tab_max_width_px_extra: ui.tab_bar.max_width_px_extra,

            divider,

            search_match_bg,
            search_current_bg,
            search_bar_bg,
            search_bar_text,
            search_bar_hint_text,
            search_invalid_regex_text,
            search_right_reserved_px: ui.search.right_reserved_px,

            help_bg,
            help_text,
            help_footer_text,
            help_padding_x_cells: ui.help.padding_x_cells,
            help_padding_y_cells: ui.help.padding_y_cells,
            help_font_scale_max,

            toast_bg,
            toast_text,
            toast_border,
            toast_bottom_margin_cells: ui.toast.bottom_margin_cells,
            toast_font_scale_override,
            toast_font_scale_max,

            sticky_prompt_bg,
            sticky_prompt_separator,

            context_menu_bg,
            context_menu_hover_bg,
            context_menu_text,
            context_menu_border,

            url_fg,

            shadow_prompt_bg,
            shadow_prompt_text,
            shadow_prompt_cursor,
            shadow_prompt_indicator,
            shadow_prompt_border,
        }
    }
}

/// A frame of terminal content ready for rendering.
pub(crate) struct RenderFrame {
    pub cursor: (u16, u16),
    pub display_rows: Vec<RowSnapshot>,
    pub rows: usize,
    pub cols: usize,
    pub view_start: usize,
    pub show_cursor: bool,
}

/// A categorized list of key bindings for the help overlay.
#[derive(Clone, Debug)]
pub struct HelpSection {
    pub title: String,
    pub bindings: Vec<(String, String)>,
}
