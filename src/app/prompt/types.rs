#[derive(Debug, Clone)]
pub enum PromptKind {
    Input,
    Confirm,
    Pick,
}

#[derive(Debug, Clone)]
pub struct PromptState {
    pub id: String,
    pub title: String,
    pub message: Option<String>,
    pub kind: PromptKind,
    pub input: String,
    pub default_value: Option<String>,
    pub items: Vec<String>,
    pub selected: usize,
    pub ok_label: String,
    pub cancel_label: String,
}

pub struct PromptResolution {
    pub ok: bool,
    pub value: Option<String>,
    pub index: Option<usize>,
    pub reason: Option<String>,
}

#[derive(Default)]
pub struct PromptUpdate {
    pub resolution: Option<PromptResolution>,
    pub needs_redraw: bool,
}

impl PromptResolution {
    pub fn submit(ok: bool, value: Option<String>, index: Option<usize>) -> Self {
        Self {
            ok,
            value,
            index,
            reason: None,
        }
    }

    pub fn cancel() -> Self {
        Self {
            ok: false,
            value: None,
            index: None,
            reason: None,
        }
    }
}

impl PromptState {
    pub fn input(
        id: String,
        title: String,
        message: Option<String>,
        default_value: Option<String>,
    ) -> Self {
        Self {
            id,
            title,
            message,
            kind: PromptKind::Input,
            input: String::new(),
            default_value,
            items: Vec::new(),
            selected: 0,
            ok_label: "OK".to_string(),
            cancel_label: "Cancel".to_string(),
        }
    }

    pub fn confirm(
        id: String,
        title: String,
        message: Option<String>,
        ok_label: Option<String>,
        cancel_label: Option<String>,
    ) -> Self {
        Self {
            id,
            title,
            message,
            kind: PromptKind::Confirm,
            input: String::new(),
            default_value: None,
            items: Vec::new(),
            selected: 0,
            ok_label: ok_label.unwrap_or_else(|| "OK".to_string()),
            cancel_label: cancel_label.unwrap_or_else(|| "Cancel".to_string()),
        }
    }

    pub fn pick(
        id: String,
        title: String,
        message: Option<String>,
        items: Vec<String>,
        selected: Option<usize>,
    ) -> Self {
        let selected = selected.unwrap_or(0).min(items.len().saturating_sub(1));
        Self {
            id,
            title,
            message,
            kind: PromptKind::Pick,
            input: String::new(),
            default_value: None,
            items,
            selected,
            ok_label: "Select".to_string(),
            cancel_label: "Cancel".to_string(),
        }
    }

    pub fn kind_label(&self) -> &'static str {
        match self.kind {
            PromptKind::Input => "input",
            PromptKind::Confirm => "confirm",
            PromptKind::Pick => "pick",
        }
    }
}
