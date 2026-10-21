//! An app opened from Finder or the Dock gets launchd's bare PATH
//! (`/usr/bin:/bin:/usr/sbin:/sbin`), so tools installed by Homebrew or Docker
//! Desktop, such as `docker`, are not found. This borrows the PATH the user's
//! own shell sets up.

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Wraps the PATH in the shell's output, which may also hold whatever the
/// user's startup files print.
const MARKER: &str = "__HOSVEN_PATH__";
/// A shell that takes longer than this is left alone, and the app keeps its PATH.
const TIMEOUT: Duration = Duration::from_secs(3);

/// Takes on the login shell's PATH. Call it before any other thread starts,
/// since it changes the process environment.
pub fn adopt() {
    if !cfg!(target_os = "macos") {
        return;
    }
    if let Some(path) = shell_path() {
        // SAFETY: this runs first thing in `run`, while the app is still single-threaded.
        unsafe { std::env::set_var("PATH", path) };
    }
}

/// The PATH of an interactive login shell, which reads both the profile and
/// the rc files, where tools like Homebrew, nvm and gcloud add themselves.
fn shell_path() -> Option<String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    let mut child = Command::new(shell)
        .args(["-ilc", &format!("printf '{MARKER}%s{MARKER}' \"$PATH\"")])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    // Polled rather than read on a thread, so no thread exists when PATH is set.
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() < TIMEOUT => {
                std::thread::sleep(Duration::from_millis(20))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }

    let mut out = String::new();
    child.stdout.take()?.read_to_string(&mut out).ok()?;
    between_markers(&out)
}

fn between_markers(out: &str) -> Option<String> {
    let (_, rest) = out.split_once(MARKER)?;
    let (path, _) = rest.split_once(MARKER)?;
    let path = path.trim();
    (!path.is_empty()).then(|| path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_what_startup_files_print() {
        let out = format!("Welcome!\n{MARKER}/opt/homebrew/bin:/usr/bin{MARKER}bye\n");
        assert_eq!(
            between_markers(&out).as_deref(),
            Some("/opt/homebrew/bin:/usr/bin")
        );
        assert_eq!(between_markers("no markers here"), None);
        assert_eq!(between_markers(&format!("{MARKER}{MARKER}")), None);
    }
}
