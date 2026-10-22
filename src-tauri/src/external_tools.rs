//! Hands a folder or file to the tools a developer already has installed.
//! Detection is by bundle id, first installed wins, so labels can name the
//! app. It runs once per launch: installing an app while HostSmith runs shows up
//! after a restart.

use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use serde::Serialize;

use crate::config;
use crate::util::blocking;

const TERMINALS: [(&str, &str); 4] = [
    ("iTerm", "com.googlecode.iterm2"),
    ("Ghostty", "com.mitchellh.ghostty"),
    ("Warp", "dev.warp.Warp-Stable"),
    ("Terminal", "com.apple.Terminal"),
];
const EDITORS: [(&str, &str); 4] = [
    ("VS Code", "com.microsoft.VSCode"),
    ("Cursor", "com.todesktop.230313mzl4w4u92"),
    ("Zed", "dev.zed.Zed"),
    ("VSCodium", "com.vscodium"),
];
const CHROME: &str = "com.google.Chrome";
/// Chrome only accepts its internal pages through `open -b`.
const CHROME_DNS_PAGE: &str = "chrome://net-internals/#dns";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct App {
    pub name: String,
    pub bundle_id: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tools {
    pub terminal: Option<App>,
    pub editor: Option<App>,
    pub chrome: bool,
}

static DETECTED: OnceLock<Tools> = OnceLock::new();

/// The installed tools, looked up on first use. Blocks while it looks.
pub fn detected() -> &'static Tools {
    DETECTED.get_or_init(detect)
}

fn detect() -> Tools {
    let home = config::home_dir();
    let dirs = [
        PathBuf::from("/Applications"),
        PathBuf::from("/Applications/Utilities"),
        home.join("Applications"),
        PathBuf::from("/System/Applications"),
        PathBuf::from("/System/Applications/Utilities"),
    ];
    let bundled = bundle_ids_in(&dirs);
    let installed = |id: &str| bundled.contains(id) || spotlight_finds(id);
    Tools {
        terminal: first_installed(&TERMINALS, installed),
        editor: first_installed(&EDITORS, installed),
        chrome: installed(CHROME),
    }
}

fn first_installed(candidates: &[(&str, &str)], installed: impl Fn(&str) -> bool) -> Option<App> {
    candidates
        .iter()
        .find(|(_, id)| installed(id))
        .map(|(name, id)| App {
            name: name.to_string(),
            bundle_id: id.to_string(),
        })
}

/// The bundle ids of the apps directly inside `dirs`, read from each app's
/// `Info.plist`. This needs no Spotlight index, which users may turn off.
fn bundle_ids_in(dirs: &[PathBuf]) -> HashSet<String> {
    dirs.iter()
        .filter_map(|dir| std::fs::read_dir(dir).ok())
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension() == Some(OsStr::new("app")))
        .filter_map(|app| bundle_id(&app))
        .collect()
}

fn bundle_id(app: &Path) -> Option<String> {
    let info = plist::Value::from_file(app.join("Contents/Info.plist")).ok()?;
    let id = info
        .as_dictionary()?
        .get("CFBundleIdentifier")?
        .as_string()?;
    Some(id.to_string())
}

/// Finds apps installed anywhere else, such as in a subfolder of
/// /Applications. `id` is one of the constants above, never user text.
fn spotlight_finds(id: &str) -> bool {
    Command::new("/usr/bin/mdfind")
        .arg(format!("kMDItemCFBundleIdentifier == '{id}'"))
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .is_ok_and(|out| {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .any(|l| !l.trim().is_empty())
        })
}

/// An absolute path that exists. Absolute also means it cannot be read as an
/// option by `open`.
fn existing(path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);
    if !path.is_absolute() {
        return Err(format!("Invalid path: {}", path.display()));
    }
    if !path.exists() {
        return Err(format!("Could not find {}", path.display()));
    }
    Ok(path)
}

/// What a tool may be handed. A terminal gets folders only: given a file it
/// may run it, as Terminal does with `.command` and `.sh` files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Directory,
    FileOrDirectory,
}

/// `path` as `existing` checks it, and of the kind `target` allows. The
/// checks follow symlinks, so they apply to what the tool would open.
fn openable(path: &str, target: Target) -> Result<PathBuf, String> {
    let path = existing(path)?;
    let metadata =
        std::fs::metadata(&path).map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    match target {
        Target::Directory if !metadata.is_dir() => {
            Err(format!("{} is not a folder", path.display()))
        }
        Target::FileOrDirectory if !metadata.is_dir() && !metadata.is_file() => {
            Err(format!("{} is not a file or folder", path.display()))
        }
        _ => Ok(path),
    }
}

fn open(args: &[&OsStr]) -> Result<(), String> {
    let out = Command::new("/usr/bin/open")
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    Err(if stderr.is_empty() {
        format!("open exited with code {}", out.status.code().unwrap_or(-1))
    } else {
        stderr
    })
}

fn open_with(app: Option<&App>, missing: &str, path: &str, target: Target) -> Result<(), String> {
    let app = app.ok_or(missing)?;
    let path = openable(path, target)?;
    open(&[
        OsStr::new("-b"),
        OsStr::new(&app.bundle_id),
        path.as_os_str(),
    ])
    .map_err(|e| format!("Could not open {} with {}: {e}", path.display(), app.name))
}

#[tauri::command]
pub async fn tools_detect() -> Result<Tools, String> {
    blocking(|| Ok(detected().clone())).await
}

#[tauri::command]
pub async fn tools_open_in_terminal(path: String) -> Result<(), String> {
    blocking(move || {
        open_with(
            detected().terminal.as_ref(),
            "No terminal app found",
            &path,
            Target::Directory,
        )
    })
    .await
}

#[tauri::command]
pub async fn tools_open_in_editor(path: String) -> Result<(), String> {
    blocking(move || {
        open_with(
            detected().editor.as_ref(),
            "No code editor found",
            &path,
            Target::FileOrDirectory,
        )
    })
    .await
}

#[tauri::command]
pub async fn tools_reveal_in_finder(path: String) -> Result<(), String> {
    blocking(move || {
        let path = existing(&path)?;
        open(&[OsStr::new("-R"), path.as_os_str()])
            .map_err(|e| format!("Could not open Finder: {e}"))
    })
    .await
}

#[tauri::command]
pub async fn tools_open_chrome_dns() -> Result<(), String> {
    blocking(|| {
        if !detected().chrome {
            return Err("Google Chrome is not installed".into());
        }
        open(&[
            OsStr::new("-b"),
            OsStr::new(CHROME),
            OsStr::new(CHROME_DNS_PAGE),
        ])
        .map_err(|e| format!("Could not open Chrome: {e}"))
    })
    .await
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hostsmith-tools-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn fake_app(dir: &Path, name: &str, id: &str) {
        let contents = dir.join(name).join("Contents");
        fs::create_dir_all(&contents).unwrap();
        let mut info = plist::Dictionary::new();
        info.insert("CFBundleIdentifier".into(), id.into());
        plist::Value::Dictionary(info)
            .to_file_xml(contents.join("Info.plist"))
            .unwrap();
    }

    #[test]
    fn reads_bundle_ids_of_apps_in_the_folders() {
        let dir = temp_dir();
        fake_app(&dir, "Ghostty.app", "com.mitchellh.ghostty");
        fake_app(&dir, "Zed.app", "dev.zed.Zed");
        // Not an app bundle, and an app without an Info.plist.
        fake_app(&dir, "Notes", "not.an.app");
        fs::create_dir_all(dir.join("Broken.app")).unwrap();

        let ids = bundle_ids_in(&[dir.clone(), dir.join("missing")]);
        assert_eq!(
            ids,
            HashSet::from(["com.mitchellh.ghostty".into(), "dev.zed.Zed".into()])
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_first_installed_candidate_wins() {
        let installed = |id: &str| matches!(id, "com.apple.Terminal" | "dev.warp.Warp-Stable");
        assert_eq!(
            first_installed(&TERMINALS, installed),
            Some(App {
                name: "Warp".into(),
                bundle_id: "dev.warp.Warp-Stable".into(),
            })
        );
        assert_eq!(first_installed(&EDITORS, installed), None);
    }

    #[test]
    fn opens_only_absolute_paths_that_exist() {
        assert!(existing("relative/dir").is_err());
        assert!(existing("-a").is_err());
        assert!(existing("/definitely/not/here").is_err());
        assert_eq!(existing("/tmp"), Ok(PathBuf::from("/tmp")));
    }

    #[test]
    fn says_which_tool_is_missing() {
        assert_eq!(
            open_with(None, "No terminal app found", "/tmp", Target::Directory),
            Err("No terminal app found".into())
        );
    }

    #[test]
    fn a_terminal_gets_folders_and_an_editor_files_or_folders() {
        use std::os::unix::fs::PermissionsExt;

        let dir = temp_dir();
        let script = dir.join("run.command");
        fs::write(&script, "#!/bin/sh\necho hi\n").unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        let link = dir.join("link");
        std::os::unix::fs::symlink(&script, &link).unwrap();
        let (dir_text, script_text, link_text) = (
            dir.to_str().unwrap(),
            script.to_str().unwrap(),
            link.to_str().unwrap(),
        );

        assert_eq!(openable(dir_text, Target::Directory), Ok(dir.clone()));
        assert!(openable(script_text, Target::Directory).is_err());
        // A link to a script is judged by the script it points at.
        assert!(openable(link_text, Target::Directory).is_err());
        assert!(openable("/definitely/not/here", Target::Directory).is_err());

        assert_eq!(
            openable(script_text, Target::FileOrDirectory),
            Ok(script.clone())
        );
        assert_eq!(openable(dir_text, Target::FileOrDirectory), Ok(dir.clone()));
        // A device is neither a file nor a folder.
        assert!(openable("/dev/null", Target::FileOrDirectory).is_err());
        fs::remove_dir_all(dir).unwrap();
    }
}
