//! Small helpers the domain modules share.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

/// Locks `mutex` even when a panic poisoned it. Every value guarded in the
/// app stays usable after a panic halfway through a change, and refusing the
/// lock would leave the app unable to do anything until it restarts.
pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Runs blocking work, such as a child process or file I/O, off the async
/// runtime's threads, so a command never stalls the others.
pub async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| e.to_string())?
}

/// How many lines differ between `before` and `after`, ignoring order and
/// counting a repeated line each time it occurs. A changed line counts once,
/// not as one removed and one added.
pub fn changed_lines(before: &str, after: &str) -> usize {
    let mut balance: HashMap<&str, isize> = HashMap::new();
    for line in before.split('\n') {
        *balance.entry(line).or_default() -= 1;
    }
    for line in after.split('\n') {
        *balance.entry(line).or_default() += 1;
    }
    let added: isize = balance.values().filter(|n| **n > 0).sum();
    let removed: isize = -balance.values().filter(|n| **n < 0).sum::<isize>();
    added.max(removed) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_changed_lines_including_repeats() {
        assert_eq!(changed_lines("a\nb\n", "a\nb\n"), 0);
        assert_eq!(changed_lines("a\nb\n", "b\na\n"), 0, "order does not count");
        assert_eq!(changed_lines("a\nb\n", "a\nc\n"), 1);
        assert_eq!(changed_lines("a\n", "a\na\n"), 1);
        assert_eq!(changed_lines("a\nb\nc\n", "a\n"), 2);
    }
}
