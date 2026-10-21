//! Everything that differs between operating systems sits behind this module,
//! so Linux (pkexec) and Windows (UAC) can be added beside macOS later.

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
    /// The script ran and exited with a non-zero status.
    Failed {
        exit: i32,
        stderr: String,
    },
}

/// Runs a shell script as root, asking for the administrator password when the
/// cached rights have expired.
pub trait PrivilegedRunner: Send + Sync {
    fn run(&self, script: &str) -> RunOutcome;
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
