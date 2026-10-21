//! Everything that differs between operating systems sits behind this module,
//! so Linux (pkexec) and Windows (UAC) can be added beside macOS later.

use std::sync::Arc;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::{
    MacPrivilegedRunner as SystemRunner, process_identity, process_path, resolve_host,
};

/// The hosts file every resolver on the system reads.
#[cfg(unix)]
pub const HOSTS_PATH: &str = "/etc/hosts";
#[cfg(windows)]
pub const HOSTS_PATH: &str = r"C:\Windows\System32\drivers\etc\hosts";

/// How a script run with administrator rights ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunOutcome {
    Ok,
    /// The user dismissed the password prompt. Nothing ran.
    Cancelled,
    /// The system refused the rights, with its status code.
    Denied(i32),
    /// The script ran and exited with a non-zero status. `stderr` holds the
    /// script's combined output.
    Failed {
        exit: i32,
        stderr: String,
    },
}

impl RunOutcome {
    /// Why the run did not do its work, in Vietnamese. `None` when it did, or
    /// when the prompt was dismissed, which the user knows about already.
    pub fn failure(&self) -> Option<String> {
        match self {
            Self::Ok | Self::Cancelled => None,
            Self::Denied(code) => Some(format!("quyền quản trị bị từ chối (mã {code})")),
            Self::Failed { exit, stderr } => Some(match stderr.trim() {
                "" => format!("lệnh thoát với mã {exit}"),
                output => output.to_string(),
            }),
        }
    }
}

/// Runs a shell script as root, asking for the administrator password when the
/// cached rights have expired.
pub trait PrivilegedRunner: Send + Sync {
    /// Runs `script` with nothing on its standard input.
    fn run(&self, script: &str) -> RunOutcome {
        self.run_with_input(script, &[])
    }

    /// Runs `script` with `input` on its standard input, followed by end of
    /// file, so data never has to pass through a file or the command line.
    fn run_with_input(&self, script: &str, input: &[u8]) -> RunOutcome;

    /// Forgets cached rights, so the next run asks again.
    fn release(&self) {}
}

/// The one runner the app shares, so a single password prompt covers hosts
/// writes and backups.
pub type SharedRunner = Arc<dyn PrivilegedRunner>;

/// Tells one process apart from a later one that reuses its pid: a pid alone
/// can name a different program by the time a signal is sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessIdentity {
    pub uid: u32,
    /// Start time, in seconds and microseconds since the Unix epoch.
    pub started: (u64, u64),
}

/// How writing the hosts file ended, as the hosts model sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteOutcome {
    Written,
    /// The password prompt was dismissed; pending edits stay pending.
    Cancelled,
    /// A message for the user, already in Vietnamese.
    Failed(String),
}

/// Replaces the hosts file and flushes the resolver caches.
pub trait HostsWriter: Send + Sync {
    fn write(&self, content: &str) -> WriteOutcome;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failures_read_as_a_reason() {
        assert_eq!(RunOutcome::Ok.failure(), None);
        assert_eq!(RunOutcome::Cancelled.failure(), None);
        assert_eq!(
            RunOutcome::Denied(-60005).failure().as_deref(),
            Some("quyền quản trị bị từ chối (mã -60005)")
        );
        let failed = |stderr: &str| RunOutcome::Failed {
            exit: 1,
            stderr: stderr.into(),
        };
        assert_eq!(
            failed("  cp: /etc/hosts: Operation not permitted\n")
                .failure()
                .as_deref(),
            Some("cp: /etc/hosts: Operation not permitted")
        );
        assert_eq!(
            failed("\n").failure().as_deref(),
            Some("lệnh thoát với mã 1")
        );
    }
}
