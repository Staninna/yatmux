use super::*;

/// Interaction tuning.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct InteractionConfig {
    pub click_move_max_steps: usize,
    pub pane_resize_step: f32,
    pub focus_move_overlap_weight: i64,
}

impl Default for InteractionConfig {
    fn default() -> Self {
        Self {
            click_move_max_steps: 512,
            pane_resize_step: 0.05,
            focus_move_overlap_weight: 1000,
        }
    }
}
