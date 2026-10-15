use global_hotkey::hotkey::{Code, HotKey, Modifiers};

pub fn parse_hotkey(text: &str) -> Result<HotKey, String> {
    let normalized = text.trim().replace(' ', "");
    if normalized.is_empty() {
        return Err("empty hotkey".to_string());
    }
    normalized
        .parse::<HotKey>()
        .map_err(|e| e.to_string())
}

#[allow(dead_code)]
pub fn hotkey_to_string(hotkey: &HotKey) -> String {
    hotkey.to_string()
}

#[allow(dead_code)]
pub fn default_toggle() -> HotKey {
    HotKey::new(None, Code::F6)
}

#[allow(dead_code)]
pub fn example_keys() -> &'static [&'static str] {
    &["F6", "F7", "F8", "Control+F6", "Alt+R", "Shift+R", "R"]
}

#[allow(dead_code)]
pub fn describe_modifiers(mods: Modifiers) -> String {
    let mut parts = Vec::new();
    if mods.contains(Modifiers::CONTROL) {
        parts.push("Ctrl");
    }
    if mods.contains(Modifiers::SHIFT) {
        parts.push("Shift");
    }
    if mods.contains(Modifiers::ALT) {
        parts.push("Alt");
    }
    if mods.contains(Modifiers::SUPER) {
        parts.push("Super");
    }
    parts.join("+")
}
