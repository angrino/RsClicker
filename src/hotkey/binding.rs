use global_hotkey::hotkey::HotKey;

pub fn parse_hotkey(text: &str) -> Result<HotKey, String> {
    let normalized = text.trim().replace(' ', "");
    if normalized.is_empty() {
        return Err("empty hotkey".to_string());
    }
    normalized
        .parse::<HotKey>()
        .map_err(|e| e.to_string())
}
