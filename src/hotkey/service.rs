use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use global_hotkey::hotkey::HotKey;

pub struct HotkeyService {
    manager: Option<GlobalHotKeyManager>,
    registered: Vec<HotKey>,
    pub last_error: Option<String>,
}

impl Default for HotkeyService {
    fn default() -> Self {
        Self::new()
    }
}

impl HotkeyService {
    pub fn new() -> Self {
        let manager = GlobalHotKeyManager::new().ok();
        Self {
            manager,
            registered: Vec::new(),
            last_error: None,
        }
    }

    pub fn register_hotkey(&mut self, hotkey: HotKey) -> Result<(), String> {
        match &self.manager {
            Some(manager) => {
                if self.registered.iter().any(|h| h.id() == hotkey.id()) {
                    return Ok(());
                }
                manager.register(hotkey).map_err(|e| e.to_string())?;
                self.registered.push(hotkey);
                self.last_error = None;
                Ok(())
            }
            None => {
                let msg = "hotkey backend unavailable on this session".to_string();
                self.last_error = Some(msg.clone());
                Err(msg)
            }
        }
    }
    /// Unregisters one concrete hotkey value.
    pub fn unregister_key(&mut self, hotkey: HotKey) {
        if let Some(manager) = &self.manager {
            let _ = manager.unregister(hotkey);
        }
        self.registered.retain(|h| h.id() != hotkey.id());
    }

    pub fn poll_events(&self) -> Vec<(u32, HotKeyState)> {
        let mut out = Vec::new();
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            out.push((event.id, event.state));
        }
        out
    }
}
