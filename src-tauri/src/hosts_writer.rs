//! Replaces the hosts file as root: the content goes to a private temporary
//! file first, and the root script only copies that file, so no text the user
//! typed ever becomes part of a command.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use crate::platform::{HOSTS_PATH, HostsWriter, RunOutcome, SharedRunner, WriteOutcome};

pub struct SystemHostsWriter {
    runner: SharedRunner,
}

impl SystemHostsWriter {
    pub fn new(runner: SharedRunner) -> Self {
        Self { runner }
    }
}

impl HostsWriter for SystemHostsWriter {
    fn write(&self, content: &str) -> WriteOutcome {
        let tmp = std::env::temp_dir().join(format!("hosts_{}", uuid::Uuid::new_v4()));
        if let Err(error) = write_private(&tmp, content) {
            let _ = fs::remove_file(&tmp);
            return WriteOutcome::Failed(format!("không tạo được file tạm: {error}"));
        }
        let outcome = self.runner.run(&write_script(&tmp));
        if outcome != RunOutcome::Ok {
            let _ = fs::remove_file(&tmp);
        }
        write_outcome(outcome)
    }
}

/// Only this user can read the file: it may list internal hostnames.
fn write_private(path: &Path, content: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()
}

/// The status reflects the copy alone. Once the file is in place a failed
/// cache flush must still read as written, or a switch would roll the model
/// back while the disk holds the new content.
fn write_script(tmp: &Path) -> String {
    let tmp = quote(&tmp.to_string_lossy());
    format!(
        "cp {tmp} {HOSTS_PATH} && rm -f {tmp} || exit $?; \
         dscacheutil -flushcache; killall -HUP mDNSResponder 2>/dev/null; exit 0"
    )
}

/// Single-quotes `value` for `sh`, closing and reopening the quotes around
/// each `'` it holds.
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

fn write_outcome(outcome: RunOutcome) -> WriteOutcome {
    match outcome {
        RunOutcome::Ok => WriteOutcome::Written,
        RunOutcome::Cancelled => WriteOutcome::Cancelled,
        RunOutcome::Failed { exit, stderr } if stderr.trim().is_empty() => {
            WriteOutcome::Failed(format!("lệnh ghi thoát với mã {exit}"))
        }
        other => WriteOutcome::Failed(other.failure().unwrap_or_default()),
    }
}

/// Where a backup of the hosts file taken at `now` goes, e.g.
/// `/etc/hosts.backup.2026-10-21T06-05-00Z`. Colons are replaced because
/// Finder shows them as slashes.
pub fn backup_path(now: jiff::Timestamp) -> PathBuf {
    let stamp = now.strftime("%Y-%m-%dT%H-%M-%SZ");
    PathBuf::from(format!("{HOSTS_PATH}.backup.{stamp}"))
}

/// The backup script. The name holds only digits and separators; it is quoted
/// anyway, so every path in a root script is.
pub fn backup_script(path: &Path) -> String {
    format!("cp {HOSTS_PATH} {}", quote(&path.to_string_lossy()))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::platform::PrivilegedRunner;

    /// Records scripts and answers with a fixed outcome instead of running.
    struct Recorder {
        outcome: RunOutcome,
        scripts: Mutex<Vec<String>>,
    }

    impl PrivilegedRunner for Recorder {
        fn run(&self, script: &str) -> RunOutcome {
            self.scripts.lock().unwrap().push(script.to_string());
            self.outcome.clone()
        }
    }

    fn writer(outcome: RunOutcome) -> (SystemHostsWriter, Arc<Recorder>) {
        let recorder = Arc::new(Recorder {
            outcome,
            scripts: Mutex::new(Vec::new()),
        });
        (SystemHostsWriter::new(recorder.clone()), recorder)
    }

    fn tmp_in(script: &str) -> PathBuf {
        let start = script.find('\'').unwrap() + 1;
        let end = start + script[start..].find('\'').unwrap();
        PathBuf::from(&script[start..end])
    }

    #[test]
    fn quotes_single_quotes_for_sh() {
        assert_eq!(quote("/tmp/a b"), "'/tmp/a b'");
        assert_eq!(quote("/tmp/it's"), r"'/tmp/it'\''s'");
    }

    #[test]
    fn the_script_copies_then_flushes_without_letting_the_flush_fail_it() {
        assert_eq!(
            write_script(Path::new("/var/folders/x/T/hosts_1")),
            "cp '/var/folders/x/T/hosts_1' /etc/hosts && rm -f '/var/folders/x/T/hosts_1' || exit $?; \
             dscacheutil -flushcache; killall -HUP mDNSResponder 2>/dev/null; exit 0"
        );
    }

    #[test]
    fn maps_run_outcomes_to_write_outcomes() {
        assert_eq!(write_outcome(RunOutcome::Ok), WriteOutcome::Written);
        assert_eq!(
            write_outcome(RunOutcome::Cancelled),
            WriteOutcome::Cancelled
        );
        assert_eq!(
            write_outcome(RunOutcome::Denied(-60005)),
            WriteOutcome::Failed("quyền quản trị bị từ chối (mã -60005)".into())
        );
        assert_eq!(
            write_outcome(RunOutcome::Failed {
                exit: 1,
                stderr: "cp: /etc/hosts: Operation not permitted\n".into(),
            }),
            WriteOutcome::Failed("cp: /etc/hosts: Operation not permitted".into())
        );
        assert_eq!(
            write_outcome(RunOutcome::Failed {
                exit: 2,
                stderr: String::new(),
            }),
            WriteOutcome::Failed("lệnh ghi thoát với mã 2".into())
        );
    }

    #[test]
    fn writes_a_private_temp_file_for_the_root_script() {
        use std::os::unix::fs::PermissionsExt;

        // The recorder does not copy anything, so the temp file is still there
        // to inspect; a real run removes it in the script.
        let (writer, recorder) = writer(RunOutcome::Ok);
        assert_eq!(writer.write("127.0.0.1\tapi.test\n"), WriteOutcome::Written);
        let script = recorder.scripts.lock().unwrap()[0].clone();
        let tmp = tmp_in(&script);
        assert!(tmp.starts_with(std::env::temp_dir()));
        assert_eq!(fs::read_to_string(&tmp).unwrap(), "127.0.0.1\tapi.test\n");
        let mode = fs::metadata(&tmp).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        fs::remove_file(tmp).unwrap();
    }

    #[test]
    fn removes_the_temp_file_when_the_write_does_not_land() {
        for outcome in [
            RunOutcome::Cancelled,
            RunOutcome::Denied(-1),
            RunOutcome::Failed {
                exit: 1,
                stderr: String::new(),
            },
        ] {
            let (writer, recorder) = writer(outcome);
            writer.write("x\n");
            let tmp = tmp_in(&recorder.scripts.lock().unwrap()[0]);
            assert!(!tmp.exists());
        }
    }

    #[test]
    fn names_backups_after_the_utc_time_without_colons() {
        let now: jiff::Timestamp = "2026-10-21T06:05:00.123Z".parse().unwrap();
        assert_eq!(
            backup_path(now),
            PathBuf::from("/etc/hosts.backup.2026-10-21T06-05-00Z")
        );
        assert_eq!(
            backup_script(&backup_path(now)),
            "cp /etc/hosts '/etc/hosts.backup.2026-10-21T06-05-00Z'"
        );
    }
}
