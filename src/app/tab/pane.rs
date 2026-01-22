use std::path::Path;

use winit::event_loop::EventLoopProxy;

use yatmux::renderer::TerminalView;
use yatmux::terminal::Terminal;

use crate::app::layout::{LayoutNode, PaneId, Rect, SplitDir};
use crate::app::pane::Pane;
use crate::app::tab::Tab;
use crate::app::{spawn_pty_reader, AppEvent};

impl Tab {
    /// Spawns a new pane with the given ID and scale.
    pub fn spawn_pane(
        &mut self,
        id: PaneId,
        scale: f32,
        scrollback_lines: usize,
        event_proxy: Option<&EventLoopProxy<AppEvent>>,
        tab_id: u64,
        shadow_prompt_enabled: bool,
        cwd: Option<&Path>,
        profile: String,
    ) {
        let initial_cwd = cwd.map(|path| path.to_string_lossy().to_string());
        let (pty, reader) = match yatmux::pty::spawn_shell_with_cwd(cwd) {
            Ok(result) => result,
            Err(e) => {
                eprintln!("Failed to spawn shell: {e}");
                return;
            }
        };

        let terminal = Terminal::new_with_scrollback(std::sync::Arc::new(pty), scrollback_lines);

        if let Some(proxy) = event_proxy {
            spawn_pty_reader(reader, proxy.clone(), tab_id, id);
        }

        self.panes.insert(
            id,
            Pane {
                terminal,
                view: TerminalView::new(),
                scale: scale.clamp(0.25, 64.0),
                shell_title: None,
                shell_cwd: initial_cwd,
                shell_integration: Default::default(),
                shadow_prompt: Default::default(),
                shadow_prompt_enabled,
                command_running: false,
                profile,
            },
        );
    }

    /// Spawns the initial pane for this tab.
    pub fn spawn_initial_pane(
        &mut self,
        scale: f32,
        scrollback_lines: usize,
        event_proxy: Option<&EventLoopProxy<AppEvent>>,
        shadow_prompt_enabled: bool,
        cwd: Option<&Path>,
        profile: String,
    ) {
        if !self.panes.is_empty() {
            return;
        }
        self.spawn_pane(
            1,
            scale,
            scrollback_lines,
            event_proxy,
            self.id,
            shadow_prompt_enabled,
            cwd,
            profile,
        );
    }

    /// Splits the focused pane in the given direction.
    /// Returns false if the split was rejected (e.g., pane too small).
    pub fn split_focused(
        &mut self,
        dir: SplitDir,
        default_scale: f32,
        scrollback_lines: usize,
        event_proxy: Option<&EventLoopProxy<AppEvent>>,
        current_rect: Option<Rect>,
        min_pane_size: usize,
        shadow_prompt_enabled: bool,
        cwd: Option<&Path>,
    ) -> Option<PaneId> {
        let focused = self.focused_pane;
        if !self.layout.contains_pane(focused) {
            return None;
        }

        // Check if splitting would create panes that are too small
        if let Some(rect) = current_rect {
            let (new_w, new_h) = match dir {
                SplitDir::Vertical => (rect.w / 2, rect.h),
                SplitDir::Horizontal => (rect.w, rect.h / 2),
            };
            if new_w < min_pane_size || new_h < min_pane_size {
                return None;
            }
        }

        let new_id = self.next_pane_id;
        self.next_pane_id += 1;
        let (focused_scale, parent_profile) = self
            .panes
            .get(&focused)
            .map(|p| (p.scale, p.profile.clone()))
            .unwrap_or((default_scale, "default".to_string()));

        self.spawn_pane(
            new_id,
            focused_scale,
            scrollback_lines,
            event_proxy,
            self.id,
            shadow_prompt_enabled,
            cwd,
            parent_profile,
        );

        let replacement = LayoutNode::Split {
            dir,
            ratio: 0.5,
            a: Box::new(LayoutNode::Leaf(focused)),
            b: Box::new(LayoutNode::Leaf(new_id)),
        };

        if self.layout.replace_leaf(focused, replacement) {
            self.set_focus(new_id);
            Some(new_id)
        } else {
            None
        }
    }
}
