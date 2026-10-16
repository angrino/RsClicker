use std::fs;
use std::path::PathBuf;

use crate::atomic_file::write_atomic;
use super::model::Profile;

fn profiles_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("rsclicker").join("profiles"))
}

fn profile_path(name: &str) -> Option<PathBuf> {
    profiles_dir().map(|d| d.join(format!("{}.json", sanitize_name(name))))
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// True when both names map to the same profile file after sanitization.
pub fn same_profile_file(a: &str, b: &str) -> bool {
    sanitize_name(a) == sanitize_name(b)
}

/// Returns true if a profile file already exists for `name` after
/// sanitization. Catches both exact duplicates and sanitize collisions
/// (e.g. "a/b" vs "a:b" both map to "a_b.json").
pub fn profile_file_exists(name: &str) -> bool {
    profile_path(name)
        .map(|p| p.is_file())
        .unwrap_or(false)
}

pub fn ensure_dir() -> bool {
    match profiles_dir() {
        Some(dir) => fs::create_dir_all(dir).is_ok(),
        None => false,
    }
}

pub fn save_profile(profile: &Profile) -> Result<(), String> {
    ensure_dir();
    let path = profile_path(&profile.name).ok_or("config dir unavailable")?;
    let data = serde_json::to_string_pretty(profile).map_err(|e| e.to_string())?;
    write_atomic(&path, &data)
}

pub fn load_profile(name: &str) -> Result<Profile, String> {
    let path = profile_path(name).ok_or("config dir unavailable")?;
    let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

pub fn list_profiles() -> Vec<String> {
    let dir = match profiles_dir() {
        Some(d) => d,
        None => return Vec::new(),
    };
    let entries = fs::read_dir(dir).ok();
    let mut names = Vec::new();
    if let Some(entries) = entries {
        for entry in entries.flatten() {
            if let Some(stem) = entry.path().file_stem().and_then(|s| s.to_str()) {
                if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                    names.push(stem.to_string());
                }
            }
        }
    }
    names.sort();
    names
}

#[allow(dead_code)]
pub fn delete_profile(name: &str) -> Result<(), String> {
    let path = profile_path(name).ok_or("config dir unavailable")?;
    fs::remove_file(path).map_err(|e| e.to_string())
}
