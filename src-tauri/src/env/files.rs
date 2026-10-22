//! Finding, reading and writing a repo's dotenv files, and keeping the
//! selected file in line with the disk. A file another tool changed after
//! Hosven read it is never overwritten without the user's say.

use std::fs;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use serde::Serialize;
use uuid::Uuid;

use super::state::{Env, NO_FILE, ToastKind};

/// The files Hosven looks for at a repo's root: the file itself, its local
/// override, and the example the others are compared against.
pub const ENV_FILES: [&str; 3] = [".env", ".env.local", ".env.example"];
pub const EXAMPLE_FILE: &str = ".env.example";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Discovery {
    Found(Vec<String>),
    RepoMissing,
    EnvMissing,
}

impl Discovery {
    pub fn files(&self) -> &[String] {
        match self {
            Self::Found(files) => files,
            _ => &[],
        }
    }
}

pub fn discover(repo: &Path) -> Discovery {
    if !repo.is_dir() {
        return Discovery::RepoMissing;
    }
    let found: Vec<String> = ENV_FILES
        .iter()
        .filter(|name| repo.join(name).is_file())
        .map(|name| name.to_string())
        .collect();
    if found.is_empty() {
        Discovery::EnvMissing
    } else {
        Discovery::Found(found)
    }
}

/// A path inside the repo: neither absolute nor climbing out of it.
pub fn validate_relative_path(path: &str) -> Result<(), String> {
    if path.is_empty() || path.contains("..") || path.starts_with('/') || path.contains('\0') {
        Err("Đường dẫn không hợp lệ".into())
    } else {
        Ok(())
    }
}

pub fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("Lỗi đọc file: {e}"))
}

/// How the file on disk differs from what Hosven last read or wrote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Conflict {
    /// Another tool changed the file.
    Changed,
    /// The file was deleted, or renamed away without a replacement.
    Missing,
}

/// `None` when the disk still holds `known`. A file that exists but cannot be
/// read counts as changed: it is not what Hosven read.
pub fn disk_state(path: &Path, known: &str) -> Option<Conflict> {
    match fs::read(path) {
        Ok(bytes) if bytes == known.as_bytes() => None,
        Ok(_) => Some(Conflict::Changed),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Some(Conflict::Missing),
        Err(_) => Some(Conflict::Changed),
    }
}

/// Replaces the file through a temporary file in the same directory, so a
/// crash never leaves half a file, keeping the old file's permissions. A
/// symlink is written through rather than replaced by a plain file.
pub fn write_content(path: &Path, content: &str) -> Result<(), String> {
    let target = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let invalid = || "Đường dẫn không hợp lệ".to_string();
    let dir = target.parent().ok_or_else(invalid)?;
    let name = target.file_name().ok_or_else(invalid)?;
    let tmp = dir.join(format!(
        ".{}.hosven-{}.tmp",
        name.to_string_lossy(),
        Uuid::new_v4().simple()
    ));
    let permissions = fs::metadata(&target).ok().map(|m| m.permissions());
    // Private until it takes the original file's permissions, so a secret
    // is never readable by others, even for a moment.
    let result = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&tmp)
        .and_then(|mut file| {
            file.write_all(content.as_bytes())?;
            file.sync_all()
        })
        .and_then(|()| match permissions {
            Some(permissions) => fs::set_permissions(&tmp, permissions),
            None => Ok(()),
        })
        .and_then(|()| fs::rename(&tmp, &target));
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result.map_err(|e| format!("Lỗi ghi file: {e}"))
}

/// How an apply ended. A refused write leaves the reason in the snapshot's
/// `conflict`, for the pane's banner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Outcome {
    Written,
    NothingToDo,
    /// The file changed on disk since Hosven read it; nothing was written.
    Conflict,
}

impl Env {
    /// Writes the selected file's entries. Unless `force`, refuses when the
    /// file changed on disk since Hosven read it. `force` also writes a clean
    /// file, which is how a deleted file is created again.
    pub fn apply(&mut self, force: bool) -> Result<Outcome, String> {
        let (key, path) = self.selected_location()?;
        let file = self.loaded.get_mut(&key).ok_or(NO_FILE)?;
        file.merge_raw_draft();
        if !force && !file.dirty() {
            return Ok(self.nothing_to_apply());
        }
        let content = file.text();
        self.write_selected(&path, content, force, false)
    }

    /// Writes the raw editor's text as it is, then reads the entries from it.
    pub fn apply_raw(&mut self, text: &str, force: bool) -> Result<Outcome, String> {
        let (key, path) = self.selected_location()?;
        let file = self.loaded.get(&key).ok_or(NO_FILE)?;
        let mut content = text.to_string();
        if !content.ends_with('\n') {
            content.push('\n');
        }
        if !force && content == file.disk_content && !file.dirty() {
            return Ok(self.nothing_to_apply());
        }
        self.write_selected(&path, content, force, true)
    }

    fn nothing_to_apply(&mut self) -> Outcome {
        self.show_toast("Không có gì để áp dụng".into(), ToastKind::Info);
        Outcome::NothingToDo
    }

    fn write_selected(
        &mut self,
        path: &Path,
        content: String,
        force: bool,
        reparse: bool,
    ) -> Result<Outcome, String> {
        let file = self.selected_mut().ok_or(NO_FILE)?;
        if !force && let Some(conflict) = disk_state(path, &file.disk_content) {
            file.conflict = Some(conflict);
            return Ok(Outcome::Conflict);
        }
        write_content(path, &content)?;
        if reparse {
            file.reparse_keeping_ids(&content);
        }
        file.mark_written(content);
        let message = format!(
            "Đã ghi {} biến vào {}",
            file.variable_count(),
            file.relative_path
        );
        self.after_disk_change();
        self.show_toast(message, ToastKind::Success);
        Ok(Outcome::Written)
    }

    /// A write may have created a file or changed the example file.
    fn after_disk_change(&mut self) {
        if let Some(repo) = self.selected_repo {
            self.rediscover(repo);
        }
        self.refresh_example();
    }

    /// Replaces the selected file with what is on disk, dropping pending edits.
    pub fn reload_from_disk(&mut self) -> Result<(), String> {
        let (key, path) = self.selected_location()?;
        let content = read(&path)?;
        match self.loaded.get_mut(&key) {
            Some(file) => file.reload(&content),
            None => {
                self.loaded.insert(
                    key.clone(),
                    super::state::EnvFile::from_disk(&key.1, &content),
                );
            }
        }
        self.load_error = None;
        self.after_disk_change();
        Ok(())
    }

    /// Brings the selected file in line with the disk: a clean file reloads
    /// in place, one with pending edits is marked as a conflict, and a file
    /// that is gone is marked missing. Also finds the repo's files again,
    /// since the user may have created or deleted one.
    pub fn refresh_from_disk(&mut self) {
        self.after_disk_change();
        let Ok((key, path)) = self.selected_location() else {
            return;
        };
        let Some(file) = self.loaded.get_mut(&key) else {
            // It could not be read before; try again.
            if let Ok(content) = read(&path) {
                self.loaded.insert(
                    key.clone(),
                    super::state::EnvFile::from_disk(&key.1, &content),
                );
                self.load_error = None;
            }
            return;
        };
        match disk_state(&path, &file.disk_content) {
            None => file.conflict = None,
            Some(Conflict::Missing) => file.conflict = Some(Conflict::Missing),
            Some(Conflict::Changed) if file.dirty() => file.conflict = Some(Conflict::Changed),
            Some(Conflict::Changed) => match read(&path) {
                Ok(content) => {
                    file.reload(&content);
                    let message = format!(
                        "Đã tải lại {} vì file vừa đổi bên ngoài",
                        file.relative_path
                    );
                    self.show_toast(message, ToastKind::Info);
                }
                Err(error) => {
                    file.conflict = Some(Conflict::Changed);
                    self.show_toast(error, ToastKind::Error);
                }
            },
        }
    }

    /// The watcher's entry point: refreshes only if the file it watches is
    /// still the selected one. Returns whether anything was looked at.
    pub fn refresh_watched(&mut self, repo_id: Uuid, relative_path: &str) -> bool {
        if self.selected_repo != Some(repo_id)
            || self.selected_file.as_deref() != Some(relative_path)
        {
            return false;
        }
        self.refresh_from_disk();
        true
    }

    /// Hides the banner without touching the file, for "Đóng".
    pub fn dismiss_conflict(&mut self) {
        if let Some(file) = self.selected_mut() {
            file.conflict = None;
        }
    }

    /// The selected file and its path on disk, for the watcher.
    pub fn watch_target(&self) -> Option<(Uuid, String, PathBuf)> {
        let (key, path) = self.selected_location().ok()?;
        Some((key.0, key.1, path))
    }
}

#[cfg(test)]
mod tests {
    use super::super::state::fixture::Fixture;
    use super::*;

    #[test]
    fn validates_relative_paths() {
        assert!(validate_relative_path(".env").is_ok());
        assert!(validate_relative_path("config/.env").is_ok());
        for bad in ["", "/etc/hosts", "../.env", "a/../../b", "a\0b"] {
            assert!(validate_relative_path(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn writing_keeps_the_old_permissions_and_leaves_no_temp_file() {
        use std::os::unix::fs::PermissionsExt;
        let f = Fixture::standard();
        let path = f.path(".env");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        write_content(&path, "A=1\n").unwrap();
        assert_eq!(f.disk(".env"), "A=1\n");
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let names: Vec<_> = fs::read_dir(&f.dir).unwrap().collect();
        assert_eq!(names.len(), 1);

        write_content(&f.path(".env.local"), "B=2\n").unwrap();
        assert_eq!(f.disk(".env.local"), "B=2\n");
    }

    #[test]
    fn writing_through_a_symlink_keeps_the_link() {
        let f = Fixture::new(&[("shared.env", "A=1\n")]);
        std::os::unix::fs::symlink(f.path("shared.env"), f.path(".env")).unwrap();
        write_content(&f.path(".env"), "A=2\n").unwrap();
        assert!(fs::symlink_metadata(f.path(".env")).unwrap().is_symlink());
        assert_eq!(f.disk("shared.env"), "A=2\n");
    }

    // MARK: - External changes

    #[test]
    fn writing_over_an_external_edit_is_refused_and_the_disk_kept() {
        let mut f = Fixture::standard();
        f.edit("mine");
        f.write(".env", "FOO=theirs\nBAR=2\n");

        assert_eq!(f.env.apply(false), Ok(Outcome::Conflict));
        assert_eq!(f.disk(".env"), "FOO=theirs\nBAR=2\n");
        assert_eq!(f.env.snapshot().conflict, Some(Conflict::Changed));
        assert!(f.file().dirty());
    }

    #[test]
    fn forcing_the_write_replaces_the_external_edit_and_clears_the_conflict() {
        let mut f = Fixture::standard();
        f.edit("mine");
        f.write(".env", "FOO=theirs\nBAR=2\n");
        f.env.apply(false).unwrap();

        assert_eq!(f.env.apply(true), Ok(Outcome::Written));
        assert!(f.disk(".env").contains("FOO=mine"));
        assert_eq!(f.env.snapshot().conflict, None);
        assert!(!f.file().dirty());
    }

    #[test]
    fn raw_writes_are_guarded_the_same_way() {
        let mut f = Fixture::standard();
        f.write(".env", "FOO=theirs\n");
        assert_eq!(f.env.apply_raw("FOO=mine\n", false), Ok(Outcome::Conflict));
        assert_eq!(f.disk(".env"), "FOO=theirs\n");

        assert_eq!(f.env.apply_raw("FOO=mine", true), Ok(Outcome::Written));
        assert_eq!(f.disk(".env"), "FOO=mine\n", "a final newline is added");
        assert_eq!(f.file().entries.len(), 1);
        assert!(!f.file().dirty());
        assert_eq!(f.toast(), "Đã ghi 1 biến vào .env");
    }

    #[test]
    fn a_raw_write_keeps_the_text_verbatim_and_ids_of_unchanged_lines() {
        let mut f = Fixture::standard();
        let bar = f.entry("BAR").id;
        let text = "FOO=2\n#  odd   spacing\nBAR=2\n";
        f.env.set_raw_draft(f.repo, ".env", Some(text.into()));
        assert!(f.env.snapshot().dirty);
        f.env.apply_raw(text, false).unwrap();
        assert_eq!(f.disk(".env"), text);
        assert_eq!(f.entry("BAR").id, bar);
        assert!(!f.env.snapshot().dirty);
    }

    #[test]
    fn hosvens_own_write_is_not_an_external_change() {
        let mut f = Fixture::standard();
        f.edit("mine");
        f.env.apply(false).unwrap();
        assert_eq!(disk_state(&f.path(".env"), &f.file().disk_content), None);
        f.env.refresh_from_disk();
        assert_eq!(f.env.snapshot().conflict, None);

        f.edit("again");
        assert_eq!(f.env.apply(false), Ok(Outcome::Written));
        assert!(f.disk(".env").contains("FOO=again"));
    }

    #[test]
    fn a_clean_file_reloads_and_a_dirty_one_becomes_a_conflict() {
        let mut f = Fixture::standard();
        let bar = f.entry("BAR").id;
        f.write(".env", "FOO=theirs\nBAR=2\n");
        f.env.refresh_from_disk();
        assert_eq!(f.entry("FOO").value, "theirs");
        assert_eq!(f.entry("BAR").id, bar, "an unchanged line keeps its id");
        assert!(!f.file().dirty());
        assert_eq!(f.toast(), "Đã tải lại .env vì file vừa đổi bên ngoài");

        f.edit("mine");
        f.write(".env", "FOO=later\nBAR=2\n");
        f.env.refresh_from_disk();
        assert_eq!(f.entry("FOO").value, "mine");
        assert_eq!(f.env.snapshot().conflict, Some(Conflict::Changed));

        f.env.reload_from_disk().unwrap();
        assert_eq!(f.entry("FOO").value, "later");
        assert_eq!(f.env.snapshot().conflict, None);
        assert!(!f.file().dirty());
    }

    #[test]
    fn a_deleted_file_is_reported_missing_and_can_be_created_again() {
        let mut f = Fixture::standard();
        fs::remove_file(f.path(".env")).unwrap();
        assert_eq!(
            disk_state(&f.path(".env"), &f.file().disk_content),
            Some(Conflict::Missing)
        );
        f.env.refresh_from_disk();
        assert_eq!(f.env.snapshot().conflict, Some(Conflict::Missing));
        assert_eq!(f.env.apply(false), Ok(Outcome::NothingToDo));

        f.env.apply(true).unwrap();
        assert_eq!(f.disk(".env"), "FOO=1\nBAR=2\n");
        assert_eq!(f.env.snapshot().conflict, None);
    }

    #[test]
    fn dismissing_hides_the_banner_only() {
        let mut f = Fixture::standard();
        fs::remove_file(f.path(".env")).unwrap();
        f.env.refresh_from_disk();
        f.env.dismiss_conflict();
        assert_eq!(f.env.snapshot().conflict, None);
        assert!(!f.path(".env").exists());
    }

    #[test]
    fn the_watcher_refreshes_only_the_selected_file() {
        let mut f = Fixture::new(&[(".env", "FOO=1\n"), (".env.local", "L=1\n")]);
        f.write(".env", "FOO=2\n");
        assert!(!f.env.refresh_watched(f.repo, ".env.local"));
        assert_eq!(f.entry("FOO").value, "1");
        assert!(f.env.refresh_watched(f.repo, ".env"));
        assert_eq!(f.entry("FOO").value, "2");
    }
}
