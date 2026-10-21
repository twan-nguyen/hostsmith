//! Everything that differs between operating systems sits behind this module,
//! so Linux (pkexec) and Windows (UAC) can be added beside macOS later.

use std::sync::Arc;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::{MacPrivilegedRunner as SystemRunner, process_path, resolve_host};

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
    fn run(&self, script: &str) -> RunOutcome;

    /// Forgets cached rights, so the next run asks again.
    fn release(&self) {}
}

/// The one runner the app shares, so a single password prompt covers hosts
/// writes, backups and stopping processes of other users.
pub type SharedRunner = Arc<dyn PrivilegedRunner>;

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
