//! App-level settings (theme base + accent + profiles view mode),
//! persisted as JSON next to the profiles.

use std::fs;
use std::path::PathBuf;

use crate::atomic_file::write_atomic;
use serde::{Deserialize, Serialize};

use crate::theme::{Accent, AppTheme, BaseTheme, ViewMode};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default)]
    pub base: BaseTheme,
    #[serde(default)]
    pub accent: Accent,
    #[serde(default)]
    pub view: ViewMode,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            base: BaseTheme::Dark,
            accent: Accent::Orange,
            view: ViewMode::List,
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
