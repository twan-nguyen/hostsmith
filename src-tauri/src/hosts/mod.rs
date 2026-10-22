//! Tauri commands over the hosts model. Every change sends the whole snapshot
//! as `hosts-changed`, so the main window and the menu bar panel never drift.

mod history;
mod model;
mod parse;
mod state;
mod switch;
mod verify;

use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

use crate::hosts_writer;
use crate::platform::{self, HOSTS_PATH, RunOutcome, SharedRunner};
use crate::profiles::{FileProfileStore, Profile, ProfileColor};
use crate::util::lock;
use crate::watcher::FileWatcher;
use model::ToastKind;
pub use state::{EntryInput, Hosts, Outcome, Snapshot, Step};
pub use switch::{ActiveState, SwitchPolicy};

const EVENT: &str = "hosts-changed";

pub struct HostsState(Mutex<Hosts>);

impl HostsState {
    /// The model with the user's profiles, loaded from the hosts file.
    pub fn load() -> Self {
        let mut hosts = Hosts::new(Box::new(FileProfileStore::default_location()));
        match read_hosts() {
            Ok(content) => hosts.load(&content),
            Err(error) => hosts.load_failed(&error),
        }
        Self(Mutex::new(hosts))
    }

    pub fn lock(&self) -> MutexGuard<'_, Hosts> {
        lock(&self.0)
    }
}

fn read_hosts() -> Result<String, String> {
    std::fs::read_to_string(HOSTS_PATH).map_err(|e| e.to_string())
}

/// Holds the watch on the hosts file for the app's lifetime.
pub struct HostsWatch {
    _watcher: FileWatcher,
}

/// Flags edits other tools make to the hosts file. HostSmith's own writes leave
/// the content it wrote, which the model recognises and ignores.
pub fn watch(app: &AppHandle) -> Result<HostsWatch, String> {
    let handle = app.clone();
    let watcher = FileWatcher::new(
        Path::new(HOSTS_PATH),
        Duration::from_millis(400),
        move || {
            let on_disk = read_hosts().ok();
            let state = handle.state::<HostsState>();
            let mut hosts = state.lock();
            hosts.disk_changed(on_disk.as_deref());
            emit(&handle, &hosts);
        },
    )?;
    Ok(HostsWatch { _watcher: watcher })
}

fn emit(app: &AppHandle, hosts: &Hosts) {
    let _ = app.emit(EVENT, hosts.snapshot());
    crate::menu::sync(app, hosts);
    crate::tray::sync(app, hosts);
}

pub(crate) fn mutate<T>(
    app: &AppHandle,
    state: &HostsState,
    change: impl FnOnce(&mut Hosts) -> T,
) -> T {
    let mut hosts = state.lock();
    let result = change(&mut hosts);
    emit(app, &hosts);
    result
}

/// Runs a prepared write off the model's lock, since the password prompt can
/// stay open for a while, then checks the hostnames it changed.
pub(crate) async fn run(app: AppHandle, step: Step) -> Outcome {
    let state = app.state::<HostsState>();
    let job = match step {
        Step::Done(outcome) => {
            emit(&app, &state.lock());
            return outcome;
        }
        Step::Write(job) => job,
    };
    emit(&app, &state.lock());

    let runner = app.state::<SharedRunner>().inner().clone();
    let content = job.content.clone();
    let changed = job.changed.clone();
    let seq = job.seq;
    let written = run_as_root(move || hosts_writer::write(runner.as_ref(), &content)).await;

    let outcome = mutate(&app, &state, |h| h.finish_write(job, written));
    if outcome == Outcome::Written && !changed.is_empty() {
        let checked = tauri::async_runtime::spawn_blocking(move || {
            verify::run(&changed, platform::resolve_host)
        })
        .await;
        if let Ok(verification) = checked {
            mutate(&app, &state, |h| h.set_verification(seq, verification));
        }
    }
    outcome
}

/// Runs a root script off the async runtime's threads, since the password
/// prompt blocks until the user answers it.
async fn run_as_root(work: impl FnOnce() -> RunOutcome + Send + 'static) -> RunOutcome {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .unwrap_or_else(|error| RunOutcome::Failed {
            exit: -1,
            stderr: error.to_string(),
        })
}

#[tauri::command]
pub fn hosts_snapshot(state: State<'_, HostsState>) -> Snapshot {
    state.lock().snapshot()
}

#[tauri::command]
pub fn hosts_reload(app: AppHandle, state: State<'_, HostsState>) -> Result<(), String> {
    mutate(&app, &state, |h| h.reload(read_hosts()))
}

#[tauri::command]
pub fn hosts_add_entry(
    app: AppHandle,
    state: State<'_, HostsState>,
    input: EntryInput,
) -> Result<(), String> {
    mutate(&app, &state, |h| h.add_entry(input))
}

#[tauri::command]
pub fn hosts_update_entry(
    app: AppHandle,
    state: State<'_, HostsState>,
    id: Uuid,
    input: EntryInput,
) -> Result<(), String> {
    mutate(&app, &state, |h| h.update_entry(id, input))
}

#[tauri::command]
pub fn hosts_duplicate_entry(
    app: AppHandle,
    state: State<'_, HostsState>,
    id: Uuid,
) -> Result<(), String> {
    mutate(&app, &state, |h| h.duplicate_entry(id))
}

#[tauri::command]
pub fn hosts_toggle_entry(
    app: AppHandle,
    state: State<'_, HostsState>,
    id: Uuid,
) -> Result<(), String> {
    mutate(&app, &state, |h| h.toggle_entry(id))
}

#[tauri::command]
pub fn hosts_delete_entry(
    app: AppHandle,
    state: State<'_, HostsState>,
    id: Uuid,
) -> Result<(), String> {
    mutate(&app, &state, |h| h.delete_entry(id))
}

#[tauri::command]
pub fn hosts_set_tag_enabled(
    app: AppHandle,
    state: State<'_, HostsState>,
    name: String,
    enabled: bool,
) -> Result<(), String> {
    mutate(&app, &state, |h| h.set_tag_enabled(&name, enabled))
}

#[tauri::command]
pub fn hosts_undo(app: AppHandle, state: State<'_, HostsState>) -> Result<(), String> {
    mutate(&app, &state, Hosts::undo)
}

#[tauri::command]
pub fn hosts_redo(app: AppHandle, state: State<'_, HostsState>) -> Result<(), String> {
    mutate(&app, &state, Hosts::redo)
}

#[tauri::command]
pub fn hosts_discard(app: AppHandle, state: State<'_, HostsState>) -> Result<(), String> {
    mutate(&app, &state, Hosts::discard)
}

#[tauri::command]
pub fn hosts_set_raw_draft(
    app: AppHandle,
    state: State<'_, HostsState>,
    text: Option<String>,
    base: String,
) {
    // The window keeps its own draft in step, so only a change to what the
    // other views show (the dirty mark and the pending count) is sent.
    let mut hosts = state.lock();
    let before = hosts.pending_state();
    hosts.set_raw_draft(text, &base);
    if hosts.pending_state() != before {
        emit(&app, &hosts);
    }
}

#[tauri::command]
pub async fn hosts_apply(app: AppHandle) -> Outcome {
    let step = app.state::<HostsState>().lock().begin_apply();
    run(app, step).await
}

#[tauri::command]
pub async fn hosts_apply_raw(app: AppHandle, text: String, base: String) -> Outcome {
    let step = app
        .state::<HostsState>()
        .lock()
        .begin_apply_raw(&text, &base);
    run(app, step).await
}

#[tauri::command]
pub async fn hosts_switch_environment(
    app: AppHandle,
    target: Option<Uuid>,
    policy: SwitchPolicy,
) -> Outcome {
    let step = app
        .state::<HostsState>()
        .lock()
        .switch_environment(target, policy);
    run(app, step).await
}

#[tauri::command]
pub async fn hosts_confirm_pending_switch(app: AppHandle) -> Outcome {
    let step = app.state::<HostsState>().lock().confirm_pending_switch();
    run(app, step).await
}

#[tauri::command]
pub fn hosts_cancel_pending_switch(app: AppHandle, state: State<'_, HostsState>) {
    mutate(&app, &state, Hosts::cancel_pending_switch);
}

/// Copies the hosts file beside itself as root. Refused while a write is out,
/// so the copy is never of a half-replaced file.
#[tauri::command]
pub async fn hosts_backup(app: AppHandle) -> Result<(), String> {
    let state = app.state::<HostsState>();
    state.lock().check_idle()?;
    let runner = app.state::<SharedRunner>().inner().clone();
    let path = hosts_writer::backup_path(jiff::Timestamp::now());
    let script = hosts_writer::backup_script(&path);
    let ran = run_as_root(move || runner.run(&script)).await;
    mutate(&app, &state, |h| match ran {
        RunOutcome::Ok => h.show_toast(
            format!("Backed up /etc/hosts to {}", path.display()),
            ToastKind::Success,
        ),
        RunOutcome::Cancelled => {}
        failed => h.show_toast(
            format!(
                "Could not back up /etc/hosts: {}",
                failed.failure().unwrap_or_default()
            ),
            ToastKind::Error,
        ),
    });
    Ok(())
}

#[tauri::command]
pub fn hosts_dismiss_verification(app: AppHandle, state: State<'_, HostsState>) {
    mutate(&app, &state, Hosts::dismiss_verification);
}

#[tauri::command]
pub fn profiles_add(
    app: AppHandle,
    state: State<'_, HostsState>,
    name: String,
    color: ProfileColor,
    is_exclusive: bool,
) -> Result<Profile, String> {
    mutate(&app, &state, |h| h.add_profile(&name, color, is_exclusive))
}

#[tauri::command]
pub fn profiles_rename(
    app: AppHandle,
    state: State<'_, HostsState>,
    id: Uuid,
    name: String,
) -> Result<(), String> {
    mutate(&app, &state, |h| h.rename_profile(id, &name))
}

#[tauri::command]
pub fn profiles_remove(app: AppHandle, state: State<'_, HostsState>, id: Uuid) {
    mutate(&app, &state, |h| h.remove_profile(id));
}

#[tauri::command]
pub fn profiles_set_exclusive(
    app: AppHandle,
    state: State<'_, HostsState>,
    id: Uuid,
    is_exclusive: bool,
) -> Result<(), String> {
    mutate(&app, &state, |h| h.set_profile_exclusive(id, is_exclusive))
}

#[tauri::command]
pub fn profiles_set_color(
    app: AppHandle,
    state: State<'_, HostsState>,
    id: Uuid,
    color: ProfileColor,
) -> Result<(), String> {
    mutate(&app, &state, |h| h.set_profile_color(id, color))
}
