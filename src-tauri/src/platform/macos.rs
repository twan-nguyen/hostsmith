//! macOS: root through Authorization Services, the resolver through
//! `dscacheutil`, process details through libproc.

use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::process::{Command, Stdio};
use std::sync::{Mutex, MutexGuard, TryLockError};
use std::time::{Duration, Instant};

use security_framework::authorization::{Authorization, AuthorizationItemSetBuilder, Flags};

use super::{PrivilegedRunner, ProcessIdentity, RunOutcome};

/// `errAuthorizationCanceled`: the user dismissed the password prompt.
const ERR_AUTHORIZATION_CANCELED: i32 = -60006;
/// The right `AuthorizationExecuteWithPrivileges` checks before it runs a tool.
const ADMIN_RIGHT: &str = "system.privilege.admin";
const EXIT_MARKER: &str = "HOSTSMITH_EXIT:";
/// The tool the trampoline runs as root. It starts the shell with an empty
/// environment: the app's own PATH comes from whatever started it, a terminal
/// included, and may list folders this user can write to, which must never
/// decide what root runs.
const TOOL: &str = "/usr/bin/env";
const ROOT_PATH: &str = "PATH=/usr/bin:/bin:/usr/sbin:/sbin";
/// How long a script may run once it has started. The password prompt comes
/// before that and is never cut short.
const SCRIPT_LIMIT: Duration = Duration::from_secs(30);
const NOT_RESPONDING: &str = "lệnh quản trị không phản hồi";

/// Holds the rights between runs. macOS keeps the admin credential inside the
/// `Authorization` that obtained it for about five minutes, so reusing one
/// handle is what spares the user a prompt on every write.
struct CachedAuthorization(Authorization);

// SAFETY: an `AuthorizationRef` is an opaque handle into the Security server,
// and Authorization Services may be called from any thread. The handle is not
// `Send` only because it is a raw pointer. It lives solely inside the runner's
// mutex, so exactly one thread uses or frees it at a time.
unsafe impl Send for CachedAuthorization {}

pub struct MacPrivilegedRunner {
    cached: Mutex<Option<CachedAuthorization>>,
}

impl MacPrivilegedRunner {
    pub fn new() -> Self {
        Self {
            cached: Mutex::new(None),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Option<CachedAuthorization>> {
        crate::util::lock(&self.cached)
    }

    /// Asks for the admin right up front, which is when the prompt shows.
    fn authorize() -> Result<Authorization, i32> {
        let rights = AuthorizationItemSetBuilder::new()
            .add_right(ADMIN_RIGHT)
            .map_err(|e| e.code())?
            .build();
        Authorization::new(
            Some(rights),
            None,
            Flags::INTERACTION_ALLOWED | Flags::PREAUTHORIZE | Flags::EXTEND_RIGHTS,
        )
        .map_err(|e| e.code())
    }
}

impl PrivilegedRunner for MacPrivilegedRunner {
    /// The prompt shows either in `authorize` or, once the cached credential
    /// has expired, inside `execute_with_privileges_piped`, which returns only
    /// after the tool has started. Neither is bounded, since the user may take
    /// their time typing; the limit starts once the script runs.
    fn run_with_input(&self, script: &str, input: &[u8]) -> RunOutcome {
        // Held for the whole run, so two writes never stack two prompts.
        let mut cached = self.lock();
        let auth = match cached.take() {
            Some(CachedAuthorization(auth)) => auth,
            None => match Self::authorize() {
                Ok(auth) => auth,
                Err(code) => return status_outcome(code),
            },
        };
        match auth.execute_with_privileges_piped(TOOL, tool_args(script), Flags::DEFAULTS) {
            Ok(channel) => {
                *cached = Some(CachedAuthorization(auth));
                exchange(channel, input, SCRIPT_LIMIT)
            }
            // The rights may have expired or been revoked; dropping `auth`
            // frees it, and the next run asks again.
            Err(error) => status_outcome(error.code()),
        }
    }

    /// Never waits: a run in progress may sit behind an open password prompt,
    /// and quitting must not hang on it. The Security server drops whatever
    /// that run holds when this process exits.
    fn release(&self) {
        match self.cached.try_lock() {
            Ok(mut cached) => drop(cached.take()),
            Err(TryLockError::Poisoned(poisoned)) => drop(poisoned.into_inner().take()),
            Err(TryLockError::WouldBlock) => {}
        }
    }
}

/// `env -i` clears the environment and sets a PATH of system folders only.
/// The trampoline raises only the effective uid, and a shell started that way
/// drops it again unless it gets `-p`; the trampoline asks for that through an
/// environment variable that `-i` removes, so the flag is passed directly.
fn tool_args(script: &str) -> [String; 6] {
    [
        "-i".into(),
        ROOT_PATH.into(),
        "/bin/sh".into(),
        "-p".into(),
        "-c".into(),
        wrap(script),
    ]
}

/// `$$` is the shell's pid: the setuid trampoline execs `env`, which execs
/// `/bin/sh`, so it is the very child to reap. The subshell keeps an `exit`
/// in `script` from skipping the line that reports the status, which goes on
/// a line of its own and is also the shell's own exit status.
fn wrap(script: &str) -> String {
    format!(
        "echo $$\n(\n{script}\n) 2>&1\nrc=$?; printf '\\n{EXIT_MARKER}%d\\n' \"$rc\"; exit \"$rc\"\n"
    )
}

fn status_outcome(code: i32) -> RunOutcome {
    if code == ERR_AUTHORIZATION_CANCELED {
        RunOutcome::Cancelled
    } else {
        RunOutcome::Denied(code)
    }
}

// MARK: - Talking to the script

/// The channel is one socket that is both the script's standard input and its
/// standard output. `input` goes out, then the write side is shut down so the
/// script reads end of file; the output is read to its end, which is when the
/// script exits. A script still running at `limit` is killed.
fn exchange(channel: File, input: &[u8], limit: Duration) -> RunOutcome {
    let transfer = transfer(&channel, input, limit);
    let output = parse_output(&String::from_utf8_lossy(&transfer.bytes));
    if transfer.timed_out || transfer.read_error.is_some() {
        if let Some(pid) = output.pid {
            stop_tree(pid);
        }
        return RunOutcome::Failed {
            exit: -1,
            stderr: match transfer.read_error {
                Some(error) => format!("không đọc được kết quả lệnh: {error}"),
                None => NOT_RESPONDING.into(),
            },
        };
    }
    drop(channel);
    let waited = output.pid.and_then(reap);
    match transfer.input_error {
        Some(error) => input_failure(output, waited, &error),
        None => exit_outcome(output, waited),
    }
}

#[derive(Debug, Default)]
struct Transfer {
    bytes: Vec<u8>,
    /// Why the input did not reach the script in full.
    input_error: Option<String>,
    read_error: Option<String>,
    timed_out: bool,
}

/// Writes and reads at the same time, so a script that prints while it reads
/// can never block on a full socket while this side blocks on writing.
fn transfer(channel: &File, input: &[u8], limit: Duration) -> Transfer {
    let fd = channel.as_raw_fd();
    let deadline = Instant::now() + limit;
    let mut result = Transfer::default();
    if let Err(error) = set_nonblocking(fd) {
        result.read_error = Some(error.to_string());
        return result;
    }
    let mut sent = 0;
    let mut sending = !input.is_empty();
    if !sending {
        // Nothing to send; a script that reads anyway gets end of file. A
        // failure here only matters to a script that reads, and the limit
        // covers that one.
        let _ = end_input(fd);
    }
    let mut buffer = [0u8; 16 * 1024];
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            result.timed_out = true;
            break;
        }
        let events = libc::POLLIN | if sending { libc::POLLOUT } else { 0 };
        let mut poll = libc::pollfd {
            fd,
            events,
            revents: 0,
        };
        let millis = left.as_millis().clamp(1, i32::MAX as u128) as i32;
        // SAFETY: one valid `pollfd` for the duration of the call.
        let ready = unsafe { libc::poll(&mut poll, 1, millis) };
        if ready < 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() == ErrorKind::Interrupted {
                continue;
            }
            result.read_error = Some(error.to_string());
            break;
        }
        if ready == 0 {
            continue;
        }
        let failed = poll.revents & (libc::POLLERR | libc::POLLHUP) != 0;
        if sending && (poll.revents & libc::POLLOUT != 0 || failed) {
            match (&*channel).write(&input[sent..]) {
                Ok(count) => {
                    sent += count;
                    if sent == input.len() {
                        sending = false;
                        if let Err(error) = end_input(fd) {
                            result.input_error = Some(error.to_string());
                        }
                    }
                }
                Err(error) if is_retry(&error) => {}
                Err(error) => {
                    sending = false;
                    result.input_error = Some(error.to_string());
                }
            }
        }
        if poll.revents & libc::POLLIN != 0 || failed {
            match (&*channel).read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => result.bytes.extend_from_slice(&buffer[..count]),
                Err(error) if is_retry(&error) => {}
                Err(error) => {
                    result.read_error = Some(error.to_string());
                    break;
                }
            }
        }
    }
    if sending && !result.timed_out && result.input_error.is_none() {
        result.input_error = Some("lệnh kết thúc trước khi nhận hết nội dung".into());
    }
    result
}

fn is_retry(error: &std::io::Error) -> bool {
    matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::Interrupted)
}

fn set_nonblocking(fd: RawFd) -> std::io::Result<()> {
    // SAFETY: plain fcntl calls on a descriptor this process owns.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// Shuts down this side's writing only, so the script sees end of file on
/// its standard input while its output still comes back.
fn end_input(fd: RawFd) -> std::io::Result<()> {
    // SAFETY: plain syscall on a socket this process owns.
    if unsafe { libc::shutdown(fd, libc::SHUT_WR) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

/// The input did not arrive in full, so the run failed whatever the script
/// reported: with `cat` reading it, a cut-off input reads as a short file.
fn input_failure(output: Output, waited: Option<i32>, error: &str) -> RunOutcome {
    let exit = output
        .exit
        .or(waited)
        .filter(|code| *code != 0)
        .unwrap_or(-1);
    RunOutcome::Failed {
        exit,
        stderr: if output.body.trim().is_empty() {
            format!("không gửi được nội dung cho lệnh: {error}")
        } else {
            output.body
        },
    }
}

/// Kills the shell and everything it started, then reaps the shell. The
/// trampoline raises only the effective uid, so the real uid is still this
/// user's and the signal is allowed. Children are listed before any of them
/// dies, as a dead parent's children move to launchd.
fn stop_tree(pid: i32) {
    let mut family = vec![pid];
    let mut next = 0;
    // The bound only guards against a pid list that changes under the walk.
    while next < family.len() && family.len() < 1024 {
        for child in child_pids(family[next]) {
            if !family.contains(&child) {
                family.push(child);
            }
        }
        next += 1;
    }
    let mut killed = false;
    for member in &family {
        // SAFETY: plain syscall; every pid in the list is positive.
        let sent = unsafe { libc::kill(*member, libc::SIGKILL) } == 0;
        killed |= sent && *member == pid;
    }
    if killed {
        reap(pid);
    } else {
        // Waiting for a shell that could not be killed would hang this
        // thread, so the zombie is collected in the background instead.
        std::thread::spawn(move || reap(pid));
    }
}

fn child_pids(pid: i32) -> Vec<i32> {
    let mut pids = vec![0i32; 256];
    let size = (pids.len() * std::mem::size_of::<i32>()) as i32;
    // SAFETY: the buffer is writable for the size passed.
    let count = unsafe { libc::proc_listchildpids(pid, pids.as_mut_ptr().cast(), size) };
    pids.truncate(count.max(0) as usize);
    pids.retain(|child| *child > 0);
    pids
}

#[derive(Debug, PartialEq, Eq)]
struct Output {
    pid: Option<i32>,
    /// The status the script reported itself.
    exit: Option<i32>,
    body: String,
}

fn parse_output(text: &str) -> Output {
    let mut lines: Vec<&str> = text.lines().collect();
    // Only a positive pid is safe to wait for: 0 and -1 mean "any child".
    let pid = match lines.first().map(|line| line.trim().parse::<i32>()) {
        Some(Ok(pid)) if pid > 0 => {
            lines.remove(0);
            Some(pid)
        }
        _ => None,
    };
    let exit = lines
        .last()
        .and_then(|line| line.trim().strip_prefix(EXIT_MARKER))
        .and_then(|code| code.parse::<i32>().ok());
    if exit.is_some() {
        lines.pop();
        // The newline printed before the marker, when the output already
        // ended with one.
        if lines.last() == Some(&"") {
            lines.pop();
        }
    }
    Output {
        pid,
        exit,
        body: lines.join("\n"),
    }
}

/// The status the shell exited with, as the kernel saw it.
fn reap(pid: i32) -> Option<i32> {
    let mut status = 0;
    loop {
        // SAFETY: plain syscall on a pid this process spawned; `status` is a
        // valid out pointer for its duration.
        let reaped = unsafe { libc::waitpid(pid, &mut status, 0) };
        if reaped == pid {
            break;
        }
        if reaped == -1 && std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
            continue;
        }
        return None;
    }
    if libc::WIFEXITED(status) {
        Some(libc::WEXITSTATUS(status))
    } else if libc::WIFSIGNALED(status) {
        Some(128 + libc::WTERMSIG(status))
    } else {
        None
    }
}

/// The script's own report wins; the kernel's status covers a shell that was
/// killed before it could print one.
fn exit_outcome(output: Output, waited: Option<i32>) -> RunOutcome {
    match output.exit.or(waited) {
        Some(0) => RunOutcome::Ok,
        Some(exit) => RunOutcome::Failed {
            exit,
            stderr: output.body,
        },
        None => RunOutcome::Failed {
            exit: -1,
            stderr: if output.body.trim().is_empty() {
                "không đọc được mã thoát của lệnh".into()
            } else {
                output.body
            },
        },
    }
}

// MARK: - Resolver and processes

/// The addresses the system resolver returns for `host`, hosts file included.
/// Any failure reads as "does not resolve".
pub fn resolve_host(host: &str) -> Vec<String> {
    Command::new("/usr/bin/dscacheutil")
        .args(["-q", "host", "-a", "name", host])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map(|out| parse_resolved(&String::from_utf8_lossy(&out.stdout)))
        .unwrap_or_default()
}

/// `dscacheutil` prints one block per answer, with `ip_address:` and
/// `ipv6_address:` lines among others.
fn parse_resolved(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| line.split_once(':'))
        .filter(|(key, _)| matches!(key.trim(), "ip_address" | "ipv6_address"))
        .map(|(_, value)| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

/// The executable a process runs, or `None` for one this user cannot inspect.
pub fn process_path(pid: i32) -> Option<String> {
    let mut buffer = vec![0u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
    // SAFETY: the buffer is writable for the length passed.
    let length =
        unsafe { libc::proc_pidpath(pid, buffer.as_mut_ptr().cast(), buffer.len() as u32) };
    if length <= 0 {
        return None;
    }
    buffer.truncate(length as usize);
    Some(String::from_utf8_lossy(&buffer).into_owned())
}

/// Who runs `pid` and since when, or `None` when it is gone or this user
/// cannot inspect it.
pub fn process_identity(pid: i32) -> Option<ProcessIdentity> {
    if pid <= 0 {
        return None;
    }
    let size = std::mem::size_of::<libc::proc_bsdinfo>() as i32;
    // SAFETY: `proc_bsdinfo` is plain integers and arrays, for which all
    // zero bytes are a valid value.
    let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
    // SAFETY: the out pointer is writable for `size` bytes.
    let written = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDTBSDINFO,
            0,
            (&mut info as *mut libc::proc_bsdinfo).cast(),
            size,
        )
    };
    (written == size).then_some(ProcessIdentity {
        uid: info.pbi_uid,
        started: (info.pbi_start_tvsec, info.pbi_start_tvusec),
    })
}

#[cfg(test)]
mod tests {
    use std::os::fd::OwnedFd;
    use std::os::unix::net::UnixStream;
    use std::path::PathBuf;

    use super::*;

    /// Starts `script` the way the trampoline does, minus root: the same tool
    /// and arguments, with one socket as standard input and output. The
    /// returned end is the channel the runner talks over.
    // `exchange` reaps the shell by the pid it prints, as it does in the app.
    #[allow(clippy::zombie_processes)]
    fn start(script: &str) -> File {
        let (ours, theirs) = UnixStream::pair().unwrap();
        let theirs = OwnedFd::from(theirs);
        Command::new(TOOL)
            .args(tool_args(script))
            .stdin(Stdio::from(theirs.try_clone().unwrap()))
            .stdout(Stdio::from(theirs))
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        File::from(OwnedFd::from(ours))
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("hostsmith-{name}-{}", uuid::Uuid::new_v4()))
    }

    fn quote(path: &std::path::Path) -> String {
        format!("'{}'", path.display())
    }

    #[test]
    fn the_shell_starts_with_an_empty_environment_and_system_path() {
        let args = tool_args("true");
        assert_eq!(TOOL, "/usr/bin/env");
        assert_eq!(
            args[..5],
            [
                "-i",
                "PATH=/usr/bin:/bin:/usr/sbin:/sbin",
                "/bin/sh",
                "-p",
                "-c"
            ]
        );
        assert_eq!(args[5], wrap("true"));

        // HOME is set for the tests; the script must not see it, nor the
        // test's PATH.
        let script = r#"[ -z "${HOME-}" ] && [ "$PATH" = /usr/bin:/bin:/usr/sbin:/sbin ] || { echo "HOME=${HOME-} PATH=$PATH"; exit 1; }"#;
        assert_eq!(
            exchange(start(script), &[], Duration::from_secs(10)),
            RunOutcome::Ok
        );
    }

    #[test]
    fn wraps_the_script_so_pid_and_status_come_back() {
        assert_eq!(
            wrap("cp a b || exit $?; true"),
            "echo $$\n(\ncp a b || exit $?; true\n) 2>&1\n\
             rc=$?; printf '\\nHOSTSMITH_EXIT:%d\\n' \"$rc\"; exit \"$rc\"\n"
        );
    }

    #[test]
    fn streams_input_to_the_script() {
        let path = temp_path("stdin");
        // Larger than the socket buffers, so the script reads while this
        // side still writes.
        let content = "127.0.0.1\tapi.test\n".repeat(20_000);
        let script = format!("/bin/cat > {} || exit $?; echo done; exit 0", quote(&path));
        assert_eq!(
            exchange(start(&script), content.as_bytes(), Duration::from_secs(10)),
            RunOutcome::Ok
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn a_script_without_input_reads_end_of_file() {
        assert_eq!(
            exchange(start("/bin/cat"), &[], Duration::from_secs(10)),
            RunOutcome::Ok
        );
    }

    #[test]
    fn reports_the_status_and_output_of_a_failing_script() {
        assert_eq!(
            exchange(start("echo oops; exit 3"), &[], Duration::from_secs(10)),
            RunOutcome::Failed {
                exit: 3,
                stderr: "oops".into(),
            }
        );
        assert_eq!(
            exchange(
                start("printf partial; exit 2"),
                &[],
                Duration::from_secs(10)
            ),
            RunOutcome::Failed {
                exit: 2,
                stderr: "partial".into(),
            }
        );
    }

    #[test]
    fn a_script_that_stops_reading_fails() {
        let content = vec![b'x'; 1 << 20];
        let outcome = exchange(start("exit 4"), &content, Duration::from_secs(10));
        assert!(
            matches!(outcome, RunOutcome::Failed { exit: 4, .. }),
            "{outcome:?}"
        );
    }

    #[test]
    fn kills_a_script_that_does_not_finish_in_time() {
        let marker = temp_path("sleeper");
        let script = format!("/bin/sleep 30 & echo $! > {}; wait", quote(&marker));
        let started = Instant::now();
        let outcome = exchange(start(&script), &[], Duration::from_millis(500));
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(
            outcome,
            RunOutcome::Failed {
                exit: -1,
                stderr: "lệnh quản trị không phản hồi".into(),
            }
        );
        // The script's own child is killed too, not left behind.
        let sleeper: i32 = std::fs::read_to_string(&marker)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        std::fs::remove_file(marker).unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while process_identity(sleeper).is_some() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(process_identity(sleeper), None);
    }

    #[test]
    fn a_dismissed_prompt_is_a_cancel_and_other_codes_are_denials() {
        assert_eq!(status_outcome(-60006), RunOutcome::Cancelled);
        assert_eq!(status_outcome(-60005), RunOutcome::Denied(-60005));
        assert_eq!(status_outcome(-60007), RunOutcome::Denied(-60007));
    }

    #[test]
    fn reads_pid_output_and_status() {
        let output =
            parse_output("4242\ncp: /etc/hosts: Read-only file system\n\nHOSTSMITH_EXIT:1\n");
        assert_eq!(
            output,
            Output {
                pid: Some(4242),
                exit: Some(1),
                body: "cp: /etc/hosts: Read-only file system".into(),
            }
        );
        assert_eq!(
            exit_outcome(output, Some(0)),
            RunOutcome::Failed {
                exit: 1,
                stderr: "cp: /etc/hosts: Read-only file system".into(),
            }
        );
        assert_eq!(
            parse_output("17\nno newline\nHOSTSMITH_EXIT:5\n").body,
            "no newline"
        );
        assert_eq!(
            exit_outcome(parse_output("17\n\nHOSTSMITH_EXIT:0\n"), Some(0)),
            RunOutcome::Ok
        );
    }

    #[test]
    fn never_waits_for_a_pid_that_means_any_child() {
        assert_eq!(parse_output("0\n\nHOSTSMITH_EXIT:0\n").pid, None);
        assert_eq!(parse_output("-1\n\nHOSTSMITH_EXIT:0\n").pid, None);
        assert_eq!(parse_output("").pid, None);
    }

    #[test]
    fn falls_back_to_the_kernel_status_when_the_script_did_not_report() {
        let output = parse_output("99\npartial output\n");
        assert_eq!(output.exit, None);
        assert_eq!(
            exit_outcome(output, Some(137)),
            RunOutcome::Failed {
                exit: 137,
                stderr: "partial output".into(),
            }
        );
        assert_eq!(
            exit_outcome(parse_output(""), None),
            RunOutcome::Failed {
                exit: -1,
                stderr: "không đọc được mã thoát của lệnh".into(),
            }
        );
    }

    #[test]
    fn a_cut_off_input_fails_even_when_the_script_succeeded() {
        assert_eq!(
            input_failure(
                parse_output("9\n\nHOSTSMITH_EXIT:0\n"),
                Some(0),
                "Broken pipe"
            ),
            RunOutcome::Failed {
                exit: -1,
                stderr: "không gửi được nội dung cho lệnh: Broken pipe".into(),
            }
        );
    }

    #[test]
    fn release_does_not_wait_for_a_run_in_progress() {
        let runner = MacPrivilegedRunner::new();
        let held = runner.lock();
        runner.release();
        drop(held);
        runner.release();
    }

    #[test]
    fn parses_resolver_addresses() {
        let text = include_str!("../../tests/fixtures/dscacheutil-host.txt");
        assert_eq!(parse_resolved(text), ["::1", "127.0.0.1", "10.0.0.5"]);
        assert!(parse_resolved("").is_empty());
    }

    #[test]
    fn finds_its_own_executable() {
        let path = process_path(std::process::id() as i32).expect("own path");
        assert!(path.starts_with('/'));
    }

    #[test]
    fn identifies_a_process_by_owner_and_start_time() {
        let own = process_identity(std::process::id() as i32).expect("own identity");
        // SAFETY: plain getter.
        assert_eq!(own.uid, unsafe { libc::getuid() });
        assert!(own.started.0 > 0);
        assert_eq!(process_identity(std::process::id() as i32), Some(own));
        assert_eq!(process_identity(0), None);
        assert_eq!(process_identity(-5), None);
    }
}
