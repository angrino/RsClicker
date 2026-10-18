//! App-level settings (theme base + accent + profiles view mode),
//! persisted as JSON next to the profiles.

use std::fs;
use std::path::PathBuf;

use crate::atomic_file::write_atomic;
use crate::clicker::InjectionBackend;
use serde::{Deserialize, Serialize};

use crate::theme::{Accent, AppTheme, BaseTheme, ViewMode};

/// Which injection backend performs clicks on Linux. `Uinput` is the
/// kernel-level alternative for systems whose desktop portals misbehave;
/// `Enigo` stays the default (XTEST on X11, portals on Wayland).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum InputBackend {
    #[default]
    Enigo,
    Uinput,
}

impl InputBackend {
    #[cfg(target_os = "linux")]
    pub fn label(self) -> &'static str {
        match self {
            InputBackend::Enigo => "enigo",
            InputBackend::Uinput => "uinput",
        }
    }

    /// Setting → engine backend. On Windows the engine has only the
    /// enigo/SendInput path, so the setting is ignored there.
    pub fn to_engine_backend(self) -> InjectionBackend {
        #[cfg(target_os = "linux")]
        {
            match self {
                InputBackend::Enigo => InjectionBackend::Enigo,
                InputBackend::Uinput => InjectionBackend::Uinput,
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = self;
            InjectionBackend::Enigo
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default)]
    pub base: BaseTheme,
    #[serde(default)]
    pub accent: Accent,
    #[serde(default)]
    pub view: ViewMode,
    #[serde(default)]
    pub input_backend: InputBackend,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            base: BaseTheme::Dark,
            accent: Accent::Orange,
            view: ViewMode::List,
            input_backend: InputBackend::Enigo,
        }
    }
}

impl AppSettings {
    pub fn theme(&self) -> iced::Theme {
        AppTheme {
            base: self.base,
            accent: self.accent,
        }
        .theme()
    }
}

fn settings_path() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("rsclicker").join("settings.json"))
}

pub fn load_settings() -> AppSettings {
    let path = match settings_path() {
        Some(p) => p,
        None => return AppSettings::default(),
    };
    let data = match fs::read_to_string(path) {
        Ok(d) => d,
        Err(_) => return AppSettings::default(),
    };
    serde_json::from_str(&data).unwrap_or_default()
}

pub fn save_settings(settings: &AppSettings) -> Result<(), String> {
    let path = settings_path().ok_or("config dir unavailable")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let data = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    write_atomic(&path, &data)
}

#[cfg(test)]
mod tests {
    use super::{AppSettings, InputBackend};

    #[test]
    fn missing_field_loads_default_backend() {
        let s: AppSettings = serde_json::from_str(r#"{"base": "Dark"}"#).unwrap();
        assert_eq!(s.input_backend, InputBackend::Enigo);
    }

    #[test]
    fn roundtrip_preserves_backend() {
        let s: AppSettings =
            serde_json::from_str(r#"{"input_backend": "Uinput"}"#).unwrap();
        assert_eq!(s.input_backend, InputBackend::Uinput);
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<AppSettings>(&json).unwrap(), s);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn mapping_reaches_engine_backend() {
        assert_eq!(
            InputBackend::Enigo.to_engine_backend(),
            crate::clicker::InjectionBackend::Enigo
        );
        assert_eq!(
            InputBackend::Uinput.to_engine_backend(),
            crate::clicker::InjectionBackend::Uinput
        );
    }
}
