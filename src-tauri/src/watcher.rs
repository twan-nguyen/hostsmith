//! Watches one file for changes made outside Hosven.
//!
//! Editors save atomically: they write a sibling file and rename it over the
//! original, which ends a watch held on the old inode. Watching the parent
//! directory and filtering by file name survives that, so the watch never has
//! to be re-armed.

use std::ffi::OsString;
use std::path::Path;
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebounceEventResult, Debouncer, new_debouncer};

/// Callers compare the file's content with what they last read or wrote, so
/// Hosven's own saves need no special handling here.
pub struct FileWatcher {
    // Dropping the debouncer stops the watch.
    _debouncer: Debouncer<RecommendedWatcher>,
}

impl FileWatcher {
    /// Calls `on_change` on a background thread once changes to `path` have
    /// been quiet for `debounce`. A write, a replacement, a creation and a
    /// deletion all count.
    pub fn new(
        path: &Path,
        debounce: Duration,
        on_change: impl Fn() + Send + 'static,
    ) -> Result<Self, String> {
        let name: OsString = path.file_name().ok_or("Đường dẫn không hợp lệ")?.into();
        // FSEvents reports resolved paths, so `/etc` arrives as `/private/etc`.
        let dir = path
            .parent()
            .ok_or("Đường dẫn không hợp lệ")?
            .canonicalize()
            .map_err(|e| e.to_string())?;
        let mut debouncer = new_debouncer(debounce, move |result: DebounceEventResult| {
            let Ok(events) = result else { return };
            if events
                .iter()
                .any(|event| event.path.file_name() == Some(name.as_os_str()))
            {
                on_change();
            }
        })
        .map_err(|e| e.to_string())?;
        debouncer
            .watcher()
            .watch(&dir, RecursiveMode::NonRecursive)
            .map_err(|e| e.to_string())?;

        Ok(Self {
            _debouncer: debouncer,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::mpsc::{Receiver, channel};

    const DEBOUNCE: Duration = Duration::from_millis(200);
    /// FSEvents batches events, so a test waits well past the debounce.
    const WAIT: Duration = Duration::from_secs(3);
    const QUIET: Duration = Duration::from_millis(1200);

    fn setup() -> (PathBuf, FileWatcher, Receiver<()>) {
        let dir = std::env::temp_dir().join(format!("hosven-watch-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join(".env");
        fs::write(&file, "A=1\n").unwrap();
        let (tx, rx) = channel();
        let watcher = FileWatcher::new(&file, DEBOUNCE, move || {
            let _ = tx.send(());
        })
        .unwrap();
        // FSEvents can still report the file's creation, which happened just
        // before the stream started; the owners compare content, so it is
        // harmless, but a test must start from silence.
        drain(&rx);
        (file, watcher, rx)
    }

    /// How many callbacks arrive before the watcher goes quiet.
    fn drain(rx: &Receiver<()>) -> usize {
        let mut seen = 0;
        while rx.recv_timeout(QUIET).is_ok() {
            seen += 1;
        }
        seen
    }

    #[test]
    fn detects_a_modification() {
        let (file, _watcher, rx) = setup();
        fs::write(&file, "A=2\n").unwrap();
        assert_eq!(rx.recv_timeout(WAIT), Ok(()));
    }

    #[test]
    fn debounces_rapid_writes() {
        let (file, _watcher, rx) = setup();
        for value in 0..10 {
            fs::write(&file, format!("A={value}\n")).unwrap();
        }
        let seen = drain(&rx);
        assert!(seen > 0);
        assert!(seen <= 2, "ten writes arrived as {seen} callbacks");
    }

    #[test]
    fn follows_atomic_saves() {
        let (file, _watcher, rx) = setup();
        for value in 0..3 {
            let tmp = file.with_file_name(".env.swp");
            fs::write(&tmp, format!("A={value}\n")).unwrap();
            fs::rename(&tmp, &file).unwrap();
            assert_eq!(rx.recv_timeout(WAIT), Ok(()), "save {value}");
            drain(&rx);
        }
    }

    #[test]
    fn reports_a_deleted_file() {
        let (file, _watcher, rx) = setup();
        fs::remove_file(&file).unwrap();
        assert_eq!(rx.recv_timeout(WAIT), Ok(()));
    }

    #[test]
    fn ignores_other_files_in_the_directory() {
        let (file, _watcher, rx) = setup();
        fs::write(file.with_file_name(".env.example"), "B=1\n").unwrap();
        assert_eq!(drain(&rx), 0);
    }
}
