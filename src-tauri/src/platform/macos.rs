//! macOS: root through Authorization Services, the resolver through
//! `dscacheutil`, executable paths through libproc.

use std::fs::File;
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::{Mutex, MutexGuard};

use security_framework::authorization::{Authorization, AuthorizationItemSetBuilder, Flags};

use super::{PrivilegedRunner, RunOutcome};

/// `errAuthorizationCanceled`: the user dismissed the password prompt.
const ERR_AUTHORIZATION_CANCELED: i32 = -60006;
/// The right `AuthorizationExecuteWithPrivileges` checks before it runs a tool.
const ADMIN_RIGHT: &str = "system.privilege.admin";
const EXIT_MARKER: &str = "HOSVEN_EXIT:";
/// Set to `1` by UI tests, so nothing ever asks for a password or runs as root.
const NO_PRIVILEGED_WRITES: &str = "HOSVEN_NO_PRIVILEGED_WRITES";

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
        self.cached
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
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
    fn run(&self, script: &str) -> RunOutcome {
        if switched_off(std::env::var(NO_PRIVILEGED_WRITES).ok().as_deref()) {
            return RunOutcome::Cancelled;
        }
        // Held for the whole run, so two writes never stack two prompts.
        let mut cached = self.lock();
        let auth = match cached.take() {
            Some(CachedAuthorization(auth)) => auth,
            None => match Self::authorize() {
                Ok(auth) => auth,
                Err(code) => return status_outcome(code),
            },
        };
        match auth.execute_with_privileges_piped("/bin/sh", ["-c", &wrap(script)], Flags::DEFAULTS)
        {
            Ok(pipe) => {
                *cached = Some(CachedAuthorization(auth));
                collect(pipe)
            }
            // The rights may have expired or been revoked; dropping `auth`
            // frees it, and the next run asks again.
            Err(error) => status_outcome(error.code()),
        }
    }

    fn release(&self) {
        self.lock().take();
    }
}

fn switched_off(value: Option<&str>) -> bool {
    value == Some("1")
}

/// `$$` is the shell's pid: the setuid trampoline execs `/bin/sh`, so it is
/// the very child to reap. The subshell keeps an `exit` in `script` from
/// skipping the line that reports the status.
fn wrap(script: &str) -> String {
    format!("echo $$\n(\n{script}\n) 2>&1\necho {EXIT_MARKER}$?\n")
}

fn status_outcome(code: i32) -> RunOutcome {
    if code == ERR_AUTHORIZATION_CANCELED {
        RunOutcome::Cancelled
    } else {
        RunOutcome::Denied(code)
    }
}

/// Reads the script's output to the end, which is when it exits, then reaps
/// the shell so it does not linger as a zombie.
fn collect(mut pipe: File) -> RunOutcome {
    let mut bytes = Vec::new();
    let read = pipe.read_to_end(&mut bytes);
    drop(pipe);
    let output = parse_output(&String::from_utf8_lossy(&bytes));
    let waited = output.pid.and_then(reap);
    if let Err(error) = read {
        return RunOutcome::Failed {
            exit: -1,
            stderr: format!("không đọc được kết quả lệnh: {error}"),
        };
    }
    exit_outcome(output, waited)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_the_script_so_pid_and_status_come_back() {
        assert_eq!(
            wrap("cp a b || exit $?; true"),
            "echo $$\n(\ncp a b || exit $?; true\n) 2>&1\necho HOSVEN_EXIT:$?\n"
        );
    }

    #[test]
    fn a_dismissed_prompt_is_a_cancel_and_other_codes_are_denials() {
        assert_eq!(status_outcome(-60006), RunOutcome::Cancelled);
        assert_eq!(status_outcome(-60005), RunOutcome::Denied(-60005));
        assert_eq!(status_outcome(-60007), RunOutcome::Denied(-60007));
    }

    #[test]
    fn reads_pid_output_and_status() {
        let output = parse_output("4242\ncp: /etc/hosts: Read-only file system\nHOSVEN_EXIT:1\n");
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
            exit_outcome(parse_output("17\nHOSVEN_EXIT:0\n"), Some(0)),
            RunOutcome::Ok
        );
    }

    #[test]
    fn never_waits_for_a_pid_that_means_any_child() {
        assert_eq!(parse_output("0\nHOSVEN_EXIT:0\n").pid, None);
        assert_eq!(parse_output("-1\nHOSVEN_EXIT:0\n").pid, None);
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
    fn parses_resolver_addresses() {
        let text = include_str!("../../tests/fixtures/dscacheutil-host.txt");
        assert_eq!(parse_resolved(text), ["::1", "127.0.0.1", "10.0.0.5"]);
        assert!(parse_resolved("").is_empty());
    }

    #[test]
    fn only_the_value_one_turns_privileged_runs_off() {
        assert!(switched_off(Some("1")));
        assert!(!switched_off(Some("0")));
        assert!(!switched_off(Some("")));
        assert!(!switched_off(None));
    }

    #[test]
    fn finds_its_own_executable() {
        let path = process_path(std::process::id() as i32).expect("own path");
        assert!(path.starts_with('/'));
    }
}
