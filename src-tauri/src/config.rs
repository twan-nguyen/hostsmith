//! Hosven keeps its plain-text configuration in `$XDG_CONFIG_HOME/hosven`,
//! falling back to `~/.config/hosven`, so a developer can version it in their
//! dotfiles. `dirs::config_dir()` is not used: on macOS it points at
//! Application Support.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;

pub fn config_dir() -> PathBuf {
    let base = match std::env::var("XDG_CONFIG_HOME") {
        Ok(xdg) if !xdg.is_empty() => PathBuf::from(xdg),
        _ => home_dir().join(".config"),
    };
    base.join("hosven")
}

/// The preferences Hosven 1.x kept in UserDefaults, read once for migration.
pub fn legacy_defaults_path() -> PathBuf {
    home_dir().join("Library/Preferences/com.twannguyen.hosven.plist")
}

/// One value from the legacy preferences file, if the file and key exist.
pub fn legacy_default(plist: &Path, key: &str) -> Option<plist::Value> {
    let value = plist::Value::from_file(plist).ok()?;
    value.as_dictionary()?.get(key).cloned()
}

/// Moves an unreadable file aside so the next save cannot overwrite what the
/// user had, and returns the name it now has.
pub fn set_aside(path: &Path) -> Option<PathBuf> {
    let stamp = jiff::Timestamp::now()
        .strftime("%Y%m%dT%H%M%SZ")
        .to_string();
    let name = format!("{}.corrupt-{stamp}", path.file_name()?.to_string_lossy());
    let target = path.with_file_name(name);
    fs::rename(path, &target).ok()?;
    Some(target)
}

pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
}

/// Writes `value` as pretty JSON with sorted keys, through a temporary file in
/// the same directory so a crash never leaves a half-written file behind.
pub fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    // Going through `Value` sorts object keys, as the Swift encoder did.
    let value = serde_json::to_value(value).map_err(|e| e.to_string())?;
    let mut text = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    text.push('\n');
    write_atomic(path, text.as_bytes())
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let dir = path.parent().ok_or("Đường dẫn không hợp lệ")?;
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let name = path.file_name().ok_or("Đường dẫn không hợp lệ")?;
    let tmp = dir.join(format!(
        ".{}.{}.tmp",
        name.to_string_lossy(),
        std::process::id()
    ));
    let result = fs::File::create(&tmp)
        .and_then(|mut file| {
            file.write_all(bytes)?;
            file.sync_all()
        })
        .and_then(|()| fs::rename(&tmp, path));
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hosven-config-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn writes_sorted_pretty_json_and_creates_the_directory() {
        let dir = temp_dir();
        let path = dir.join("nested").join("profiles.json");
        write_json_atomic(&path, &json!({ "name": "Local", "color": "blue" })).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        assert!(text.find("\"color\"").unwrap() < text.find("\"name\"").unwrap());
        assert!(text.ends_with("}\n"));
        let leftovers: Vec<_> = fs::read_dir(path.parent().unwrap()).unwrap().collect();
        assert_eq!(leftovers.len(), 1, "the temporary file is renamed away");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn replaces_an_existing_file() {
        let dir = temp_dir();
        let path = dir.join("env.json");
        write_json_atomic(&path, &json!({ "version": 1 })).unwrap();
        write_json_atomic(&path, &json!({ "version": 2 })).unwrap();
        assert!(fs::read_to_string(&path).unwrap().contains('2'));
        fs::remove_dir_all(dir).unwrap();
    }
}
