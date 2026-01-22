/// A filtered help section with match information.
#[derive(Debug, Clone)]
pub struct FilteredHelpSection {
    pub title: String,
    pub bindings: Vec<FilteredBinding>,
}

/// A filtered keybinding with match highlights.
#[derive(Debug, Clone)]
pub struct FilteredBinding {
    pub key: String,
    pub action: String,
    pub score: i32,
}
