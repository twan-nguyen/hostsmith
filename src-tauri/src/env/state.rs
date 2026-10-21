//! The env model: the repo list, the selected dotenv file, its entries and
//! dirty state. Dotenv files belong to the user and need no privilege, so the
//! model reads and writes them itself; every rule still runs in a unit test
//! against a temporary directory.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::files::{self, Conflict, Discovery};
use super::parse::{self, EnvEntry};
use super::store::{self, EnvStore, Repo};

pub const NO_FILE: &str = "Chưa chọn file .env nào";
pub const REPO_NOT_FOUND: &str = "Không tìm thấy repo";
pub const INVALID_NAME: &str = "Tên không hợp lệ";
const ENTRY_NOT_FOUND: &str = "Không tìm thấy biến";

/// The fields of the variable form.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryInput {
    pub key: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub comment: String,
    /// `None` keeps the prefix the entry has; a new entry has none.
    #[serde(default)]
    pub exported: Option<bool>,
}

/// The key and comment as they will be written. The value may hold anything:
/// the writer quotes and escapes what would not read back.
fn validate(input: &EntryInput) -> Result<(String, String), String> {
    let key = input.key.trim();
    if !parse::is_valid_key(key) {
        return Err(
            "Khoá chỉ gồm chữ cái không dấu, chữ số và dấu _, không bắt đầu bằng chữ số".into(),
        );
    }
    let comment = input.comment.trim();
    if comment.contains(['\n', '\r']) {
        return Err("Ghi chú phải nằm trên một dòng".into());
    }
    Ok((key.to_string(), comment.to_string()))
}

/// A name for a repo or a saved profile: trimmed, non-empty, one line.
pub fn clean_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.contains(['\n', '\r']) {
        Err(INVALID_NAME.into())
    } else {
        Ok(trimmed.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ToastKind {
    Success,
    Error,
    Info,
}

/// A counted fact for the pane's banner slot. `id` grows with every toast, so
/// the UI can tell a new message from a snapshot that repeats an old one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Toast {
    pub id: u64,
    pub message: String,
    pub kind: ToastKind,
}

/// A dotenv file as Hosven holds it between reading and writing.
#[derive(Debug, Clone)]
pub struct EnvFile {
    pub relative_path: String,
    pub entries: Vec<EnvEntry>,
    /// The entries at the last load or write; dirty means they differ.
    pristine: Vec<EnvEntry>,
    /// What the raw editor holds but has not folded into `entries`.
    raw_draft: Option<String>,
    /// The exact text last read from or written to disk. The write guard and
    /// the watcher compare the disk against it.
    pub(super) disk_content: String,
    pub(super) conflict: Option<Conflict>,
}

impl EnvFile {
    pub fn from_disk(relative_path: &str, content: &str) -> Self {
        let entries = parse::parse(content);
        Self {
            relative_path: relative_path.to_string(),
            pristine: entries.clone(),
            entries,
            raw_draft: None,
            disk_content: content.to_string(),
            conflict: None,
        }
    }

    pub fn text(&self) -> String {
        parse::format(&self.entries)
    }

    fn draft_pending(&self) -> bool {
        self.raw_draft
            .as_ref()
            .is_some_and(|draft| *draft != self.text())
    }

    /// Compared line by line, not by id, so editing a value and typing the
    /// old one back leaves the file clean.
    pub fn dirty(&self) -> bool {
        self.draft_pending()
            || self.entries.len() != self.pristine.len()
            || self
                .entries
                .iter()
                .zip(&self.pristine)
                .any(|(a, b)| !a.same_line(b))
    }

    /// Lines that differ between the last clean state and what would be
    /// written now, counting repeated lines each time they occur.
    pub fn pending_count(&self) -> usize {
        let current = match &self.raw_draft {
            Some(draft) if self.draft_pending() => draft.clone(),
            _ => self.text(),
        };
        let pristine = parse::format(&self.pristine);
        let mut balance: HashMap<&str, isize> = HashMap::new();
        for line in pristine.split('\n') {
            *balance.entry(line).or_default() -= 1;
        }
        for line in current.split('\n') {
            *balance.entry(line).or_default() += 1;
        }
        let added: isize = balance.values().filter(|n| **n > 0).sum();
        let removed: isize = -balance.values().filter(|n| **n < 0).sum::<isize>();
        let diff = added.max(removed) as usize;
        if diff == 0 && self.dirty() { 1 } else { diff }
    }

    /// Variables, switched off ones included, as the write toast counts them.
    pub fn variable_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| !e.is_blank_or_comment)
            .count()
    }

    pub fn set_raw_draft(&mut self, text: Option<String>) {
        self.raw_draft = text;
    }

    /// Folds the raw editor's typing into the entries, so an edit made
    /// elsewhere lands on top of it instead of regenerating over it.
    pub fn merge_raw_draft(&mut self) {
        if let Some(draft) = self.raw_draft.take()
            && draft != self.text()
        {
            self.reparse_keeping_ids(&draft);
        }
    }

    /// Parses `content` into the entries, giving each line that did not
    /// change the id it had, so the UI's references to it stay valid.
    pub fn reparse_keeping_ids(&mut self, content: &str) {
        let mut old = std::mem::take(&mut self.entries);
        self.entries = parse::parse(content);
        for entry in &mut self.entries {
            if let Some(i) = old.iter().position(|o| o.same_line(entry)) {
                entry.id = old.remove(i).id;
            }
        }
    }

    /// Takes `content` as the file on disk, dropping every pending edit.
    pub fn reload(&mut self, content: &str) {
        self.reparse_keeping_ids(content);
        self.mark_written(content.to_string());
    }

    /// Records that the entries are now on disk as `content`.
    pub fn mark_written(&mut self, content: String) {
        self.pristine = self.entries.clone();
        self.disk_content = content;
        self.raw_draft = None;
        self.conflict = None;
    }

    pub fn discard(&mut self) {
        self.entries = self.pristine.clone();
        self.raw_draft = None;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DiscoveryState {
    Ok,
    /// The repo's folder is gone or is not a folder.
    RepoMissing,
    /// The folder holds none of the dotenv files Hosven looks for.
    EnvMissing,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileView {
    pub relative_path: String,
    pub dirty: bool,
    pub conflict: Option<Conflict>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileView {
    pub id: Uuid,
    pub name: String,
    pub captured_at: String,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoView {
    pub id: Uuid,
    pub name: String,
    pub path: String,
    pub missing: bool,
    pub discovery: DiscoveryState,
    pub files: Vec<FileView>,
    pub profiles: Vec<ProfileView>,
}

/// Everything the UI shows about env files, sent whole after each change.
/// The fields after `selected_file` describe the selected file.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub repos: Vec<RepoView>,
    pub selected_repo: Option<Uuid>,
    pub selected_file: Option<String>,
    pub entries: Vec<EnvEntry>,
    pub text: String,
    pub dirty: bool,
    pub pending_count: usize,
    pub conflict: Option<Conflict>,
    pub load_error: Option<String>,
    pub missing_keys: Vec<String>,
    pub toast: Option<Toast>,
}

pub type FileKey = (Uuid, String);

pub struct Env {
    store: EnvStore,
    pub(super) repos: Vec<Repo>,
    pub(super) selected_repo: Option<Uuid>,
    pub(super) selected_file: Option<String>,
    pub(super) discovered: HashMap<Uuid, Discovery>,
    pub(super) loaded: HashMap<FileKey, EnvFile>,
    /// The selected repo's example file as on disk, for missing variables.
    pub(super) example: Option<Vec<EnvEntry>>,
    /// Why the selected file could not be read.
    pub(super) load_error: Option<String>,
    toast: Option<Toast>,
    toast_seq: u64,
}

impl Env {
    pub fn new(mut store: EnvStore) -> Self {
        let loaded = store.load();
        let mut env = Self {
            store,
            selected_repo: loaded.repos.first().map(|r| r.id),
            repos: loaded.repos,
            selected_file: None,
            discovered: HashMap::new(),
            loaded: HashMap::new(),
            example: None,
            load_error: None,
            toast: None,
            toast_seq: 0,
        };
        for id in env.repos.iter().map(|r| r.id).collect::<Vec<_>>() {
            env.rediscover(id);
        }
        if let Some(notice) = loaded.notice {
            env.show_toast(notice, ToastKind::Error);
        }
        env
    }

    pub fn snapshot(&self) -> Snapshot {
        let selected = self.selected();
        Snapshot {
            repos: self.repos.iter().map(|r| self.repo_view(r)).collect(),
            selected_repo: self.selected_repo,
            selected_file: self.selected_file.clone(),
            entries: selected.map(|f| f.entries.clone()).unwrap_or_default(),
            text: selected.map(EnvFile::text).unwrap_or_default(),
            dirty: selected.is_some_and(EnvFile::dirty),
            pending_count: selected.map_or(0, EnvFile::pending_count),
            conflict: selected.and_then(|f| f.conflict),
            load_error: self.load_error.clone(),
            missing_keys: self.missing_keys(),
            toast: self.toast.clone(),
        }
    }

    fn repo_view(&self, repo: &Repo) -> RepoView {
        let discovery = self.discovered.get(&repo.id);
        let state = match discovery {
            Some(Discovery::RepoMissing) => DiscoveryState::RepoMissing,
            Some(Discovery::EnvMissing) => DiscoveryState::EnvMissing,
            _ => DiscoveryState::Ok,
        };
        let files = discovery
            .map(Discovery::files)
            .unwrap_or_default()
            .iter()
            .map(|path| {
                let file = self.loaded.get(&(repo.id, path.clone()));
                FileView {
                    relative_path: path.clone(),
                    dirty: file.is_some_and(EnvFile::dirty),
                    conflict: file.and_then(|f| f.conflict),
                }
            })
            .collect();
        RepoView {
            id: repo.id,
            name: repo.name.clone(),
            path: repo.path.clone(),
            missing: state == DiscoveryState::RepoMissing,
            discovery: state,
            files,
            profiles: repo
                .profiles
                .iter()
                .map(|p| ProfileView {
                    id: p.id,
                    name: p.name.clone(),
                    captured_at: store::iso8601(p.captured_at),
                    files: p.files.iter().map(|f| f.relative_path.clone()).collect(),
                })
                .collect(),
        }
    }

    // MARK: - Repos

    pub(super) fn repo(&self, id: Uuid) -> Result<&Repo, String> {
        self.repos
            .iter()
            .find(|r| r.id == id)
            .ok_or_else(|| REPO_NOT_FOUND.to_string())
    }

    pub(super) fn repo_index(&self, id: Uuid) -> Result<usize, String> {
        self.repos
            .iter()
            .position(|r| r.id == id)
            .ok_or_else(|| REPO_NOT_FOUND.to_string())
    }

    /// Adds the folder the user picked and selects it.
    pub fn add_repo(&mut self, path: &str) -> Result<(), String> {
        let path = repo_path(path)?;
        if self.repos.iter().any(|r| r.path == path) {
            return Err("Repo này đã có trong danh sách".into());
        }
        let name = Path::new(&path)
            .file_name()
            .map_or_else(|| path.clone(), |n| n.to_string_lossy().into_owned());
        let repo = Repo::new(&name, &path);
        let id = repo.id;
        self.repos.push(repo);
        let saved = self.persist();
        self.select(id, None)?;
        if saved {
            self.show_toast(format!("Đã thêm repo {name}"), ToastKind::Success);
        }
        Ok(())
    }

    /// Drops the repo and its saved profiles from the list. Its files stay.
    pub fn remove_repo(&mut self, id: Uuid) -> Result<(), String> {
        let index = self.repo_index(id)?;
        let repo = self.repos.remove(index);
        self.loaded.retain(|(repo_id, _), _| *repo_id != id);
        self.discovered.remove(&id);
        if self.selected_repo == Some(id) {
            self.selected_repo = self.repos.first().map(|r| r.id);
            self.selected_file = None;
            self.example = None;
            self.load_error = None;
        }
        if self.persist() {
            self.show_toast(
                format!("Đã xoá repo {} khỏi danh sách", repo.name),
                ToastKind::Info,
            );
        }
        Ok(())
    }

    pub fn rename_repo(&mut self, id: Uuid, name: &str) -> Result<(), String> {
        let name = clean_name(name)?;
        let index = self.repo_index(id)?;
        self.repos[index].name = name;
        self.persist();
        Ok(())
    }

    /// Saves the repo list. A failure is reported, not returned: the change
    /// stands for this session either way.
    pub(super) fn persist(&mut self) -> bool {
        match self.store.save(&self.repos) {
            Ok(()) => true,
            Err(error) => {
                self.show_toast(
                    format!("Không lưu được cấu hình: {error}"),
                    ToastKind::Error,
                );
                false
            }
        }
    }

    /// Each repo's id, name and folder, for telling which repo a process
    /// belongs to.
    #[allow(dead_code)] // Read by the ports scanner.
    pub fn known_repo_paths(&self) -> Vec<(Uuid, String, PathBuf)> {
        self.repos
            .iter()
            .map(|r| (r.id, r.name.clone(), PathBuf::from(&r.path)))
            .collect()
    }

    // MARK: - Selection

    /// Selects a repo and one of its files: `relative_path` when the repo has
    /// it, else the file already selected there, else the first one found.
    pub fn select(&mut self, repo_id: Uuid, relative_path: Option<String>) -> Result<(), String> {
        self.repo(repo_id)?;
        if let Some(path) = &relative_path {
            files::validate_relative_path(path)?;
        }
        // Typing in the raw editor stays with the file it was typed into.
        if let Some(file) = self.selected_mut() {
            file.merge_raw_draft();
        }
        let same_repo = self.selected_repo == Some(repo_id);
        self.selected_repo = Some(repo_id);
        self.rediscover(repo_id);
        let available = self.discovered[&repo_id].files().to_vec();
        let keep = same_repo
            && relative_path.is_none()
            && self
                .selected_file
                .as_ref()
                .is_some_and(|f| available.contains(f));
        self.selected_file = match relative_path {
            Some(path) if available.contains(&path) => Some(path),
            _ if keep => self.selected_file.take(),
            _ => available.first().cloned(),
        };
        self.open_selected();
        Ok(())
    }

    /// Finds the repo's files again, keeping the selection when it still exists.
    pub fn discover(&mut self, repo_id: Uuid) -> Result<(), String> {
        if self.selected_repo == Some(repo_id) {
            self.select(repo_id, None)
        } else {
            self.repo(repo_id)?;
            self.rediscover(repo_id);
            Ok(())
        }
    }

    pub(super) fn rediscover(&mut self, repo_id: Uuid) {
        if let Ok(repo) = self.repo(repo_id) {
            let found = files::discover(Path::new(&repo.path));
            self.discovered.insert(repo_id, found);
        }
    }

    /// Reads the selected file unless it is loaded already, in which case it
    /// is brought in line with the disk instead.
    pub(super) fn open_selected(&mut self) {
        self.load_error = None;
        let Some(key) = self.selected_key() else {
            self.refresh_example();
            return;
        };
        if self.loaded.contains_key(&key) {
            self.refresh_from_disk();
            return;
        }
        self.refresh_example();
        match self.file_path(&key).and_then(|path| files::read(&path)) {
            Ok(content) => {
                self.loaded
                    .insert(key.clone(), EnvFile::from_disk(&key.1, &content));
            }
            Err(error) => self.load_error = Some(error),
        }
    }

    pub(super) fn refresh_example(&mut self) {
        self.example = self.selected_repo.and_then(|id| {
            let repo = self.repo(id).ok()?;
            let content =
                std::fs::read_to_string(Path::new(&repo.path).join(files::EXAMPLE_FILE)).ok()?;
            Some(parse::parse(&content))
        });
    }

    pub(super) fn selected_key(&self) -> Option<FileKey> {
        Some((self.selected_repo?, self.selected_file.clone()?))
    }

    pub(super) fn file_path(&self, key: &FileKey) -> Result<PathBuf, String> {
        files::validate_relative_path(&key.1)?;
        Ok(Path::new(&self.repo(key.0)?.path).join(&key.1))
    }

    /// The selected file's key and where it lives on disk.
    pub(super) fn selected_location(&self) -> Result<(FileKey, PathBuf), String> {
        let key = self.selected_key().ok_or(NO_FILE)?;
        let path = self.file_path(&key)?;
        Ok((key, path))
    }

    pub(super) fn selected(&self) -> Option<&EnvFile> {
        self.loaded.get(&self.selected_key()?)
    }

    pub(super) fn selected_mut(&mut self) -> Option<&mut EnvFile> {
        let key = self.selected_key()?;
        self.loaded.get_mut(&key)
    }

    // MARK: - Entries

    /// The selected file, with typing from the raw editor folded in first so
    /// the edit goes on top of it.
    pub(super) fn editable(&mut self) -> Result<&mut EnvFile, String> {
        let file = self.selected_mut().ok_or(NO_FILE)?;
        file.merge_raw_draft();
        Ok(file)
    }

    pub fn add_entry(&mut self, input: EntryInput) -> Result<(), String> {
        let (key, comment) = validate(&input)?;
        let file = self.editable()?;
        let mut entry = EnvEntry::variable(&key, &input.value, &comment);
        entry.exported = input.exported.unwrap_or(false);
        file.entries.push(entry);
        Ok(())
    }

    pub fn update_entry(&mut self, id: Uuid, input: EntryInput) -> Result<(), String> {
        let (key, comment) = validate(&input)?;
        let file = self.editable()?;
        let entry = variable_mut(file, id)?;
        entry.key = key;
        entry.value = input.value;
        entry.comment = comment;
        if let Some(exported) = input.exported {
            entry.exported = exported;
        }
        Ok(())
    }

    pub fn duplicate_entry(&mut self, id: Uuid) -> Result<(), String> {
        let file = self.editable()?;
        let index = entry_index(file, id)?;
        let copy = file.entries[index].copy();
        file.entries.insert(index + 1, copy);
        Ok(())
    }

    pub fn toggle_entry(&mut self, id: Uuid) -> Result<(), String> {
        let entry = variable_mut(self.editable()?, id)?;
        entry.enabled = !entry.enabled;
        Ok(())
    }

    pub fn delete_entry(&mut self, id: Uuid) -> Result<(), String> {
        let file = self.editable()?;
        let index = entry_index(file, id)?;
        file.entries.remove(index);
        Ok(())
    }

    /// Picks one of the definitions of a key the file defines more than
    /// once: it is switched on and every other one off. Picking the one that
    /// is on switches the key off altogether.
    pub fn select_variant(&mut self, key: &str, id: Uuid) -> Result<(), String> {
        let file = self.editable()?;
        let chosen = variable_mut(file, id)?;
        if chosen.key != key {
            return Err(ENTRY_NOT_FOUND.into());
        }
        let turn_off = chosen.enabled;
        for entry in file
            .entries
            .iter_mut()
            .filter(|e| !e.is_blank_or_comment && e.key == key)
        {
            entry.enabled = !turn_off && entry.id == id;
        }
        Ok(())
    }

    /// Returns the selected file to its last loaded or written state.
    pub fn discard(&mut self) -> Result<(), String> {
        self.selected_mut().ok_or(NO_FILE)?.discard();
        Ok(())
    }

    /// Holds the raw editor's text, or `None` when the editor closes.
    pub fn set_raw_draft(&mut self, text: Option<String>) {
        if let Some(file) = self.selected_mut() {
            file.set_raw_draft(text);
        }
    }

    // MARK: - Toast

    pub fn show_toast(&mut self, message: String, kind: ToastKind) {
        self.toast_seq += 1;
        self.toast = Some(Toast {
            id: self.toast_seq,
            message,
            kind,
        });
    }

    #[cfg(test)]
    pub(super) fn toast(&self) -> Option<&Toast> {
        self.toast.as_ref()
    }

    #[cfg(test)]
    pub(super) fn store_path(&self) -> &Path {
        self.store.path()
    }
}

/// An absolute path to an existing folder, without a trailing slash.
fn repo_path(path: &str) -> Result<String, String> {
    let invalid = || "Đường dẫn không hợp lệ".to_string();
    if !path.starts_with('/') || path.contains(['\0', '\n', '\r']) {
        return Err(invalid());
    }
    let trimmed = path.trim_end_matches('/');
    let path = if trimmed.is_empty() { "/" } else { trimmed };
    if !Path::new(path).is_dir() {
        return Err(invalid());
    }
    Ok(path.to_string())
}

fn entry_index(file: &EnvFile, id: Uuid) -> Result<usize, String> {
    file.entries
        .iter()
        .position(|e| e.id == id)
        .ok_or_else(|| ENTRY_NOT_FOUND.to_string())
}

fn variable_mut(file: &mut EnvFile, id: Uuid) -> Result<&mut EnvEntry, String> {
    file.entries
        .iter_mut()
        .find(|e| e.id == id && !e.is_blank_or_comment)
        .ok_or_else(|| ENTRY_NOT_FOUND.to_string())
}

/// A repo in a temporary directory with the given files, and the model
/// holding it with `.env` selected.
#[cfg(test)]
pub(super) mod fixture {
    use super::*;
    use std::fs;

    pub struct Fixture {
        pub env: Env,
        pub repo: Uuid,
        pub dir: PathBuf,
        store_dir: PathBuf,
    }

    impl Fixture {
        pub fn new(files: &[(&str, &str)]) -> Self {
            let dir = std::env::temp_dir().join(format!("hosven-env-{}", Uuid::new_v4()));
            fs::create_dir_all(&dir).unwrap();
            for (name, content) in files {
                fs::write(dir.join(name), content).unwrap();
            }
            let (store, store_dir) = store::temp_store();
            let mut env = Env::new(store);
            env.add_repo(dir.to_str().unwrap()).unwrap();
            let repo = env.repos[0].id;
            Self {
                env,
                repo,
                dir,
                store_dir,
            }
        }

        /// The usual repo: a `.env` with `FOO=1` and `BAR=2`.
        pub fn standard() -> Self {
            Self::new(&[(".env", "FOO=1\nBAR=2\n")])
        }

        pub fn path(&self, name: &str) -> PathBuf {
            self.dir.join(name)
        }

        pub fn disk(&self, name: &str) -> String {
            fs::read_to_string(self.path(name)).unwrap()
        }

        pub fn write(&self, name: &str, content: &str) {
            fs::write(self.path(name), content).unwrap();
        }

        pub fn file(&self) -> &EnvFile {
            self.env.selected().expect("a file is selected")
        }

        pub fn entry(&self, key: &str) -> &EnvEntry {
            self.file()
                .entries
                .iter()
                .find(|e| e.key == key && !e.is_blank_or_comment)
                .expect("the key exists")
        }

        /// Sets `FOO`'s value through the model, as the variable form does.
        pub fn edit(&mut self, value: &str) {
            let id = self.entry("FOO").id;
            self.env
                .update_entry(
                    id,
                    EntryInput {
                        key: "FOO".into(),
                        value: value.into(),
                        ..Default::default()
                    },
                )
                .unwrap();
        }

        pub fn toast(&self) -> String {
            self.env
                .toast()
                .map(|t| t.message.clone())
                .unwrap_or_default()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
            let _ = fs::remove_dir_all(&self.store_dir);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixture::Fixture;
    use super::*;

    fn input(key: &str, value: &str) -> EntryInput {
        EntryInput {
            key: key.into(),
            value: value.into(),
            ..Default::default()
        }
    }

    // MARK: - Dirty state of one file

    #[test]
    fn a_fresh_file_is_clean() {
        let file = EnvFile::from_disk(".env", "FOO=1\n");
        assert!(!file.dirty());
        assert_eq!(file.pending_count(), 0);
    }

    #[test]
    fn changing_an_entry_makes_the_file_dirty() {
        let mut file = EnvFile::from_disk(".env", "FOO=1\n");
        file.entries[0].value = "2".into();
        assert!(file.dirty());
        assert_eq!(file.pending_count(), 1);
    }

    #[test]
    fn changing_it_back_makes_the_file_clean_again() {
        let mut file = EnvFile::from_disk(".env", "FOO=1\n");
        file.entries[0].value = "2".into();
        file.entries[0].value = "1".into();
        assert!(!file.dirty());
    }

    #[test]
    fn a_raw_draft_that_differs_makes_the_file_dirty() {
        let mut file = EnvFile::from_disk(".env", "");
        file.set_raw_draft(Some(file.text()));
        assert!(!file.dirty(), "text matching the entries is not an edit");
        file.set_raw_draft(Some("X=1\n".into()));
        assert!(file.dirty());
        assert_eq!(file.pending_count(), 1);
        file.discard();
        assert!(!file.dirty());
    }

    #[test]
    fn merging_a_draft_keeps_ids_of_unchanged_lines() {
        let mut file = EnvFile::from_disk(".env", "FOO=1\nBAR=2\n");
        let foo = file.entries[0].id;
        let bar = file.entries[1].id;
        file.set_raw_draft(Some("FOO=1\nBAR=3\nNEW=4\n".into()));
        file.merge_raw_draft();
        assert_eq!(file.entries.len(), 3);
        assert_eq!(file.entries[0].id, foo);
        assert_ne!(file.entries[1].id, bar, "a changed line is a new entry");
        assert!(file.dirty());
    }

    // MARK: - Edit and apply flow

    #[test]
    fn a_loaded_file_is_clean() {
        let f = Fixture::standard();
        assert_eq!(f.env.selected_file.as_deref(), Some(".env"));
        assert_eq!(f.file().entries.len(), 2);
        assert!(!f.file().dirty());
        assert_eq!(
            f.toast(),
            format!(
                "Đã thêm repo {}",
                f.dir.file_name().unwrap().to_string_lossy()
            )
        );
    }

    #[test]
    fn editing_an_entry_makes_the_file_dirty() {
        let mut f = Fixture::standard();
        f.edit("999");
        assert_eq!(f.entry("FOO").value, "999");
        assert!(f.file().dirty());
        assert!(f.env.snapshot().dirty);
    }

    #[test]
    fn applying_writes_the_file_and_clears_dirty() {
        let mut f = Fixture::standard();
        f.edit("999");
        assert_eq!(f.env.apply(false), Ok(files::Outcome::Written));
        assert!(!f.file().dirty());
        assert_eq!(f.disk(".env"), "FOO=999\nBAR=2\n");
        assert_eq!(f.toast(), "Đã ghi 2 biến vào .env");
    }

    #[test]
    fn add_toggle_and_delete_each_make_the_file_dirty() {
        let mut f = Fixture::standard();
        f.env.add_entry(input("BAZ", "3")).unwrap();
        assert!(f.file().dirty());
        f.env.apply(false).unwrap();
        assert_eq!(f.disk(".env"), "FOO=1\nBAR=2\nBAZ=3\n");

        let first = f.file().entries[0].id;
        f.env.toggle_entry(first).unwrap();
        assert!(f.file().dirty());
        f.env.apply(false).unwrap();
        assert_eq!(f.disk(".env"), "# FOO=1\nBAR=2\nBAZ=3\n");

        let last = f.file().entries[2].id;
        f.env.delete_entry(last).unwrap();
        assert!(f.file().dirty());
        f.env.discard().unwrap();
        assert!(!f.file().dirty());
        assert_eq!(f.file().entries.len(), 3);
    }

    #[test]
    fn duplicating_inserts_a_copy_below() {
        let mut f = Fixture::standard();
        let foo = f.entry("FOO").id;
        f.env.duplicate_entry(foo).unwrap();
        let entries = &f.file().entries;
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[1].key, "FOO");
        assert_ne!(entries[1].id, foo);
    }

    #[test]
    fn nothing_to_apply_when_clean() {
        let mut f = Fixture::standard();
        assert_eq!(f.env.apply(false), Ok(files::Outcome::NothingToDo));
        assert_eq!(f.toast(), "Không có gì để áp dụng");
    }

    #[test]
    fn rejects_keys_the_parser_would_not_read_back() {
        let mut f = Fixture::standard();
        for key in ["", "1A", "KÉY", "A B", "A\nB"] {
            assert!(f.env.add_entry(input(key, "x")).is_err(), "{key:?}");
        }
        let bad_comment = EntryInput {
            comment: "two\nlines".into(),
            ..input("OK", "x")
        };
        assert!(f.env.add_entry(bad_comment).is_err());
        assert!(!f.file().dirty());

        // A value may hold anything: the writer escapes it.
        f.env.add_entry(input(" OK ", "multi\nline")).unwrap();
        f.env.apply(false).unwrap();
        assert!(f.disk(".env").ends_with("OK=\"multi\\nline\"\n"));
    }

    #[test]
    fn an_exported_variable_keeps_its_prefix() {
        let mut f = Fixture::new(&[(".env", "export FOO=1\n")]);
        f.edit("2");
        f.env.apply(false).unwrap();
        assert_eq!(f.disk(".env"), "export FOO=2\n");
    }

    #[test]
    fn edits_fold_in_typing_from_the_raw_editor_first() {
        let mut f = Fixture::standard();
        f.env
            .set_raw_draft(Some("FOO=1\nBAR=2\nTYPED=yes\n".into()));
        f.env.add_entry(input("ADDED", "1")).unwrap();
        let keys: Vec<_> = f.file().entries.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(keys, ["FOO", "BAR", "TYPED", "ADDED"]);
    }

    // MARK: - Variants

    fn variants() -> Fixture {
        Fixture::new(&[(".env", "DB=local\n# DB=staging\n# DB=prod\nOTHER=1\n")])
    }

    fn db_states(f: &Fixture) -> Vec<bool> {
        f.file()
            .entries
            .iter()
            .filter(|e| e.key == "DB")
            .map(|e| e.enabled)
            .collect()
    }

    #[test]
    fn selecting_a_variant_switches_the_others_off() {
        let mut f = variants();
        let staging = f.file().entries[1].id;
        f.env.select_variant("DB", staging).unwrap();
        assert_eq!(db_states(&f), [false, true, false]);
        f.env.apply(false).unwrap();
        assert_eq!(
            f.disk(".env"),
            "# DB=local\nDB=staging\n# DB=prod\nOTHER=1\n"
        );
    }

    #[test]
    fn selecting_the_enabled_variant_switches_the_key_off() {
        let mut f = variants();
        let local = f.file().entries[0].id;
        f.env.select_variant("DB", local).unwrap();
        assert_eq!(db_states(&f), [false, false, false]);
        assert!(f.entry("OTHER").enabled, "other keys are left alone");
    }

    #[test]
    fn a_variant_must_belong_to_the_key() {
        let mut f = variants();
        let other = f.entry("OTHER").id;
        assert!(f.env.select_variant("DB", other).is_err());
        assert_eq!(db_states(&f), [true, false, false]);
    }

    // MARK: - Repos and selection

    #[test]
    fn a_repo_is_added_once() {
        let mut f = Fixture::standard();
        let path = f.dir.to_str().unwrap().to_string();
        assert_eq!(
            f.env.add_repo(&format!("{path}/")),
            Err("Repo này đã có trong danh sách".into())
        );
        assert!(f.env.add_repo("relative/path").is_err());
        assert!(f.env.add_repo("/no/such/folder/for/hosven").is_err());
        assert_eq!(f.env.repos.len(), 1);
    }

    #[test]
    fn repos_persist_and_reload() {
        let mut f = Fixture::standard();
        f.env.rename_repo(f.repo, "  API  ").unwrap();
        assert_eq!(f.env.rename_repo(f.repo, "  "), Err(INVALID_NAME.into()));
        let reread = Env::new(EnvStore::new(f.env.store_path().to_path_buf(), None));
        assert_eq!(reread.repos.len(), 1);
        assert_eq!(reread.repos[0].name, "API");
        assert_eq!(reread.selected_repo, Some(f.repo));
        assert_eq!(reread.snapshot().repos[0].files[0].relative_path, ".env");
    }

    #[test]
    fn removing_a_repo_leaves_its_files() {
        let mut f = Fixture::standard();
        f.env.remove_repo(f.repo).unwrap();
        assert!(f.env.repos.is_empty());
        assert!(f.env.loaded.is_empty());
        assert_eq!(f.env.selected_repo, None);
        assert!(f.path(".env").exists());
        assert!(f.toast().contains("khỏi danh sách"));
    }

    #[test]
    fn discovers_only_the_three_root_files() {
        let mut f = Fixture::new(&[
            (".env.example", "A=\n"),
            (".env.production", "A=1\n"),
            (".env.local", "A=2\n"),
        ]);
        std::fs::create_dir_all(f.path(".env")).unwrap();
        f.env.discover(f.repo).unwrap();
        let view = &f.env.snapshot().repos[0];
        let files: Vec<_> = view
            .files
            .iter()
            .map(|v| v.relative_path.as_str())
            .collect();
        assert_eq!(files, [".env.local", ".env.example"]);
        assert_eq!(f.env.selected_file.as_deref(), Some(".env.local"));
    }

    #[test]
    fn reports_a_missing_folder_and_a_folder_without_files() {
        let mut f = Fixture::new(&[]);
        assert_eq!(
            f.env.snapshot().repos[0].discovery,
            DiscoveryState::EnvMissing
        );
        assert_eq!(f.env.selected_file, None);
        std::fs::remove_dir_all(&f.dir).unwrap();
        f.env.discover(f.repo).unwrap();
        let view = &f.env.snapshot().repos[0];
        assert_eq!(view.discovery, DiscoveryState::RepoMissing);
        assert!(view.missing);
    }

    #[test]
    fn selecting_keeps_pending_edits_per_file() {
        let mut f = Fixture::new(&[(".env", "FOO=1\n"), (".env.local", "LOCAL=1\n")]);
        f.edit("2");
        f.env.select(f.repo, Some(".env.local".into())).unwrap();
        assert_eq!(f.file().relative_path, ".env.local");
        let view = f.env.snapshot();
        assert!(view.repos[0].files[0].dirty, ".env keeps its edit");
        assert!(!view.dirty);

        // A path the repo does not have falls back to the first file.
        f.env.select(f.repo, Some(".env.nope".into())).unwrap();
        assert_eq!(f.env.selected_file.as_deref(), Some(".env"));
        assert_eq!(f.entry("FOO").value, "2");
        assert!(f.env.select(f.repo, Some("../secret".into())).is_err());
        assert!(f.env.select(f.repo, Some("/etc/passwd".into())).is_err());
    }

    #[test]
    fn a_file_that_cannot_be_read_reports_why() {
        let mut f = Fixture::new(&[(".env", "FOO=1\n")]);
        std::fs::write(f.path(".env.local"), [0xFF, 0xFE, 0x00]).unwrap();
        f.env.select(f.repo, Some(".env.local".into())).unwrap();
        let snapshot = f.env.snapshot();
        assert!(snapshot.load_error.unwrap().starts_with("Lỗi đọc file"));
        assert!(snapshot.entries.is_empty());
        assert_eq!(f.env.apply(false), Err(NO_FILE.into()));
    }
}
