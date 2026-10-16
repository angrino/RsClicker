//! Crash-safe file writing: a `.tmp` sibling is written and flushed, then
//! atomically renamed over the target — a crash mid-write can never leave
//! a truncated target file behind.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

fn tmp_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".tmp");
    PathBuf::from(name)
}

/// Writes `data` to `path` via a `.tmp` sibling plus `sync_all` and an
/// atomic rename. Leftover `.tmp` files (from a crash between write and
/// rename) are overwritten by the next save and are invisible to the
/// profile listing, which only matches the `json` extension.
pub fn write_atomic(path: &Path, data: &str) -> Result<(), String> {
    let tmp = tmp_path(path);
    {
        // The scope closes the file before rename — required on Windows.
        let mut file = fs::File::create(&tmp)
            .map_err(|e| format!("could not create {}: {e}", tmp.display()))?;
        file.write_all(data.as_bytes())
            .map_err(|e| format!("could not write {}: {e}", tmp.display()))?;
        file.sync_all()
            .map_err(|e| format!("could not flush {}: {e}", tmp.display()))?;
    }
    fs::rename(&tmp, path)
        .map_err(|e| format!("could not replace {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::write_atomic;
    use std::fs;

    fn test_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("rsclicker-atomic-file-tests");
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn replaces_existing_content_and_removes_tmp() {
        let path = test_dir().join("replace.json");
        write_atomic(&path, "{\"v\":1}").unwrap();
        write_atomic(&path, "{\"v\":2}").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"v\":2}");
        assert!(!test_dir().join("replace.json.tmp").exists());
    }

    #[test]
    fn stale_tmp_from_a_crash_is_overwritten() {
        let path = test_dir().join("stale.json");
        fs::write(test_dir().join("stale.json.tmp"), "garbage").unwrap();
        write_atomic(&path, "{\"ok\":true}").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"ok\":true}");
        assert!(!test_dir().join("stale.json.tmp").exists());
    }
}
