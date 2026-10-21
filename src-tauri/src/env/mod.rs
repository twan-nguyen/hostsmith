//! Tauri commands over the env model. Every change sends the whole snapshot
//! as `env-changed`, so the window and the menu bar panel never drift.

mod files;
mod missing;
mod parse;
mod profiles;
mod state;
mod store;

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::watcher::FileWatcher;
pub use files::Outcome;
pub use state::{EntryInput, Env, Snapshot};
use store::EnvStore;

const EVENT: &str = "env-changed";
const DEBOUNCE: Duration = Duration::from_millis(300);

/// Called with the model after the watcher changed it.
type Notify = Arc<dyn Fn(&Env) + Send + Sync>;

/// The env model and the watch on its selected file.
pub struct EnvState {
    env: Arc<Mutex<Env>>,
    watch: Mutex<Option<Watched>>,
}

struct Watched {
    repo_id: Uuid,
    relative_path: String,
    // Dropping the watcher stops the watch.
    _watcher: FileWatcher,
}

impl EnvState {
    /// The model with the user's repos, from `env.json`.
    pub fn load() -> Self {
        Self::new(Env::new(EnvStore::default_location()))
    }

    fn new(env: Env) -> Self {
        Self {
            env: Arc::new(Mutex::new(env)),
            watch: Mutex::new(None),
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, Env> {
        lock(&self.env)
    }

    /// Points the watch at the selected file, or stops it when nothing is
    /// selected. Called without the model's lock held: a watcher event that
    /// is waiting for that lock would otherwise never let the old watch stop.
    fn sync_watch(&self, notify: Notify) {
        let target = self.lock().watch_target();
        let mut watch = self.watch.lock().unwrap_or_else(|p| p.into_inner());
        let current = watch
            .as_ref()
            .map(|w| (w.repo_id, w.relative_path.as_str()));
        if current == target.as_ref().map(|(id, rel, _)| (*id, rel.as_str())) {
            return;
        }
        *watch = None;
        let Some((repo_id, relative_path, path)) = target else {
            return;
        };
        let env = self.env.clone();
        let watched = relative_path.clone();
        // Hosven's own writes leave the content it recorded, which the model
        // recognises and ignores.
        let watcher = FileWatcher::new(&path, DEBOUNCE, move |_| {
            let mut env = lock(&env);
            if env.refresh_watched(repo_id, &watched) {
                notify(&env);
            }
        });
        match watcher {
            Ok(watcher) => {
                *watch = Some(Watched {
                    repo_id,
                    relative_path,
                    _watcher: watcher,
                });
            }
            // The repo's folder is gone; the pane already says so, and a
            // refresh on focus covers the file coming back.
            Err(error) => eprintln!("hosven: not watching {}: {error}", path.display()),
        }
    }
}

fn lock(env: &Mutex<Env>) -> MutexGuard<'_, Env> {
    env.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn emit(app: &AppHandle, env: &Env) {
    let _ = app.emit(EVENT, env.snapshot());
}

fn mutate<T>(app: &AppHandle, state: &EnvState, change: impl FnOnce(&mut Env) -> T) -> T {
    let result = {
        let mut env = state.lock();
        let result = change(&mut env);
        emit(app, &env);
        result
    };
    let app = app.clone();
    state.sync_watch(Arc::new(move |env: &Env| emit(&app, env)));
    result
}

#[tauri::command]
pub fn env_snapshot(state: State<'_, EnvState>) -> Snapshot {
    state.lock().snapshot()
}

#[tauri::command]
pub fn env_add_repo(
    app: AppHandle,
    state: State<'_, EnvState>,
    path: String,
) -> Result<(), String> {
    mutate(&app, &state, |e| e.add_repo(&path))
}

#[tauri::command]
pub fn env_remove_repo(
    app: AppHandle,
    state: State<'_, EnvState>,
    repo_id: Uuid,
) -> Result<(), String> {
    mutate(&app, &state, |e| e.remove_repo(repo_id))
}

#[tauri::command]
pub fn env_rename_repo(
    app: AppHandle,
    state: State<'_, EnvState>,
    repo_id: Uuid,
    name: String,
) -> Result<(), String> {
    mutate(&app, &state, |e| e.rename_repo(repo_id, &name))
}

#[tauri::command]
pub fn env_select(
    app: AppHandle,
    state: State<'_, EnvState>,
    repo_id: Uuid,
    relative_path: Option<String>,
) -> Result<(), String> {
    mutate(&app, &state, |e| e.select(repo_id, relative_path))
}

#[tauri::command]
pub fn env_discover(
    app: AppHandle,
    state: State<'_, EnvState>,
    repo_id: Uuid,
) -> Result<(), String> {
    mutate(&app, &state, |e| e.discover(repo_id))
}

#[tauri::command]
pub fn env_add_entry(
    app: AppHandle,
    state: State<'_, EnvState>,
    input: EntryInput,
) -> Result<(), String> {
    mutate(&app, &state, |e| e.add_entry(input))
}

#[tauri::command]
pub fn env_update_entry(
    app: AppHandle,
    state: State<'_, EnvState>,
    id: Uuid,
    input: EntryInput,
) -> Result<(), String> {
    mutate(&app, &state, |e| e.update_entry(id, input))
}

#[tauri::command]
pub fn env_duplicate_entry(
    app: AppHandle,
    state: State<'_, EnvState>,
    id: Uuid,
) -> Result<(), String> {
    mutate(&app, &state, |e| e.duplicate_entry(id))
}

#[tauri::command]
pub fn env_toggle_entry(
    app: AppHandle,
    state: State<'_, EnvState>,
    id: Uuid,
) -> Result<(), String> {
    mutate(&app, &state, |e| e.toggle_entry(id))
}

#[tauri::command]
pub fn env_delete_entry(
    app: AppHandle,
    state: State<'_, EnvState>,
    id: Uuid,
) -> Result<(), String> {
    mutate(&app, &state, |e| e.delete_entry(id))
}

#[tauri::command]
pub fn env_select_variant(
    app: AppHandle,
    state: State<'_, EnvState>,
    key: String,
    id: Uuid,
) -> Result<(), String> {
    mutate(&app, &state, |e| e.select_variant(&key, id))
}

#[tauri::command]
pub fn env_add_missing(app: AppHandle, state: State<'_, EnvState>) -> Result<usize, String> {
    mutate(&app, &state, Env::add_missing)
}

#[tauri::command]
pub fn env_discard(app: AppHandle, state: State<'_, EnvState>) -> Result<(), String> {
    mutate(&app, &state, Env::discard)
}

#[tauri::command]
pub fn env_set_raw_draft(
    app: AppHandle,
    state: State<'_, EnvState>,
    repo_id: Uuid,
    relative_path: String,
    text: Option<String>,
) {
    mutate(&app, &state, |e| {
        e.set_raw_draft(repo_id, &relative_path, text)
    });
}

#[tauri::command]
pub fn env_merge_raw_draft(app: AppHandle, state: State<'_, EnvState>) -> Result<(), String> {
    mutate(&app, &state, Env::merge_raw_draft)
}

#[tauri::command]
pub fn env_apply(
    app: AppHandle,
    state: State<'_, EnvState>,
    force: bool,
) -> Result<Outcome, String> {
    mutate(&app, &state, |e| e.apply(force))
}

#[tauri::command]
pub fn env_apply_raw(
    app: AppHandle,
    state: State<'_, EnvState>,
    text: String,
    force: bool,
) -> Result<Outcome, String> {
    mutate(&app, &state, |e| e.apply_raw(&text, force))
}

#[tauri::command]
pub fn env_reload_from_disk(app: AppHandle, state: State<'_, EnvState>) -> Result<(), String> {
    mutate(&app, &state, Env::reload_from_disk)
}

/// For the window to call when it gains focus, a backstop for watcher events
/// missed while Hosven was in the background.
#[tauri::command]
pub fn env_refresh_from_disk(app: AppHandle, state: State<'_, EnvState>) {
    mutate(&app, &state, Env::refresh_from_disk);
}

#[tauri::command]
pub fn env_dismiss_conflict(app: AppHandle, state: State<'_, EnvState>) {
    mutate(&app, &state, Env::dismiss_conflict);
}

#[tauri::command]
pub fn env_profile_save(
    app: AppHandle,
    state: State<'_, EnvState>,
    repo_id: Uuid,
    name: String,
) -> Result<(), String> {
    mutate(&app, &state, |e| e.save_profile(repo_id, &name))
}

#[tauri::command]
pub fn env_profile_apply(
    app: AppHandle,
    state: State<'_, EnvState>,
    repo_id: Uuid,
    profile_id: Uuid,
) -> Result<(), String> {
    mutate(&app, &state, |e| e.apply_profile(repo_id, profile_id))
}

#[tauri::command]
pub fn env_profile_rename(
    app: AppHandle,
    state: State<'_, EnvState>,
    repo_id: Uuid,
    profile_id: Uuid,
    name: String,
) -> Result<(), String> {
    mutate(&app, &state, |e| {
        e.rename_profile(repo_id, profile_id, &name)
    })
}

#[tauri::command]
pub fn env_profile_delete(
    app: AppHandle,
    state: State<'_, EnvState>,
    repo_id: Uuid,
    profile_id: Uuid,
) -> Result<(), String> {
    mutate(&app, &state, |e| e.delete_profile(repo_id, profile_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Instant;

    use state::fixture::Fixture;

    fn wait_until(state: &EnvState, condition: impl Fn(&Env) -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if condition(&state.lock()) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        condition(&state.lock())
    }

    fn foo(env: &Env) -> String {
        env.snapshot()
            .entries
            .iter()
            .find(|e| e.key == "FOO")
            .map(|e| e.value.clone())
            .unwrap_or_default()
    }

    /// Moves the fixture's model into the shared state the watcher works on.
    /// The fixture keeps an empty model that never saves.
    fn into_state(f: &mut Fixture) -> EnvState {
        let empty = Env::new(EnvStore::new(f.dir.join("unused.json"), None));
        EnvState::new(std::mem::replace(&mut f.env, empty))
    }

    /// Saves the way editors do: write a temporary file, then rename it over
    /// the original.
    fn save_atomically(f: &Fixture, content: &str) {
        let tmp = f.path(&format!(".env.tmp-{}", Uuid::new_v4()));
        fs::write(&tmp, content).unwrap();
        fs::rename(tmp, f.path(".env")).unwrap();
    }

    #[test]
    fn the_watcher_follows_atomic_saves_across_more_than_one_save() {
        let mut f = Fixture::standard();
        let state = into_state(&mut f);
        let notified = Arc::new(AtomicUsize::new(0));
        let count = notified.clone();
        state.sync_watch(Arc::new(move |_| {
            count.fetch_add(1, Ordering::SeqCst);
        }));
        assert!(state.watch.lock().unwrap().is_some());
        std::thread::sleep(Duration::from_millis(300));

        save_atomically(&f, "FOO=first\nBAR=2\n");
        assert!(wait_until(&state, |e| foo(e) == "first"));

        save_atomically(&f, "FOO=second\nBAR=2\n");
        assert!(wait_until(&state, |e| foo(e) == "second"));
        assert!(notified.load(Ordering::SeqCst) >= 2);
    }

    #[test]
    fn the_watcher_marks_a_dirty_file_as_a_conflict() {
        let mut f = Fixture::standard();
        f.edit("mine");
        let state = into_state(&mut f);
        state.sync_watch(Arc::new(|_| {}));
        std::thread::sleep(Duration::from_millis(300));

        save_atomically(&f, "FOO=theirs\nBAR=2\n");
        assert!(wait_until(&state, |e| e.snapshot().conflict
            == Some(files::Conflict::Changed)));
        assert_eq!(foo(&state.lock()), "mine");
    }

    #[test]
    fn the_watch_follows_the_selection() {
        let mut f = Fixture::new(&[(".env", "FOO=1\n"), (".env.local", "L=1\n")]);
        let repo = f.repo;
        let state = into_state(&mut f);
        state.sync_watch(Arc::new(|_| {}));
        let watched = || {
            state
                .watch
                .lock()
                .unwrap()
                .as_ref()
                .map(|w| w.relative_path.clone())
        };
        assert_eq!(watched().as_deref(), Some(".env"));

        state
            .lock()
            .select(repo, Some(".env.local".into()))
            .unwrap();
        state.sync_watch(Arc::new(|_| {}));
        assert_eq!(watched().as_deref(), Some(".env.local"));

        state.lock().remove_repo(repo).unwrap();
        state.sync_watch(Arc::new(|_| {}));
        assert_eq!(watched(), None);
    }
}
