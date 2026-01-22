/// A keybind specification.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Keybind {
    pub key: String,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Keybind {
    /// Parses a keybind string like "ctrl+shift+c" or "f12".
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.to_lowercase();
        let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();

        let mut ctrl = false;
        let mut shift = false;
        let mut alt = false;
        let mut key = None;

        for part in parts {
            match part {
                "ctrl" | "control" => ctrl = true,
                "shift" => shift = true,
                "alt" | "meta" => alt = true,
                k => key = Some(k.to_string()),
            }
        }

        key.map(|k| Keybind {
            key: k,
            ctrl,
            shift,
            alt,
        })
    }

    /// Checks if this keybind matches the given key and modifiers.
    pub fn matches(&self, key: &str, ctrl: bool, shift: bool, alt: bool) -> bool {
        self.key.eq_ignore_ascii_case(key)
            && self.ctrl == ctrl
            && self.shift == shift
            && self.alt == alt
    }
}
