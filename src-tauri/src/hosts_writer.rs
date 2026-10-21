//! Replaces the hosts file as root. The content goes to the root script on
//! its standard input, so no text the user typed ever becomes part of a
//! command, and nothing is staged in a file another process could swap.

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
        let bytes = content.as_bytes();
        write_outcome(
            self.runner
                .run_with_input(&write_script(bytes.len()), bytes),
        )
    }
}

/// The content goes to a root-owned file beside the hosts file first, and
/// only a complete copy is renamed over it: if Hosven died mid-stream, `cat`
/// would see an early end of input, and writing straight into the hosts file
/// would leave it cut short. The status reflects the write alone; once the
/// file is in place a failed cache flush must still read as written, or a
/// switch would roll the model back while the disk holds the new content.
fn write_script(len: usize) -> String {
    format!(
        "t=$(/usr/bin/mktemp /private/etc/hosts.hosven.XXXXXX) || exit $?; \
         /bin/cat > \"$t\" || {{ /bin/rm -f \"$t\"; exit 1; }}; \
         [ \"$(/usr/bin/wc -c < \"$t\" | /usr/bin/tr -d ' ')\" = \"{len}\" ] \
         || {{ /bin/rm -f \"$t\"; echo 'nội dung nhận được không đủ' >&2; exit 1; }}; \
         /bin/chmod 644 \"$t\" && /usr/sbin/chown root:wheel \"$t\" && /bin/mv -f \"$t\" {HOSTS_PATH} \
         || {{ /bin/rm -f \"$t\"; exit 1; }}; \
         /usr/bin/dscacheutil -flushcache; /usr/bin/killall -HUP mDNSResponder 2>/dev/null; exit 0"
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
/// `/etc/hosts.backup.2026-10-21T06-05-00.123Z`. Colons are replaced because
/// Finder shows them as slashes; the milliseconds keep two backups in the
/// same second apart.
pub fn backup_path(now: jiff::Timestamp) -> PathBuf {
    let stamp = now.strftime("%Y-%m-%dT%H-%M-%S%.3fZ");
    PathBuf::from(format!("{HOSTS_PATH}.backup.{stamp}"))
}

/// The backup script. The name holds only digits and separators; it is quoted
/// anyway, so every path in a root script is.
pub fn backup_script(path: &Path) -> String {
    format!("/bin/cp {HOSTS_PATH} {}", quote(&path.to_string_lossy()))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::platform::PrivilegedRunner;

    /// Records scripts and their input, and answers with a fixed outcome
    /// instead of running anything.
    struct Recorder {
        outcome: RunOutcome,
        runs: Mutex<Vec<(String, Vec<u8>)>>,
    }

    impl PrivilegedRunner for Recorder {
        fn run_with_input(&self, script: &str, input: &[u8]) -> RunOutcome {
            self.runs
                .lock()
                .unwrap()
                .push((script.to_string(), input.to_vec()));
            self.outcome.clone()
        }
    }

    fn writer(outcome: RunOutcome) -> (SystemHostsWriter, Arc<Recorder>) {
        let recorder = Arc::new(Recorder {
            outcome,
            runs: Mutex::new(Vec::new()),
        });
        (SystemHostsWriter::new(recorder.clone()), recorder)
    }

    /// Every command a root script runs is named by its absolute path, so the
    /// PATH it runs under never decides which program that is.
    fn commands_are_absolute(script: &str) -> bool {
        const TOOLS: [&str; 11] = [
            "cat",
            "rm",
            "mv",
            "cp",
            "chmod",
            "chown",
            "mktemp",
            "wc",
            "tr",
            "dscacheutil",
            "killall",
        ];
        let words: Vec<&str> = script
            .split(|c: char| c.is_whitespace() || "(){};|&\"'".contains(c))
            .filter(|w| !w.is_empty())
            .collect();
        words.iter().all(|word| !TOOLS.contains(word))
    }

    #[test]
    fn quotes_single_quotes_for_sh() {
        assert_eq!(quote("/tmp/a b"), "'/tmp/a b'");
        assert_eq!(quote("/tmp/it's"), r"'/tmp/it'\''s'");
    }

    #[test]
    fn the_script_replaces_the_file_only_with_a_complete_copy() {
        let script = write_script(1234);
        assert!(
            script.starts_with("t=$(/usr/bin/mktemp /private/etc/hosts.hosven.XXXXXX) || exit $?;")
        );
        assert!(script.contains("= \"1234\" ]"), "the byte count is checked");
        assert!(script.contains("/bin/mv -f \"$t\" /etc/hosts"));
        assert!(
            script.ends_with("/usr/bin/killall -HUP mDNSResponder 2>/dev/null; exit 0"),
            "a failed flush does not fail the write"
        );
        assert!(commands_are_absolute(&script));
        assert!(commands_are_absolute(&backup_script(Path::new(
            "/etc/hosts.backup.x"
        ))));
        assert!(!commands_are_absolute("cp a b; exit 0"));
    }

    #[test]
    fn sends_the_content_on_standard_input() {
        let (writer, recorder) = writer(RunOutcome::Ok);
        assert_eq!(writer.write("127.0.0.1\tapi.test\n"), WriteOutcome::Written);
        let runs = recorder.runs.lock().unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].0, write_script(19));
        assert_eq!(runs[0].1, b"127.0.0.1\tapi.test\n");
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
                stderr: "sh: /etc/hosts: Operation not permitted\n".into(),
            }),
            WriteOutcome::Failed("sh: /etc/hosts: Operation not permitted".into())
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
    fn names_backups_after_the_utc_time_to_the_millisecond_without_colons() {
        let now: jiff::Timestamp = "2026-10-21T06:05:00.123Z".parse().unwrap();
        assert_eq!(
            backup_path(now),
            PathBuf::from("/etc/hosts.backup.2026-10-21T06-05-00.123Z")
        );
        assert_eq!(
            backup_script(&backup_path(now)),
            "/bin/cp /etc/hosts '/etc/hosts.backup.2026-10-21T06-05-00.123Z'"
        );
        let later: jiff::Timestamp = "2026-10-21T06:05:00.124Z".parse().unwrap();
        assert_ne!(backup_path(now), backup_path(later));
        let whole: jiff::Timestamp = "2026-10-21T06:05:00Z".parse().unwrap();
        assert_eq!(
            backup_path(whole),
            PathBuf::from("/etc/hosts.backup.2026-10-21T06-05-00.000Z")
        );
    }
}
