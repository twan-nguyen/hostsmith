//! The hosts model: entries, blocks, profile metadata, undo, dirty state and
//! the write flow. Pure logic; reading and writing the real file happens in
//! the command layer, so every rule here runs in a plain unit test.

use std::collections::{HashMap, VecDeque};

use serde::Serialize;
use uuid::Uuid;

use super::history::History;
use super::model::{HostEntry, Toast, ToastKind};
use super::parse::{self, Parsed};
use super::switch::{ActiveState, PendingSwitch};
use super::verify::{self, Verification};
use crate::platform::RunOutcome;
use crate::profiles::{self, Profile, ProfileColor, ProfileStore};
use crate::toast::Toasts;
use crate::util::changed_lines;

pub const BUSY: &str = "Hosven đang ghi /etc/hosts. Thử lại khi xong.";
pub const STALE_EDITOR: &str = "Nội dung hosts vừa đổi nên chưa ghi. Trình soạn thảo đã lấy nội dung mới, hãy xem lại rồi áp dụng.";

/// The fields of the entry form.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryInput {
    pub ip: String,
    pub hostname: String,
    /// `None` keeps the aliases the entry has.
    #[serde(default)]
    pub aliases: Option<Vec<String>>,
    #[serde(default)]
    pub comment: String,
    pub tag: Option<String>,
}

/// How an action that may write the file ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum Outcome {
    Written,
    /// The password prompt was dismissed; nothing was written.
    Cancelled,
    Failed(String),
    /// Other edits are pending and the window must ask before writing them too.
    NeedsConfirmation(usize),
    /// Other edits are pending and the caller cannot ask, so nothing was written.
    Refused(usize),
    NothingToDo,
}

/// A write the model has prepared. The caller runs it without holding the
/// model's lock, since the password prompt can stay open for a while, then
/// hands the result to [`Hosts::finish_write`]. Edits are refused until then.
#[derive(Debug)]
pub struct WriteJob {
    pub content: String,
    /// Hostnames to check with the resolver once the write lands.
    pub changed: Vec<(String, String)>,
    /// Matches the verification to this write, not a later one.
    pub seq: u64,
    reparse: bool,
    rollback: Option<Rollback>,
    success_toast: Option<String>,
}

#[derive(Debug)]
struct Rollback {
    entries: Vec<HostEntry>,
}

pub enum Step {
    Done(Outcome),
    Write(WriteJob),
}

/// Everything the UI shows about the hosts file, sent whole after each change.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub text: String,
    pub entries: Vec<HostEntry>,
    pub tags: Vec<String>,
    pub profiles: Vec<Profile>,
    pub active: ActiveState,
    pub dirty: bool,
    pub pending_count: usize,
    pub can_undo: bool,
    pub can_redo: bool,
    pub applying: bool,
    pub external_change: bool,
    pub load_error: Option<String>,
    pub verification: Option<Verification>,
    pub pending_switch: Option<PendingSwitch>,
    pub toast: Option<Toast>,
}

pub struct Hosts {
    pub(super) entries: Vec<HostEntry>,
    comment_lines: Vec<String>,
    tags: Vec<String>,
    pub(super) profiles: Vec<Profile>,
    store: Box<dyn ProfileStore>,
    pub(super) history: History,
    /// The text last read from or written to disk.
    original: String,
    /// The normalised text of the last clean state; dirty means the model
    /// serialises to something else.
    pristine: String,
    /// What the raw editor holds but has not folded into `entries`.
    raw_draft: Option<String>,
    pub(super) applying: bool,
    pub(super) pending_switch: Option<PendingSwitch>,
    external_change: bool,
    /// Why the file could not be read. Writing then would replace everything
    /// in it with only what the model holds, so writes wait for a reload.
    load_error: Option<String>,
    verification: Option<Verification>,
    write_seq: u64,
    toasts: Toasts,
}

impl Hosts {
    pub fn new(store: Box<dyn ProfileStore>) -> Self {
        let loaded = store.load();
        let mut hosts = Self {
            entries: Vec::new(),
            comment_lines: Vec::new(),
            tags: Vec::new(),
            profiles: loaded.profiles,
            store,
            history: History::default(),
            original: String::new(),
            pristine: "\n".to_string(),
            raw_draft: None,
            applying: false,
            pending_switch: None,
            external_change: false,
            load_error: None,
            verification: None,
            write_seq: 0,
            toasts: Toasts::default(),
        };
        if let Some(notice) = loaded.notice {
            hosts.show_toast(notice, ToastKind::Error);
        }
        hosts
    }

    pub fn snapshot(&self) -> Snapshot {
        // Serialised once: every derived field compares against the same text.
        let text = self.serialize();
        Snapshot {
            dirty: self.dirty_with(&text),
            pending_count: self.pending_count_with(&text),
            text,
            entries: self.entries.clone(),
            tags: self.tags.clone(),
            profiles: self.profiles.clone(),
            active: self.active_state(),
            can_undo: self.history.can_undo(),
            can_redo: self.history.can_redo(),
            applying: self.applying,
            external_change: self.external_change,
            load_error: self.load_error.clone(),
            verification: self.verification.clone(),
            pending_switch: self.pending_switch.clone(),
            toast: self.toasts.current().cloned(),
        }
    }

    pub fn profiles(&self) -> &[Profile] {
        &self.profiles
    }

    // MARK: - Loading

    /// Takes `content` as the file on disk, dropping every pending edit.
    pub fn load(&mut self, content: &str) {
        self.original = content.to_string();
        self.raw_draft = None;
        self.load_error = None;
        self.replace_from(content);
        self.capture_clean();
        self.external_change = false;
        self.history.clear();
    }

    pub fn load_failed(&mut self, error: &str) {
        self.load_error = Some(error.to_string());
        self.show_toast(
            format!("Không đọc được /etc/hosts: {error}"),
            ToastKind::Error,
        );
    }

    /// Reads the file again on the user's request. Not while a write is out,
    /// whose result would land on top of the reloaded entries.
    pub fn reload(&mut self, read: Result<String, String>) -> Result<(), String> {
        self.check_idle()?;
        match read {
            Ok(content) => self.load(&content),
            Err(error) => self.load_failed(&error),
        }
        Ok(())
    }

    /// Called when the file changed on disk. Hosven's own write leaves the
    /// content it wrote, which is not an outside change.
    pub fn disk_changed(&mut self, on_disk: Option<&str>) {
        if on_disk != Some(self.original.as_str()) {
            self.external_change = true;
        }
    }

    fn replace_from(&mut self, content: &str) {
        let Parsed {
            entries,
            comment_lines,
            tags,
        } = parse::parse(content);
        self.entries = entries;
        self.comment_lines = comment_lines;
        self.tags = tags;
        self.adopt_profile_spelling();
        self.sync_profiles_with_tags();
    }

    pub fn serialize(&self) -> String {
        parse::serialize_parts(&self.entries, &self.comment_lines, &self.tags)
    }

    // MARK: - Dirty state

    pub fn capture_clean(&mut self) {
        self.pristine = self.serialize();
    }

    /// The raw editor's text when it differs from the model's `text`.
    fn pending_draft(&self, text: &str) -> Option<&str> {
        self.raw_draft.as_deref().filter(|draft| *draft != text)
    }

    pub fn dirty(&self) -> bool {
        self.dirty_with(&self.serialize())
    }

    fn dirty_with(&self, text: &str) -> bool {
        self.pending_draft(text).is_some() || text != self.pristine
    }

    /// Lines that differ between the last clean state and what would be
    /// written now, counting repeated lines each time they occur.
    pub fn pending_count(&self) -> usize {
        self.pending_count_with(&self.serialize())
    }

    fn pending_count_with(&self, text: &str) -> usize {
        let draft = self.pending_draft(text);
        let diff = changed_lines(&self.pristine, draft.unwrap_or(text));
        if diff == 0 && draft.is_some() {
            1
        } else {
            diff
        }
    }

    /// Whether anything is pending and how much, from one serialisation.
    pub fn pending_state(&self) -> (bool, usize) {
        let text = self.serialize();
        (self.dirty_with(&text), self.pending_count_with(&text))
    }

    /// Holds the raw editor's text so an edit or switch started elsewhere
    /// folds the typing in instead of regenerating the editor over it.
    /// `None` when the editor matches the model again. `base` is the model
    /// text the typing started from: a draft typed over text that a switch,
    /// reload or edit has since replaced would undo that change when merged,
    /// so it is dropped.
    pub fn set_raw_draft(&mut self, text: Option<String>, base: &str) {
        if text.is_none() || base == self.serialize() {
            self.raw_draft = text;
        }
    }

    pub(super) fn merge_raw_draft(&mut self) {
        if let Some(draft) = self.raw_draft.take()
            && draft != self.serialize()
        {
            self.history.push(&self.entries);
            self.reparse_keeping_ids(&draft);
        }
    }

    /// Parses `content` into the model, giving each line that did not change
    /// the id it had, so the UI's references to it stay valid.
    fn reparse_keeping_ids(&mut self, content: &str) {
        let mut old: HashMap<(String, String, Option<String>), VecDeque<Uuid>> = HashMap::new();
        for o in std::mem::take(&mut self.entries) {
            old.entry((o.ip, o.hostname, o.tag))
                .or_default()
                .push_back(o.id);
        }
        self.replace_from(content);
        for entry in &mut self.entries {
            let key = (entry.ip.clone(), entry.hostname.clone(), entry.tag.clone());
            if let Some(id) = old.get_mut(&key).and_then(VecDeque::pop_front) {
                entry.id = id;
            }
        }
    }

    pub(super) fn check_idle(&self) -> Result<(), String> {
        if self.applying {
            Err(BUSY.to_string())
        } else {
            Ok(())
        }
    }

    /// Every change to entries starts here: nothing changes while a write is
    /// out, and typing from the raw editor lands first, so the change goes
    /// on top of it.
    fn prepare_edit(&mut self) -> Result<(), String> {
        self.check_idle()?;
        self.merge_raw_draft();
        Ok(())
    }

    /// Prepares an edit and records it as one undo step.
    fn begin_edit(&mut self) -> Result<(), String> {
        self.prepare_edit()?;
        self.history.push(&self.entries);
        Ok(())
    }

    /// Returns to the last loaded or written state.
    pub fn discard(&mut self) -> Result<(), String> {
        self.check_idle()?;
        if self.original.is_empty() {
            return Ok(());
        }
        self.raw_draft = None;
        self.history.push(&self.entries);
        let original = self.original.clone();
        self.reparse_keeping_ids(&original);
        self.capture_clean();
        Ok(())
    }

    // MARK: - Undo

    pub fn undo(&mut self) -> Result<(), String> {
        self.check_idle()?;
        self.merge_raw_draft();
        self.history.undo(&mut self.entries);
        Ok(())
    }

    pub fn redo(&mut self) -> Result<(), String> {
        self.check_idle()?;
        self.merge_raw_draft();
        self.history.redo(&mut self.entries);
        Ok(())
    }

    // MARK: - Entries

    pub fn add_entry(&mut self, input: EntryInput) -> Result<(), String> {
        let mut entry = HostEntry::new("", "", "", None);
        validate(&mut entry, input)?;
        self.begin_edit()?;
        self.add_tag(&entry.tag);
        self.show_toast(format!("Đã thêm {}", entry.hostname), ToastKind::Success);
        self.entries.push(entry);
        Ok(())
    }

    pub fn update_entry(&mut self, id: Uuid, input: EntryInput) -> Result<(), String> {
        self.prepare_edit()?;
        let index = self.index_of(id)?;
        let mut entry = self.entries[index].clone();
        validate(&mut entry, input)?;
        self.history.push(&self.entries);
        self.add_tag(&entry.tag);
        self.entries[index] = entry;
        Ok(())
    }

    fn add_tag(&mut self, tag: &Option<String>) {
        if let Some(tag) = tag
            && !self.tags.contains(tag)
        {
            self.tags.push(tag.clone());
        }
    }

    pub fn duplicate_entry(&mut self, id: Uuid) -> Result<(), String> {
        self.begin_edit()?;
        let index = self.index_of(id)?;
        let mut copy = self.entries[index].clone();
        copy.id = Uuid::new_v4();
        self.entries.insert(index + 1, copy);
        Ok(())
    }

    pub fn toggle_entry(&mut self, id: Uuid) -> Result<(), String> {
        self.begin_edit()?;
        let index = self.index_of(id)?;
        self.entries[index].enabled = !self.entries[index].enabled;
        Ok(())
    }

    pub fn delete_entry(&mut self, id: Uuid) -> Result<(), String> {
        self.begin_edit()?;
        self.entries.retain(|e| e.id != id);
        Ok(())
    }

    fn index_of(&self, id: Uuid) -> Result<usize, String> {
        self.entries
            .iter()
            .position(|e| e.id == id)
            .ok_or_else(|| "Không tìm thấy entry".to_string())
    }

    /// Whether any entry already answers for `hostname`, aliases included.
    #[cfg(test)]
    pub fn hostname_exists(&self, hostname: &str) -> bool {
        let wanted = hostname.to_lowercase();
        self.entries
            .iter()
            .any(|e| e.hostnames().any(|h| h.to_lowercase() == wanted))
    }

    // MARK: - Blocks

    pub fn set_tag_enabled(&mut self, name: &str, enabled: bool) -> Result<(), String> {
        self.begin_edit()?;
        for entry in self
            .entries
            .iter_mut()
            .filter(|e| e.tag.as_deref() == Some(name))
        {
            entry.enabled = enabled;
        }
        Ok(())
    }

    /// Flips a block and writes it at once, for the menu bar panel. Edits
    /// pending in the window refuse it, since it would write them too. Like a
    /// switch, it becomes an undo step only once it is on disk.
    pub fn begin_block_write(&mut self, name: &str, enabled: bool) -> Step {
        if let Some(reason) = self.write_blocker() {
            return self.blocked(reason);
        }
        if self.dirty() {
            return Step::Done(Outcome::Refused(self.pending_count()));
        }
        let mut block = self
            .entries
            .iter()
            .filter(|e| e.tag.as_deref() == Some(name))
            .peekable();
        if block.peek().is_none() || block.all(|e| e.enabled == enabled) {
            return Step::Done(Outcome::NothingToDo);
        }
        let before = self.entries.clone();
        for entry in self
            .entries
            .iter_mut()
            .filter(|e| e.tag.as_deref() == Some(name))
        {
            entry.enabled = enabled;
        }
        let toast = format!("Đã {} block {name}", if enabled { "bật" } else { "tắt" });
        Step::Write(self.begin_switch_write(before, toast))
    }

    pub fn tag_entry_count(&self, name: &str) -> usize {
        self.entries
            .iter()
            .filter(|e| e.tag.as_deref() == Some(name))
            .count()
    }

    // MARK: - Profiles

    /// A marker spelled `## [tag:dev]` belongs to the profile `Dev`; the
    /// profile's spelling wins, so the environment keeps its entries.
    fn adopt_profile_spelling(&mut self) {
        for tag in &mut self.tags {
            let Some(profile) = self
                .profiles
                .iter()
                .find(|p| p.name != *tag && p.name.to_lowercase() == tag.to_lowercase())
            else {
                continue;
            };
            for entry in &mut self.entries {
                if entry.tag.as_ref() == Some(tag) {
                    entry.tag = Some(profile.name.clone());
                }
            }
            *tag = profile.name.clone();
        }
    }

    /// Gives every block found in the file a profile. A block starts
    /// independent; the user marks it an environment. Profiles whose block is
    /// gone are kept, so their metadata survives the block coming back.
    fn sync_profiles_with_tags(&mut self) {
        let missing: Vec<String> = self
            .tags
            .iter()
            .filter(|tag| !self.profiles.iter().any(|p| &p.name == *tag))
            .cloned()
            .collect();
        if missing.is_empty() {
            return;
        }
        for tag in missing {
            let mut profile = Profile::new(&tag, ProfileColor::guess(&tag), false);
            profile.shortcut_number = Some(self.next_shortcut()).filter(|n| *n <= 9);
            self.profiles.push(profile);
        }
        self.save_profiles();
    }

    fn next_shortcut(&self) -> u32 {
        self.profiles
            .iter()
            .filter_map(|p| p.shortcut_number)
            .max()
            .unwrap_or(0)
            + 1
    }

    fn check_name(&self, name: &str, except: Option<Uuid>) -> Result<String, String> {
        let trimmed = name.trim().to_string();
        if !profiles::is_valid_name(&trimmed) {
            return Err("Tên không được để trống và không chứa [ hoặc ]".into());
        }
        let lower = trimmed.to_lowercase();
        if self
            .profiles
            .iter()
            .any(|p| Some(p.id) != except && p.name.to_lowercase() == lower)
        {
            return Err(format!("Đã có profile tên {trimmed}"));
        }
        Ok(trimmed)
    }

    pub fn add_profile(
        &mut self,
        name: &str,
        color: ProfileColor,
        is_exclusive: bool,
    ) -> Result<Profile, String> {
        let name = self.check_name(name, None)?;
        let mut profile = Profile::new(&name, color, is_exclusive);
        profile.shortcut_number = Some(self.next_shortcut());
        self.profiles.push(profile.clone());
        self.save_profiles();
        Ok(profile)
    }

    /// Renames the profile and the block its entries sit in. Not an undo
    /// step: the rename follows into the undo history instead.
    pub fn rename_profile(&mut self, id: Uuid, new_name: &str) -> Result<(), String> {
        self.check_idle()?;
        let index = self.profile_index(id)?;
        let name = self.check_name(new_name, Some(id))?;
        self.merge_raw_draft();
        let old = std::mem::replace(&mut self.profiles[index].name, name.clone());
        for entry in self
            .entries
            .iter_mut()
            .filter(|e| e.tag.as_deref() == Some(old.as_str()))
        {
            entry.tag = Some(name.clone());
        }
        if let Some(tag) = self.tags.iter_mut().find(|t| **t == old) {
            *tag = name.clone();
        }
        self.history.rename_tag(&old, &name);
        self.save_profiles();
        Ok(())
    }

    /// Drops the metadata only; the block's entries stay in the file.
    pub fn remove_profile(&mut self, id: Uuid) {
        self.profiles.retain(|p| p.id != id);
        self.save_profiles();
    }

    pub fn set_profile_exclusive(&mut self, id: Uuid, is_exclusive: bool) -> Result<(), String> {
        let index = self.profile_index(id)?;
        self.profiles[index].is_exclusive = is_exclusive;
        self.save_profiles();
        Ok(())
    }

    pub fn set_profile_color(&mut self, id: Uuid, color: ProfileColor) -> Result<(), String> {
        let index = self.profile_index(id)?;
        self.profiles[index].color = color;
        self.save_profiles();
        Ok(())
    }

    fn profile_index(&self, id: Uuid) -> Result<usize, String> {
        self.profiles
            .iter()
            .position(|p| p.id == id)
            .ok_or_else(|| "Không tìm thấy profile".to_string())
    }

    fn save_profiles(&mut self) {
        if let Err(error) = self.store.save(&self.profiles) {
            self.show_toast(
                format!("Không lưu được profiles.json: {error}"),
                ToastKind::Error,
            );
        }
    }

    // MARK: - Writing

    /// Why nothing may be written now, if anything.
    pub(super) fn write_blocker(&self) -> Option<String> {
        if self.applying {
            Some(BUSY.to_string())
        } else if self.load_error.is_some() {
            Some("Không đọc được /etc/hosts nên Hosven không ghi đè nó. Tải lại để thử lại.".into())
        } else if self.external_change {
            Some("/etc/hosts vừa đổi bên ngoài Hosven. Tải lại trước khi áp dụng.".into())
        } else {
            None
        }
    }

    pub(super) fn blocked(&mut self, reason: String) -> Step {
        self.show_toast(reason.clone(), ToastKind::Error);
        Step::Done(Outcome::Failed(reason))
    }

    /// Prepares writing the model. Typing still in the raw editor is folded
    /// in first, so it is written rather than lost.
    pub fn begin_apply(&mut self) -> Step {
        if let Some(reason) = self.write_blocker() {
            return self.blocked(reason);
        }
        self.merge_raw_draft();
        if !self.dirty() {
            return self.nothing_to_apply();
        }
        let content = self.serialize();
        Step::Write(self.begin_write(content, false, None, None))
    }

    /// Prepares writing the raw editor's text as it is.
    /// Writes the raw editor's text, typed over `base`. Text typed over a
    /// model that has changed since would undo that change, so it is refused
    /// and the editor takes the new text.
    pub fn begin_apply_raw(&mut self, text: &str, base: &str) -> Step {
        if let Some(reason) = self.write_blocker() {
            return self.blocked(reason);
        }
        if base != self.serialize() {
            self.show_toast(STALE_EDITOR.into(), ToastKind::Error);
            return Step::Done(Outcome::NothingToDo);
        }
        if text.trim().is_empty() {
            return self.nothing_to_apply();
        }
        let mut content = text.to_string();
        if !content.ends_with('\n') {
            content.push('\n');
        }
        if content == self.pristine && !self.dirty() {
            return self.nothing_to_apply();
        }
        Step::Write(self.begin_write(content, true, None, None))
    }

    fn nothing_to_apply(&mut self) -> Step {
        self.show_toast("Không có gì để áp dụng".into(), ToastKind::Info);
        Step::Done(Outcome::NothingToDo)
    }

    pub(super) fn begin_switch_write(&mut self, before: Vec<HostEntry>, toast: String) -> WriteJob {
        let content = self.serialize();
        self.begin_write(
            content,
            false,
            Some(Rollback { entries: before }),
            Some(toast),
        )
    }

    fn begin_write(
        &mut self,
        content: String,
        reparse: bool,
        rollback: Option<Rollback>,
        success_toast: Option<String>,
    ) -> WriteJob {
        self.applying = true;
        self.verification = None;
        self.write_seq += 1;
        WriteJob {
            changed: verify::changed_pairs(&self.pristine, &content),
            seq: self.write_seq,
            content,
            reparse,
            rollback,
            success_toast,
        }
    }

    /// Records how a prepared write ended. A switch that did not land puts the
    /// entries back, so the model keeps matching the disk.
    pub fn finish_write(&mut self, job: WriteJob, outcome: RunOutcome) -> Outcome {
        self.applying = false;
        match outcome {
            RunOutcome::Ok => {
                self.original = job.content.clone();
                self.raw_draft = None;
                if job.reparse {
                    self.reparse_keeping_ids(&job.content);
                }
                self.pristine = self.serialize();
                self.external_change = false;
                // A switch becomes an undo step only once it is on disk, so
                // one that did not land leaves the history as it was.
                if let Some(rollback) = &job.rollback {
                    self.history.push(&rollback.entries);
                }
                self.history.clear_redo();
                let message = job.success_toast.unwrap_or_else(|| {
                    format!("Đã ghi {} entry vào /etc/hosts", self.entries.len())
                });
                self.show_toast(message, ToastKind::Success);
                Outcome::Written
            }
            RunOutcome::Cancelled => {
                self.roll_back(job.rollback);
                Outcome::Cancelled
            }
            failed => {
                let error = failed.failure().unwrap_or_default();
                self.roll_back(job.rollback);
                self.show_toast(
                    format!("Không ghi được /etc/hosts: {error}"),
                    ToastKind::Error,
                );
                Outcome::Failed(error)
            }
        }
    }

    fn roll_back(&mut self, rollback: Option<Rollback>) {
        if let Some(rollback) = rollback {
            self.entries = rollback.entries;
        }
    }

    /// Keeps the resolver check of write `seq`, unless a later write started.
    pub fn set_verification(&mut self, seq: u64, verification: Verification) {
        if seq == self.write_seq {
            self.verification = Some(verification);
        }
    }

    pub fn dismiss_verification(&mut self) {
        self.verification = None;
    }

    pub fn show_toast(&mut self, message: String, kind: ToastKind) {
        self.toasts.show(message, kind);
    }

    #[cfg(test)]
    pub(super) fn toast(&self) -> Option<&Toast> {
        self.toasts.current()
    }

    #[cfg(test)]
    pub(super) fn clear_toast(&mut self) {
        self.toasts.clear();
    }
}

/// Fills `entry` from the form, refusing what would break the file: a field
/// with a newline would write extra lines, and spaces or `#` in an address or
/// hostname would split it into other fields on the next read.
fn validate(entry: &mut HostEntry, input: EntryInput) -> Result<(), String> {
    entry.ip = token(&input.ip, "IP", "Cần một địa chỉ IP")?;
    entry.hostname = token(&input.hostname, "Hostname", "Cần một hostname")?;
    if let Some(aliases) = input.aliases {
        entry.aliases = aliases
            .iter()
            .filter(|a| !a.trim().is_empty())
            .map(|a| token(a, "Alias", ""))
            .collect::<Result<_, _>>()?;
    }
    let comment = input.comment.trim();
    if comment.chars().any(char::is_control) {
        return Err("Ghi chú phải nằm trên một dòng".into());
    }
    entry.comment = comment.to_string();
    entry.tag = match input.tag.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(tag) if profiles::is_valid_name(tag) => Some(tag.to_string()),
        Some(_) => return Err("Tên block không được chứa [ hoặc ]".into()),
    };
    Ok(())
}

fn token(value: &str, label: &str, missing: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(missing.to_string());
    }
    if value
        .chars()
        .any(|c| c.is_whitespace() || c.is_control() || c == '#')
    {
        return Err(format!("{label} không được chứa khoảng trắng hoặc #"));
    }
    Ok(value.to_string())
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::profiles::MemoryStore;

    pub const BASE: &str = "127.0.0.1\tlocalhost\n";

    /// A model seeded with `content` as the clean state on disk.
    pub fn hosts(content: &str, profiles: Vec<Profile>) -> (Hosts, MemoryStore) {
        let store = MemoryStore::with(profiles);
        let mut hosts = Hosts::new(Box::new(store.clone()));
        hosts.load(content);
        (hosts, store)
    }

    /// Runs a prepared write with the given result, as the command layer would.
    pub fn complete(
        hosts: &mut Hosts,
        step: Step,
        outcome: RunOutcome,
    ) -> (Outcome, Option<String>) {
        match step {
            Step::Write(job) => {
                let content = job.content.clone();
                (hosts.finish_write(job, outcome), Some(content))
            }
            Step::Done(outcome) => (outcome, None),
        }
    }

    pub fn input(ip: &str, hostname: &str) -> EntryInput {
        EntryInput {
            ip: ip.into(),
            hostname: hostname.into(),
            ..Default::default()
        }
    }

    fn job(step: Step) -> WriteJob {
        match step {
            Step::Write(job) => job,
            Step::Done(outcome) => panic!("expected a write, got {outcome:?}"),
        }
    }

    const FIXTURE: &str = "127.0.0.1\tlocalhost
## [tag:Release]
127.0.0.1\tapi.dev.example.com
## [/tag:Release]
## [tag:Production]
10.0.0.5\tapi.prod.example.com
## [/tag:Production]
";

    // MARK: Dirty state

    #[test]
    fn a_fresh_model_is_clean() {
        let (h, _) = hosts(BASE, vec![]);
        assert!(!h.dirty());
        assert_eq!(h.pending_count(), 0);
    }

    #[test]
    fn adding_an_entry_makes_it_dirty() {
        let (mut h, _) = hosts(BASE, vec![]);
        h.add_entry(input("127.0.0.1", "a.test")).unwrap();
        assert!(h.dirty());
        assert_eq!(h.pending_count(), 1);
    }

    #[test]
    fn changing_back_clears_dirty() {
        let (mut h, _) = hosts(BASE, vec![]);
        let id = h.entries[0].id;
        h.toggle_entry(id).unwrap();
        assert!(h.dirty());
        h.toggle_entry(id).unwrap();
        assert!(!h.dirty());
    }

    #[test]
    fn undo_to_the_loaded_state_clears_dirty() {
        let (mut h, _) = hosts(BASE, vec![]);
        h.add_entry(input("127.0.0.1", "a.test")).unwrap();
        assert!(h.dirty());
        h.undo().unwrap();
        assert!(!h.dirty());
    }

    #[test]
    fn capture_clean_makes_the_current_state_pristine() {
        let (mut h, _) = hosts(BASE, vec![]);
        h.add_entry(input("127.0.0.1", "a.test")).unwrap();
        h.capture_clean();
        assert!(!h.dirty());
    }

    #[test]
    fn a_raw_draft_is_dirty_only_when_it_differs() {
        let (mut h, _) = hosts(BASE, vec![]);
        h.set_raw_draft(Some(h.serialize()), &h.serialize());
        assert!(!h.dirty());
        h.set_raw_draft(
            Some(format!("{BASE}127.0.0.1\ttyped.test\n")),
            &h.serialize(),
        );
        assert!(h.dirty());
        assert_eq!(h.pending_count(), 1);
        h.set_raw_draft(None, &h.serialize());
        assert!(!h.dirty());
    }

    #[test]
    fn repeated_lines_are_counted_each_time() {
        let (mut h, _) = hosts(BASE, vec![]);
        h.add_entry(input("127.0.0.1", "localhost")).unwrap();
        assert_eq!(h.pending_count(), 1);
    }

    #[test]
    fn the_pending_state_follows_the_draft_and_matches_the_snapshot() {
        let (mut h, _) = hosts(BASE, vec![]);
        assert_eq!(h.pending_state(), (false, 0));
        h.set_raw_draft(Some(h.serialize()), &h.serialize());
        assert_eq!(h.pending_state(), (false, 0), "typing back the same text");
        h.set_raw_draft(
            Some(format!("{BASE}127.0.0.1\ta.test\n127.0.0.1\tb.test\n")),
            &h.serialize(),
        );
        assert_eq!(h.pending_state(), (true, 2));
        let snapshot = h.snapshot();
        assert_eq!(h.pending_state(), (snapshot.dirty, snapshot.pending_count));
    }

    #[test]
    fn discard_returns_to_the_loaded_state() {
        let (mut h, _) = hosts(BASE, vec![]);
        h.add_entry(input("127.0.0.1", "a.test")).unwrap();
        h.discard().unwrap();
        assert!(!h.dirty());
        assert_eq!(h.entries.len(), 1);
        assert!(h.history.can_undo(), "the discard itself can be undone");
    }

    // MARK: Undo

    #[test]
    fn undo_and_redo() {
        let (mut h, _) = hosts(BASE, vec![]);
        assert!(!h.history.can_undo() && !h.history.can_redo());
        let id = h.entries[0].id;
        h.toggle_entry(id).unwrap();
        assert!(!h.entries[0].enabled);
        h.undo().unwrap();
        assert!(h.entries[0].enabled);
        assert!(!h.history.can_undo() && h.history.can_redo());
        h.redo().unwrap();
        assert!(!h.entries[0].enabled);
        assert!(!h.history.can_redo());
    }

    #[test]
    fn undo_restores_a_deleted_entry() {
        let (mut h, _) = hosts(BASE, vec![]);
        let id = h.entries[0].id;
        h.delete_entry(id).unwrap();
        assert!(h.entries.is_empty());
        h.undo().unwrap();
        assert_eq!(h.entries[0].hostname, "localhost");
    }

    #[test]
    fn a_new_change_clears_redo() {
        let (mut h, _) = hosts(BASE, vec![]);
        let id = h.entries[0].id;
        h.toggle_entry(id).unwrap();
        h.undo().unwrap();
        assert!(h.history.can_redo());
        h.toggle_entry(id).unwrap();
        assert!(!h.history.can_redo());
    }

    #[test]
    fn the_undo_stack_keeps_the_last_fifty() {
        let (mut h, _) = hosts(BASE, vec![]);
        let id = h.entries[0].id;
        for _ in 0..60 {
            h.toggle_entry(id).unwrap();
        }
        assert_eq!(h.history.depth(), 50);
    }

    #[test]
    fn identical_snapshots_are_kept_once() {
        let (mut h, _) = hosts(BASE, vec![]);
        h.set_tag_enabled("nothing", true).unwrap();
        h.set_tag_enabled("nothing", true).unwrap();
        assert_eq!(h.history.depth(), 1);
    }

    // MARK: Raw editor draft

    #[test]
    fn an_edit_lands_on_top_of_the_raw_draft() {
        let (mut h, _) = hosts(BASE, vec![]);
        h.set_raw_draft(
            Some(format!("{BASE}127.0.0.1\ttyped.test\n")),
            &h.serialize(),
        );
        let id = h.entries[0].id;
        h.toggle_entry(id).unwrap();
        let job = job(h.begin_apply());
        assert!(job.content.contains("typed.test"), "the typing is kept");
        assert!(
            job.content.contains("# 127.0.0.1\tlocalhost"),
            "and so is the toggle"
        );
    }

    #[test]
    fn undo_first_takes_back_the_edit_then_the_typing() {
        let (mut h, _) = hosts(BASE, vec![]);
        h.set_raw_draft(
            Some(format!("{BASE}127.0.0.1\ttyped.test\n")),
            &h.serialize(),
        );
        h.add_entry(input("127.0.0.1", "form.test")).unwrap();
        h.undo().unwrap();
        assert!(h.hostname_exists("typed.test") && !h.hostname_exists("form.test"));
        h.undo().unwrap();
        assert!(!h.hostname_exists("typed.test"));
    }

    // MARK: Entries and blocks

    #[test]
    fn duplicate_inserts_a_copy_after_the_source() {
        let (mut h, _) = hosts(FIXTURE, vec![]);
        let id = h.entries[1].id;
        h.duplicate_entry(id).unwrap();
        assert_eq!(h.entries[2].hostname, "api.dev.example.com");
        assert_ne!(h.entries[2].id, id);
        assert_eq!(h.entries[2].tag.as_deref(), Some("Release"));
    }

    #[test]
    fn hostname_lookup_ignores_case_and_sees_aliases() {
        let (h, _) = hosts("127.0.0.1\tapi.test www.test\n", vec![]);
        assert!(h.hostname_exists("API.test"));
        assert!(h.hostname_exists("www.test"));
        assert!(!h.hostname_exists("db.test"));
    }

    #[test]
    fn a_block_switch_sets_every_entry_in_it() {
        let (mut h, _) = hosts(FIXTURE, vec![]);
        let id = h.entries[2].id;
        h.update_entry(
            id,
            EntryInput {
                tag: Some("Release".into()),
                ..input("10.0.0.5", "api.prod.example.com")
            },
        )
        .unwrap();
        h.toggle_entry(id).unwrap();
        h.set_tag_enabled("Release", false).unwrap();
        let release = |h: &Hosts| -> Vec<bool> {
            h.entries
                .iter()
                .filter(|e| e.tag.as_deref() == Some("Release"))
                .map(|e| e.enabled)
                .collect()
        };
        assert_eq!(release(&h), [false, false]);
        assert_eq!(h.tag_entry_count("Production"), 0);
        h.set_tag_enabled("Release", true).unwrap();
        assert_eq!(release(&h), [true, true]);
    }

    #[test]
    fn an_entry_in_a_new_profile_reaches_the_file() {
        let (mut h, _) = hosts(BASE, vec![]);
        h.add_profile("QA", ProfileColor::Amber, true).unwrap();
        h.add_entry(EntryInput {
            tag: Some("QA".into()),
            ..input("10.0.0.7", "qa.test")
        })
        .unwrap();
        assert!(
            h.serialize()
                .contains("## [tag:QA]\n10.0.0.7\tqa.test\n## [/tag:QA]")
        );
        assert_eq!(h.tag_entry_count("QA"), 1);
    }

    #[test]
    fn fields_that_would_break_the_file_are_refused() {
        let (mut h, _) = hosts(BASE, vec![]);
        let cases = [
            (input("", "a.test"), "Cần một địa chỉ IP"),
            (input("127.0.0.1", " "), "Cần một hostname"),
            (
                input("127.0.0.1 1", "a.test"),
                "IP không được chứa khoảng trắng hoặc #",
            ),
            (
                input("127.0.0.1", "a.test#b"),
                "Hostname không được chứa khoảng trắng hoặc #",
            ),
            (
                EntryInput {
                    aliases: Some(vec!["b c".into()]),
                    ..input("127.0.0.1", "a.test")
                },
                "Alias không được chứa khoảng trắng hoặc #",
            ),
            (
                EntryInput {
                    comment: "x\n0.0.0.0 bank.example".into(),
                    ..input("127.0.0.1", "a.test")
                },
                "Ghi chú phải nằm trên một dòng",
            ),
            (
                EntryInput {
                    tag: Some("T]\nzz".into()),
                    ..input("127.0.0.1", "a.test")
                },
                "Tên block không được chứa [ hoặc ]",
            ),
        ];
        for (input, message) in cases {
            assert_eq!(h.add_entry(input).unwrap_err(), message);
        }
        assert_eq!(h.entries.len(), 1);
        assert!(!h.history.can_undo(), "a refused edit is not an undo step");

        let id = h.entries[0].id;
        assert!(h.update_entry(id, input("127.0.0.1", "")).is_err());
        assert!(!h.history.can_undo());
    }

    #[test]
    fn editing_without_aliases_keeps_them() {
        let (mut h, _) = hosts("127.0.0.1\tapi.test www.test\n", vec![]);
        let id = h.entries[0].id;
        h.update_entry(id, input("127.0.0.1", "api2.test")).unwrap();
        assert_eq!(h.entries[0].aliases, ["www.test"]);
        h.update_entry(
            id,
            EntryInput {
                aliases: Some(vec![]),
                ..input("127.0.0.1", "api2.test")
            },
        )
        .unwrap();
        assert!(h.entries[0].aliases.is_empty());
    }

    // MARK: Profiles

    #[test]
    fn blocks_found_in_the_file_get_profiles() {
        let (h, store) = hosts(FIXTURE, vec![]);
        let release = h.profiles.iter().find(|p| p.name == "Release").unwrap();
        let production = h.profiles.iter().find(|p| p.name == "Production").unwrap();
        assert_eq!(release.color, ProfileColor::Purple);
        assert_eq!(production.color, ProfileColor::Red);
        assert!(!release.is_exclusive && !production.is_exclusive);
        assert!(h.profiles.iter().all(|p| p.shortcut_number.is_some()));
        assert_eq!(store.stored().len(), 2);
    }

    #[test]
    fn a_marker_in_other_case_belongs_to_the_profile() {
        let dev = Profile::new("Dev", ProfileColor::Blue, true);
        let id = dev.id;
        let (h, _) = hosts(
            "## [tag:dev]\n127.0.0.1\tapi.test\n## [/tag:dev]\n",
            vec![dev],
        );
        assert_eq!(h.profiles.len(), 1, "no second profile for the same block");
        assert_eq!(h.entries[0].tag.as_deref(), Some("Dev"));
        assert_eq!(h.active_state(), ActiveState::Active { id });
    }

    #[test]
    fn a_profile_without_its_block_is_kept() {
        let staging = Profile::new("Staging", ProfileColor::Amber, true);
        let (h, _) = hosts(FIXTURE, vec![staging]);
        assert_eq!(h.profiles.len(), 3);
        assert!(h.profiles.iter().any(|p| p.name == "Staging"));
    }

    #[test]
    fn add_profile_checks_the_name() {
        let (mut h, _) = hosts(FIXTURE, vec![]);
        assert_eq!(
            h.add_profile("release", ProfileColor::Blue, false)
                .unwrap_err(),
            "Đã có profile tên release"
        );
        assert!(h.add_profile("", ProfileColor::Blue, false).is_err());
        assert!(
            h.add_profile("Bad[Name", ProfileColor::Blue, false)
                .is_err()
        );
        assert!(h.add_profile("QA", ProfileColor::Blue, false).is_ok());
    }

    #[test]
    fn remove_profile_persists() {
        let (mut h, store) = hosts(FIXTURE, vec![]);
        let id = h.profiles[0].id;
        h.remove_profile(id);
        assert!(!store.stored().iter().any(|p| p.id == id));
        assert_eq!(h.tag_entry_count("Release"), 1, "the entries stay");
    }

    #[test]
    fn rename_moves_the_entries_to_the_new_block() {
        let (mut h, store) = hosts(FIXTURE, vec![]);
        let id = h.profiles.iter().find(|p| p.name == "Release").unwrap().id;
        h.rename_profile(id, "Dev").unwrap();
        assert!(h.entries.iter().any(|e| e.tag.as_deref() == Some("Dev")));
        assert!(
            !h.entries
                .iter()
                .any(|e| e.tag.as_deref() == Some("Release"))
        );
        assert!(h.serialize().contains("## [tag:Dev]"));
        assert!(h.dirty());
        assert!(store.stored().iter().any(|p| p.name == "Dev"));
    }

    #[test]
    fn undo_after_a_rename_keeps_the_new_block() {
        let (mut h, _) = hosts(FIXTURE, vec![]);
        let api = h.entries[1].id;
        h.toggle_entry(api).unwrap();
        let id = h.profiles.iter().find(|p| p.name == "Release").unwrap().id;
        h.rename_profile(id, "Dev").unwrap();
        h.undo().unwrap();
        assert!(h.entries[1].enabled, "the toggle is undone");
        assert_eq!(h.entries[1].tag.as_deref(), Some("Dev"), "the rename stays");
    }

    #[test]
    fn rename_checks_the_name() {
        let (mut h, _) = hosts(FIXTURE, vec![]);
        let id = h.profiles.iter().find(|p| p.name == "Release").unwrap().id;
        assert!(h.rename_profile(id, "Production").is_err());
        assert!(h.rename_profile(id, "").is_err());
        assert!(h.rename_profile(Uuid::new_v4(), "Whatever").is_err());
        assert!(
            h.rename_profile(id, "release").is_ok(),
            "a case change of its own name is fine"
        );
    }

    #[test]
    fn shortcuts_continue_from_the_highest() {
        let mut existing = Profile::new("Existing", ProfileColor::Red, true);
        existing.shortcut_number = Some(3);
        let (h, _) = hosts(FIXTURE, vec![existing]);
        let shortcuts: Vec<_> = h.profiles[1..]
            .iter()
            .map(|p| p.shortcut_number.unwrap())
            .collect();
        assert_eq!(shortcuts, [4, 5]);
    }

    // MARK: Writing

    #[test]
    fn a_raw_write_reparses_and_lists_changed_hostnames() {
        let (mut h, _) = hosts(BASE, vec![]);
        let step = h.begin_apply_raw(&format!("{BASE}127.0.0.1\tapi.test"), &h.serialize());
        let Step::Write(job) = step else { panic!() };
        assert_eq!(
            job.changed,
            [("api.test".to_string(), "127.0.0.1".to_string())]
        );
        let (outcome, content) = complete(&mut h, Step::Write(job), RunOutcome::Ok);
        assert_eq!(outcome, Outcome::Written);
        assert_eq!(content.unwrap(), format!("{BASE}127.0.0.1\tapi.test\n"));
        assert!(!h.dirty());
        assert!(h.entries.iter().any(|e| e.hostname == "api.test"));
    }

    #[test]
    fn a_model_write_keeps_entry_ids() {
        let (mut h, _) = hosts(BASE, vec![]);
        h.add_entry(input("127.0.0.1", "web.test")).unwrap();
        let ids: Vec<_> = h.entries.iter().map(|e| e.id).collect();
        let job = job(h.begin_apply());
        assert_eq!(job.content, h.serialize());
        complete(&mut h, Step::Write(job), RunOutcome::Ok);
        assert_eq!(h.entries.iter().map(|e| e.id).collect::<Vec<_>>(), ids);
        assert!(!h.dirty());
        assert_eq!(h.toast().unwrap().message, "Đã ghi 2 entry vào /etc/hosts");
    }

    #[test]
    fn a_clean_model_has_nothing_to_apply() {
        let (mut h, _) = hosts(BASE, vec![]);
        assert!(matches!(h.begin_apply(), Step::Done(Outcome::NothingToDo)));
        assert_eq!(h.toast().unwrap().message, "Không có gì để áp dụng");
        h.clear_toast();
        assert!(matches!(
            h.begin_apply_raw(BASE, &h.serialize()),
            Step::Done(Outcome::NothingToDo)
        ));
        assert_eq!(h.toast().unwrap().kind, ToastKind::Info);
        assert!(
            matches!(
                h.begin_apply_raw("127.0.0.1 localhost\n", &h.serialize()),
                Step::Write(_)
            ),
            "a formatting change is still a change"
        );
    }

    #[test]
    fn apply_folds_in_typing_from_the_raw_editor() {
        let (mut h, _) = hosts(BASE, vec![]);
        h.set_raw_draft(
            Some(format!("{BASE}127.0.0.1\ttyped.test\n")),
            &h.serialize(),
        );
        assert!(job(h.begin_apply()).content.contains("typed.test"));
    }

    #[test]
    fn typing_over_text_the_model_has_replaced_is_not_kept_or_written() {
        let (mut h, _) = hosts(BASE, vec![]);
        let before = h.serialize();
        h.add_entry(input("127.0.0.1", "web.test")).unwrap();
        h.set_raw_draft(Some(format!("{before}127.0.0.1\ttyped.test\n")), &before);
        assert!(!h.serialize().contains("typed.test"));
        assert!(job(h.begin_apply()).content.contains("web.test"));

        let (mut h, _) = hosts(BASE, vec![]);
        let before = h.serialize();
        h.add_entry(input("127.0.0.1", "web.test")).unwrap();
        assert!(matches!(
            h.begin_apply_raw(&format!("{before}127.0.0.1\ttyped.test\n"), &before),
            Step::Done(Outcome::NothingToDo)
        ));
        assert_eq!(h.toast().unwrap().message, STALE_EDITOR);
        assert!(!h.applying);
    }

    #[test]
    fn a_cancelled_prompt_keeps_edits_pending_and_says_nothing() {
        let (mut h, _) = hosts(BASE, vec![]);
        h.add_entry(input("127.0.0.1", "web.test")).unwrap();
        h.clear_toast();
        let step = h.begin_apply();
        let (outcome, _) = complete(&mut h, step, RunOutcome::Cancelled);
        assert_eq!(outcome, Outcome::Cancelled);
        assert!(h.dirty());
        assert!(h.toast().is_none());
        assert!(!h.applying);
    }

    #[test]
    fn a_failed_write_shows_the_error() {
        let (mut h, _) = hosts(BASE, vec![]);
        let step = h.begin_apply_raw(&format!("{BASE}127.0.0.1\tapi.test\n"), &h.serialize());
        complete(&mut h, step, RunOutcome::Denied(-60005));
        let toast = h.toast().unwrap();
        assert_eq!(toast.kind, ToastKind::Error);
        assert_eq!(
            toast.message,
            "Không ghi được /etc/hosts: quyền quản trị bị từ chối (mã -60005)"
        );
    }

    #[test]
    fn nothing_changes_while_a_write_is_out() {
        let (mut h, _) = hosts(BASE, vec![]);
        h.add_entry(input("127.0.0.1", "a.test")).unwrap();
        let job = job(h.begin_apply());
        let id = h.entries[0].id;
        assert_eq!(h.add_entry(input("127.0.0.1", "b.test")).unwrap_err(), BUSY);
        assert_eq!(h.toggle_entry(id).unwrap_err(), BUSY);
        assert_eq!(h.undo().unwrap_err(), BUSY);
        assert_eq!(h.reload(Ok(BASE.into())).unwrap_err(), BUSY);
        assert!(matches!(h.begin_apply(), Step::Done(Outcome::Failed(_))));
        complete(&mut h, Step::Write(job), RunOutcome::Ok);
        assert!(!h.dirty());
        assert!(h.add_entry(input("127.0.0.1", "b.test")).is_ok());
    }

    #[test]
    fn an_unread_file_is_never_overwritten() {
        let store = MemoryStore::default();
        let mut h = Hosts::new(Box::new(store));
        h.load_failed("stream did not contain valid UTF-8");
        h.add_entry(input("127.0.0.1", "a.test")).unwrap();
        let step = h.begin_apply();
        let (outcome, content) = complete(&mut h, step, RunOutcome::Ok);
        assert!(matches!(outcome, Outcome::Failed(_)));
        assert!(content.is_none());
        assert!(h.snapshot().load_error.is_some());
        h.reload(Ok(BASE.into())).unwrap();
        assert!(h.snapshot().load_error.is_none());
    }

    #[test]
    fn unchanged_lines_keep_their_ids_through_a_raw_edit() {
        let (mut h, _) = hosts(FIXTURE, vec![]);
        let ids: Vec<Uuid> = h.entries.iter().map(|e| e.id).collect();
        h.set_raw_draft(
            Some(h.serialize().replace("api.prod", "api.production")),
            &h.serialize(),
        );
        h.toggle_entry(ids[0]).unwrap();
        assert_eq!(h.entries[0].id, ids[0]);
        assert_eq!(h.entries[1].id, ids[1]);
        assert_ne!(h.entries[2].id, ids[2], "the edited line is a new entry");
        let step = h.begin_apply_raw(&h.serialize(), &h.serialize());
        complete(&mut h, step, RunOutcome::Ok);
        assert_eq!(h.entries[0].id, ids[0], "a raw write keeps them too");
    }

    #[test]
    fn an_outside_change_must_be_reloaded_before_writing() {
        let (mut h, _) = hosts(BASE, vec![]);
        h.add_entry(input("127.0.0.1", "a.test")).unwrap();
        h.disk_changed(Some("127.0.0.1\tdocker.internal\n"));
        let step = h.begin_apply();
        assert!(matches!(step, Step::Done(Outcome::Failed(_))));
        assert_eq!(h.toast().unwrap().kind, ToastKind::Error);
    }

    #[test]
    fn its_own_write_is_not_an_outside_change() {
        let (mut h, _) = hosts(BASE, vec![]);
        let step = h.begin_apply_raw(&format!("{BASE}127.0.0.1\tapi.test\n"), &h.serialize());
        complete(&mut h, step, RunOutcome::Ok);
        h.disk_changed(Some(&format!("{BASE}127.0.0.1\tapi.test\n")));
        assert!(!h.snapshot().external_change);
        h.disk_changed(Some("10.0.0.1\tother.test\n"));
        assert!(h.snapshot().external_change);
        h.load("10.0.0.1\tother.test\n");
        assert!(!h.snapshot().external_change);
        h.disk_changed(None);
        assert!(
            h.snapshot().external_change,
            "a deleted file is an outside change"
        );
    }

    #[test]
    fn only_the_latest_write_keeps_its_verification() {
        let (mut h, _) = hosts(BASE, vec![]);
        let first = job(h.begin_apply_raw(&format!("{BASE}127.0.0.1\ta.test\n"), &h.serialize()));
        let first_seq = first.seq;
        complete(&mut h, Step::Write(first), RunOutcome::Ok);
        let second = job(h.begin_apply_raw(&format!("{BASE}127.0.0.1\tb.test\n"), &h.serialize()));
        h.set_verification(first_seq, verify::run(&[], |_| vec![]));
        assert!(h.snapshot().verification.is_none());
        let second_seq = second.seq;
        complete(&mut h, Step::Write(second), RunOutcome::Ok);
        h.set_verification(second_seq, verify::run(&[], |_| vec![]));
        assert!(h.snapshot().verification.is_some());
    }

    #[test]
    fn toasts_are_numbered() {
        let (mut h, _) = hosts(BASE, vec![]);
        h.add_entry(input("127.0.0.1", "a.test")).unwrap();
        let first = h.toast().unwrap().id;
        h.add_entry(input("127.0.0.1", "a.test")).unwrap();
        assert_eq!(h.toast().unwrap().id, first + 1);
    }
}
