//! HostSmith keeps its plain-text configuration in `$XDG_CONFIG_HOME/hostsmith`,
//! falling back to `~/.config/hostsmith`, so a developer can version it in their
//! dotfiles. `dirs::config_dir()` is not used: on macOS it points at
//! Application Support.

use std::fs;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use serde::Serialize;

fn config_base() -> PathBuf {
    match std::env::var("XDG_CONFIG_HOME") {
        Ok(xdg) if !xdg.is_empty() => PathBuf::from(xdg),
        _ => home_dir().join(".config"),
    }
}

pub fn config_dir() -> PathBuf {
    config_base().join("hostsmith")
}

/// Files the app kept under its earlier name, before 2.0 was renamed.
const CARRIED_OVER: [&str; 2] = ["profiles.json", "settings.json"];

/// Copies the configuration of the app's earlier name, `hosven`, the first
/// time the new directory is missing. The old directory is left as it was,
/// so going back to an older build loses nothing.
pub fn carry_over_old_config() {
    carry_over(&config_base().join("hosven"), &config_dir());
}

fn carry_over(old: &Path, new: &Path) {
    if new.exists() || !old.is_dir() {
        return;
    }
    if let Err(error) = fs::create_dir_all(new) {
        eprintln!("hostsmith: could not create {}: {error}", new.display());
        return;
    }
    for name in CARRIED_OVER {
        let from = old.join(name);
        if from.is_file()
            && let Err(error) = fs::read(&from).and_then(|bytes| {
                write_atomic(&new.join(name), &bytes).map_err(std::io::Error::other)
            })
        {
            eprintln!("hostsmith: could not carry over {name}: {error}");
        }
    }
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

/// Each call gets its own temporary name, created fresh, so two saves of the
/// same file at once never write into one temporary file, and a name planted
/// in the directory beforehand is never followed or reused. The file is
/// readable by its owner only: nothing else on the machine has a reason to
/// read the user's setup.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let dir = path.parent().ok_or("Đường dẫn không hợp lệ")?;
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let name = path.file_name().ok_or("Đường dẫn không hợp lệ")?;
    let tmp = dir.join(format!(
        ".{}.{}.tmp",
        name.to_string_lossy(),
        uuid::Uuid::new_v4()
    ));
    let result = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&tmp)
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
        let dir = std::env::temp_dir().join(format!("hostsmith-config-{}", uuid::Uuid::new_v4()));
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
    fn concurrent_saves_of_one_file_do_not_share_a_temporary_file() {
        let dir = temp_dir();
        let path = dir.join("profiles.json");
        let writers: Vec<_> = (0..8)
            .map(|n| {
                let path = path.clone();
                std::thread::spawn(move || {
                    let text = format!("{n}").repeat(64 * 1024);
                    write_atomic(&path, text.as_bytes())
                })
            })
            .collect();
        for writer in writers {
            writer.join().unwrap().unwrap();
        }
        // Whichever save landed last, the file is one save, whole.
        let text = fs::read_to_string(&path).unwrap();
        assert_eq!(text.len(), 64 * 1024);
        assert!(text.chars().all(|c| text.starts_with(c)));
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn saved_files_are_private_to_their_owner() {
        let dir = temp_dir();
        let path = dir.join("settings.json");
        write_json_atomic(&path, &json!({ "secret": "x" })).unwrap();
        let mode =
            std::os::unix::fs::PermissionsExt::mode(&fs::metadata(&path).unwrap().permissions());
        assert_eq!(mode & 0o777, 0o600);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn carries_the_old_config_over_once_and_leaves_it_in_place() {
        let dir = temp_dir();
        let old = dir.join("hosven");
        let new = dir.join("hostsmith");
        fs::create_dir_all(&old).unwrap();
        fs::write(old.join("profiles.json"), "[]").unwrap();
        fs::write(old.join("env.json"), "{}").unwrap();

        carry_over(&old, &new);
        assert_eq!(fs::read_to_string(new.join("profiles.json")).unwrap(), "[]");
        assert!(
            !new.join("env.json").exists(),
            "the dropped .env data stays behind"
        );
        assert!(old.join("profiles.json").exists());

        // Once the new directory exists, later edits to the old one are not copied.
        fs::write(old.join("profiles.json"), "[1]").unwrap();
        carry_over(&old, &new);
        assert_eq!(fs::read_to_string(new.join("profiles.json")).unwrap(), "[]");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn replaces_an_existing_file() {
        let dir = temp_dir();
        let path = dir.join("settings.json");
        write_json_atomic(&path, &json!({ "version": 1 })).unwrap();
        write_json_atomic(&path, &json!({ "version": 2 })).unwrap();
        assert!(fs::read_to_string(&path).unwrap().contains('2'));
        fs::remove_dir_all(dir).unwrap();
    }
}
