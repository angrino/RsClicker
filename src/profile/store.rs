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

/// DOS device names are reserved on Windows even with an extension
/// (pre-Win11 semantics — our targets include Win10): writing e.g.
/// `NUL.json` reports success while going nowhere. Such sanitized names get
/// a trailing `_` on every platform.
fn is_reserved_name(name: &str) -> bool {
    let normalized: String = name
        .chars()
        .map(|c| match c {
            '⁰' => '0',
            '¹' => '1',
            '²' => '2',
            '³' => '3',
            '⁴' => '4',
            '⁵' => '5',
            '⁶' => '6',
            '⁷' => '7',
            '⁸' => '8',
            '⁹' => '9',
            _ => c,
        })
        .collect();
    let upper = normalized.to_ascii_uppercase();
    match upper.as_str() {
        "CON" | "PRN" | "AUX" | "NUL" => true,
        "COM1" | "COM2" | "COM3" | "COM4" | "COM5" | "COM6" | "COM7" | "COM8" | "COM9"
        | "LPT1" | "LPT2" | "LPT3" | "LPT4" | "LPT5" | "LPT6" | "LPT7" | "LPT8" | "LPT9" => {
            true
        }
        _ => false,
    }
}

fn sanitize_name(name: &str) -> String {
    let mut name: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if is_reserved_name(&name) {
        name.push('_');
    }
    name
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

#[cfg(test)]
mod tests {
    use super::{is_reserved_name, same_profile_file, sanitize_name};

    #[test]
    fn reserved_names_get_suffix() {
        for reserved in ["NUL", "con", "Com3", "lpt9", "COM¹", "lpt¹"] {
            let sanitized = sanitize_name(reserved);
            assert!(
                sanitized.ends_with('_') && !is_reserved_name(&sanitized),
                "reserved name survived: {reserved:?} -> {sanitized:?}"
            );
        }
    }

    #[test]
    fn plain_names_untouched() {
        for name in ["Normal name", "Μy-note_2", "COM10", "COM¹²", "nul_", "a/b", "yes.exe"] {
            assert_eq!(sanitize_name(name), {
                name.chars()
                    .map(|c| {
                        if c.is_alphanumeric() || c == '-' || c == '_' {
                            c
                        } else {
                            '_'
                        }
                    })
                    .collect::<String>()
            });
        }
    }

    #[test]
    fn sanitize_collision_matrix() {
        assert!(same_profile_file("a/b", "a:b"));
        // Display name vs the sanitized stem it was listed under — both
        // map to the same file, so a save must never delete it.
        assert!(same_profile_file("My Profile", "My_Profile"));
        assert!(same_profile_file("Nice profile!", "Nice_profile_"));
        assert!(!same_profile_file("one", "two"));
    }
}
