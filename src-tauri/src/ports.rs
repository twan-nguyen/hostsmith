//! Listening TCP ports: who holds each one, and stopping it. Scans shell out
//! to `lsof` and `docker`; parsing and attribution are pure, so they run
//! against fixtures in unit tests. Every change sends the
//! whole snapshot as `ports-changed`.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::{Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::platform::{self, ProcessIdentity};
use crate::toast::{Toast, ToastKind, Toasts};
use crate::util::lock;

const EVENT: &str = "ports-changed";
const LSOF: &str = "/usr/sbin/lsof";
/// Where Homebrew and Docker Desktop put the CLI. An app opened from Finder
/// gets launchd's bare PATH, which holds none of them.
const DOCKER_PATHS: [&str; 3] = [
    "/opt/homebrew/bin/docker",
    "/usr/local/bin/docker",
    "/Applications/Docker.app/Contents/Resources/bin/docker",
];
/// Where OrbStack and Rancher Desktop put the CLI, under the home folder.
const DOCKER_HOME_PATHS: [&str; 2] = [".orbstack/bin/docker", ".rd/bin/docker"];
const SYSTEM_PATH_PREFIXES: [&str; 5] = [
    "/System/",
    "/usr/libexec/",
    "/usr/sbin/",
    "/Library/Apple/",
    "/private/var/db/",
];
/// macOS daemons whose executable path this user may not be able to read.
const SYSTEM_COMMANDS: [&str; 12] = [
    "ControlCenter",
    "rapportd",
    "sharingd",
    "AirPlayXPCHelper",
    "remoted",
    "launchd",
    "identityservicesd",
    "cupsd",
    "bluetoothd",
    "airportd",
    "mDNSResponder",
    "remotepairingd",
];
/// How long a process gets to exit after the signal before it counts as
/// still running.
const SETTLE: Duration = Duration::from_millis(1200);
/// How long each tool may take before it is killed, so a stuck `lsof` or a
/// Docker daemon that does not answer never leaves the pane scanning forever.
const LSOF_LIMIT: Duration = Duration::from_secs(5);
const DOCKER_PS_LIMIT: Duration = Duration::from_secs(5);
const DOCKER_STOP_LIMIT: Duration = Duration::from_secs(15);

// MARK: - Model

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "name", rename_all = "camelCase")]
pub enum Owner {
    /// Started inside a project, the nearest folder above the working
    /// directory that holds `.git`; carries that folder's name.
    Yours(String),
    /// Published by Docker; carries the container name when known.
    Container(Option<String>),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListeningPort {
    /// `port-pid`, unique within a scan.
    pub id: String,
    pub port: u16,
    pub process: String,
    pub pid: i32,
    pub user: String,
    pub cwd: Option<String>,
    pub owner: Owner,
    /// macOS daemons and helpers, listed only under the system filter.
    pub is_system: bool,
    /// An unknown process on a dev port: the one case worth a banner.
    pub needs_attention: bool,
    /// Who ran the pid and since when at scan time, checked again before any
    /// signal so a pid that now names another process is left alone.
    #[serde(skip)]
    pub identity: Option<ProcessIdentity>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortsSnapshot {
    pub ports: Vec<ListeningPort>,
    /// When the shown list was taken, in milliseconds since the Unix epoch.
    pub scanned_at: Option<i64>,
    pub scanning: bool,
    pub toast: Option<Toast>,
}

/// How stopping a process ended. `StillRunning` lets the window offer SIGKILL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum KillOutcome {
    Freed,
    StillRunning,
    Failed,
}

// MARK: - State

#[derive(Default)]
pub struct Ports {
    ports: Vec<ListeningPort>,
    scanned_at: Option<i64>,
    /// Scans run concurrently (a manual rescan during a kill's rescan), so
    /// each gets a number and an older one never replaces a newer result.
    started: u64,
    applied: u64,
    toasts: Toasts,
}

impl Ports {
    pub fn snapshot(&self) -> PortsSnapshot {
        PortsSnapshot {
            ports: self.ports.clone(),
            scanned_at: self.scanned_at,
            scanning: self.applied < self.started,
            toast: self.toasts.current().cloned(),
        }
    }

    fn begin_scan(&mut self) -> u64 {
        self.started += 1;
        self.started
    }

    fn finish_scan(&mut self, seq: u64, result: Result<Vec<ListeningPort>, String>, now: i64) {
        if seq <= self.applied {
            return;
        }
        self.applied = seq;
        match result {
            Ok(ports) => {
                self.ports = ports;
                self.scanned_at = Some(now);
            }
            Err(error) => self.show_toast(error, ToastKind::Error),
        }
    }

    fn show_toast(&mut self, message: String, kind: ToastKind) {
        self.toasts.show(message, kind);
    }

    /// The first port `pid` holds in the last scan. Only listed processes may
    /// be signalled, so the window cannot be used to stop an arbitrary pid.
    fn listed_pid(&self, pid: i32) -> Option<&ListeningPort> {
        self.ports.iter().find(|p| p.pid == pid)
    }

    fn container_port(&self, name: &str) -> Option<u16> {
        self.ports
            .iter()
            .find(|p| matches!(&p.owner, Owner::Container(Some(n)) if n == name))
            .map(|p| p.port)
    }
}

pub struct PortsState(Mutex<Ports>);

impl PortsState {
    pub fn new() -> Self {
        Self(Mutex::new(Ports::default()))
    }

    fn lock(&self) -> MutexGuard<'_, Ports> {
        lock(&self.0)
    }
}

fn mutate<T>(app: &AppHandle, state: &PortsState, change: impl FnOnce(&mut Ports) -> T) -> T {
    let mut ports = state.lock();
    let result = change(&mut ports);
    let _ = app.emit(EVENT, ports.snapshot());
    result
}

// MARK: - Parsing

#[derive(Debug, Clone, PartialEq, Eq)]
struct Record {
    pid: i32,
    command: String,
    user: String,
    port: u16,
}

/// Parses `lsof -F pcuLn`: one field per line, `p` starts a process and `n`
/// carries `host:port`. A process listening on both IPv4 and IPv6 shows the
/// port twice, so records are unique by pid and port.
fn parse_listing(text: &str) -> Vec<Record> {
    let mut records: Vec<Record> = Vec::new();
    let mut pid: Option<i32> = None;
    let (mut command, mut user, mut login) = (String::new(), String::new(), String::new());
    for line in text.lines() {
        let Some(tag) = line.chars().next() else {
            continue;
        };
        let value = &line[tag.len_utf8()..];
        match tag {
            'p' => {
                pid = value.parse().ok().filter(|pid| *pid > 0);
                command.clear();
                user.clear();
                login.clear();
            }
            'c' => command = value.to_string(),
            'u' => user = value.to_string(),
            'L' => login = value.to_string(),
            'n' => {
                let Some(pid) = pid else { continue };
                let Some(port) = value.rsplit_once(':').and_then(|(_, p)| p.parse().ok()) else {
                    continue;
                };
                if records.iter().any(|r| r.pid == pid && r.port == port) {
                    continue;
                }
                records.push(Record {
                    pid,
                    command: command.clone(),
                    user: if login.is_empty() {
                        user.clone()
                    } else {
                        login.clone()
                    },
                    port,
                });
            }
            _ => {}
        }
    }
    records
}

/// Parses `lsof -a -d cwd -Fpn`: `p` then `n` with the working directory.
fn parse_cwds(text: &str) -> HashMap<i32, String> {
    let mut cwds = HashMap::new();
    let mut pid = None;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix('p') {
            pid = value.parse::<i32>().ok();
        } else if let (Some(value), Some(pid)) = (line.strip_prefix('n'), pid) {
            cwds.insert(pid, value.to_string());
        }
    }
    cwds
}

/// Parses `docker ps --format "{{.Names}}\t{{.Ports}}"` into the container
/// that publishes each host port. Port ranges such as `8080-8081->80-81/tcp`
/// publish every port in the range.
fn parse_containers(text: &str) -> HashMap<u16, String> {
    let mut containers = HashMap::new();
    for line in text.lines() {
        let Some((name, ports)) = line.split_once('\t') else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        for token in ports.split(',') {
            let Some((host, _)) = token.split_once("->") else {
                continue;
            };
            let Some((_, spec)) = host.rsplit_once(':') else {
                continue;
            };
            for port in port_range(spec.trim()).into_iter().flatten() {
                containers.entry(port).or_insert_with(|| name.to_string());
            }
        }
    }
    containers
}

fn port_range(spec: &str) -> Option<std::ops::RangeInclusive<u16>> {
    let (low, high) = match spec.split_once('-') {
        Some((low, high)) => (low.parse().ok()?, high.parse().ok()?),
        None => {
            let port = spec.parse().ok()?;
            (port, port)
        }
    };
    Some(low..=high)
}

// MARK: - Attribution

/// The nearest folder at or above `cwd` that holds `.git`, a directory in a
/// clone or a file in a worktree or submodule. The walk stops before `home`
/// and `/`: a dotfiles repo in the home folder would otherwise claim every
/// process started anywhere under it.
fn project_root(cwd: &Path, home: &Path) -> Option<PathBuf> {
    // Projects live under the home folder. Outside it, a git checkout is a
    // tool's own (Homebrew keeps one in /opt/homebrew), not someone's project.
    if !cwd.is_absolute() || home.as_os_str().is_empty() || !cwd.starts_with(home) {
        return None;
    }
    cwd.ancestors()
        .take_while(|dir| *dir != home && dir.parent().is_some())
        .find(|dir| dir.join(".git").exists())
        .map(Path::to_path_buf)
}

/// The project a working directory belongs to, named after its folder.
fn project_name(cwd: &str, home: &Path) -> Option<String> {
    let root = project_root(Path::new(cwd), home)?;
    Some(root.file_name()?.to_string_lossy().into_owned())
}

/// Docker first, then the project the process runs in.
fn owner(
    record: &Record,
    cwd: Option<&str>,
    project: impl Fn(&str) -> Option<String>,
    containers: &HashMap<u16, String>,
) -> Owner {
    if record.command.to_lowercase().contains("docker") {
        return Owner::Container(containers.get(&record.port).cloned());
    }
    cwd.and_then(project).map_or(Owner::Unknown, Owner::Yours)
}

fn is_system(command: &str, executable: Option<&str>) -> bool {
    executable.is_some_and(|path| SYSTEM_PATH_PREFIXES.iter().any(|p| path.starts_with(p)))
        || SYSTEM_COMMANDS.contains(&command)
}

/// Ports a dev server would pick: above the privileged range and below the
/// ephemeral range macOS hands to background helpers.
fn needs_attention(owner: &Owner, is_system: bool, port: u16) -> bool {
    *owner == Owner::Unknown && !is_system && (1024..=49151).contains(&port)
}

/// What the scan learns about each process beyond the listing.
struct Lookups<E, I, P> {
    executable: E,
    identity: I,
    /// The project a working directory belongs to.
    project: P,
}

fn assemble<E, I, P>(
    records: Vec<Record>,
    cwds: &HashMap<i32, String>,
    lookups: Lookups<E, I, P>,
    containers: &HashMap<u16, String>,
) -> Vec<ListeningPort>
where
    E: Fn(i32) -> Option<String>,
    I: Fn(i32) -> Option<ProcessIdentity>,
    P: Fn(&str) -> Option<String>,
{
    let mut ports: Vec<ListeningPort> = records
        .into_iter()
        .map(|record| {
            let cwd = cwds.get(&record.pid).cloned();
            let owner = owner(&record, cwd.as_deref(), &lookups.project, containers);
            let is_system = is_system(&record.command, (lookups.executable)(record.pid).as_deref());
            ListeningPort {
                identity: (lookups.identity)(record.pid),
                id: format!("{}-{}", record.port, record.pid),
                needs_attention: needs_attention(&owner, is_system, record.port),
                port: record.port,
                process: record.command,
                pid: record.pid,
                user: record.user,
                cwd,
                owner,
                is_system,
            }
        })
        .collect();
    ports.sort_by_key(|p| (p.port, p.pid));
    ports
}

// MARK: - System

/// What a tool printed, and how it exited.
struct Ran {
    status: ExitStatus,
    stdout: String,
    stderr: String,
}

/// Runs a tool and collects its output, killing it once `limit` has passed.
/// The output is read on threads while the tool runs, so a long listing can
/// never fill the pipe and stall it.
fn run_tool(program: &Path, args: &[&str], limit: Duration) -> std::io::Result<Ran> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().map(drain);
    let stderr = child.stderr.take().map(drain);
    let deadline = Instant::now() + limit;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            waited => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(match waited {
                    Err(error) => error,
                    _ => std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        format!(
                            "did not respond after {}",
                            crate::util::counted(limit.as_secs() as usize, "second", "seconds")
                        ),
                    ),
                });
            }
        }
    };
    Ok(Ran {
        status,
        stdout: collected(stdout),
        stderr: collected(stderr),
    })
}

fn drain(mut pipe: impl Read + Send + 'static) -> JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        // A read error keeps what arrived before it, as `output()` would.
        let _ = pipe.read_to_end(&mut bytes);
        bytes
    })
}

fn collected(reader: Option<JoinHandle<Vec<u8>>>) -> String {
    let bytes = reader
        .and_then(|reader| reader.join().ok())
        .unwrap_or_default();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// A tool's standard output. The exit status is ignored on purpose: `lsof`
/// exits 1 when nothing listens or when one of the pids cannot be inspected,
/// and still prints everything it could read.
fn output(program: &Path, args: &[&str], limit: Duration) -> std::io::Result<String> {
    run_tool(program, args, limit).map(|ran| ran.stdout)
}

fn scan_system() -> Result<Vec<ListeningPort>, String> {
    let listing = output(
        Path::new(LSOF),
        &["-nP", "-iTCP", "-sTCP:LISTEN", "-F", "pcuLn"],
        LSOF_LIMIT,
    )
    .map_err(|e| format!("Could not run lsof: {e}"))?;
    let records = parse_listing(&listing);

    let mut pids: Vec<String> = records.iter().map(|r| r.pid.to_string()).collect();
    pids.sort();
    pids.dedup();
    let cwds = if pids.is_empty() {
        HashMap::new()
    } else {
        output(
            Path::new(LSOF),
            &["-a", "-d", "cwd", "-Fpn", "-p", &pids.join(",")],
            LSOF_LIMIT,
        )
        .map(|text| parse_cwds(&text))
        .unwrap_or_default()
    };
    // Docker is optional: without it, or with the daemon stopped, containers
    // just stay unnamed.
    let containers = docker()
        .and_then(|docker| {
            output(
                &docker,
                &["ps", "--format", "{{.Names}}\t{{.Ports}}"],
                DOCKER_PS_LIMIT,
            )
            .ok()
        })
        .map(|text| parse_containers(&text))
        .unwrap_or_default();
    let home = crate::config::home_dir();
    let lookups = Lookups {
        executable: platform::process_path,
        identity: platform::process_identity,
        project: |cwd: &str| project_name(cwd, &home),
    };
    Ok(assemble(records, &cwds, lookups, &containers))
}

fn docker() -> Option<PathBuf> {
    let home = crate::config::home_dir();
    let fallbacks: Vec<PathBuf> = DOCKER_PATHS
        .iter()
        .map(PathBuf::from)
        // Without HOME the names would be relative to the working directory.
        .chain(
            DOCKER_HOME_PATHS
                .iter()
                .filter(|_| home.is_absolute())
                .map(|rel| home.join(rel)),
        )
        .collect();
    find_executable("docker", std::env::var_os("PATH").as_deref(), &fallbacks)
}

/// The first executable `name` on `path`, then the first executable fallback.
/// PATH matters when HostSmith runs from a terminal, as in development.
fn find_executable(name: &str, path: Option<&OsStr>, fallbacks: &[PathBuf]) -> Option<PathBuf> {
    path.into_iter()
        .flat_map(std::env::split_paths)
        .filter(|dir| dir.is_absolute())
        .map(|dir| dir.join(name))
        .chain(fallbacks.iter().cloned())
        .find(|candidate| {
            std::fs::metadata(candidate)
                .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        })
}

/// Whether a pid still names the process the scan saw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Presence {
    Same,
    /// The pid now belongs to another process, or to one this user may not
    /// signal.
    Replaced,
    Gone,
}

/// Compares the identity recorded at scan time with the one the pid has now.
/// A process that cannot be identified counts as replaced while it exists,
/// since this user's own processes can always be inspected.
fn presence(
    recorded: Option<ProcessIdentity>,
    current: Option<ProcessIdentity>,
    exists: bool,
) -> Presence {
    match current {
        Some(current) if recorded == Some(current) => Presence::Same,
        Some(_) => Presence::Replaced,
        None if exists => Presence::Replaced,
        None => Presence::Gone,
    }
}

fn presence_of(target: &ListeningPort) -> Presence {
    presence(
        target.identity,
        platform::process_identity(target.pid),
        exists(target.pid),
    )
}

/// EPERM means the process exists but belongs to someone else.
fn exists(pid: i32) -> bool {
    // SAFETY: signal 0 only checks for existence and permission.
    let probed = unsafe { libc::kill(pid, 0) };
    probed == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// How sending the signal went, before checking whether the process left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Delivery {
    Sent,
    /// The process had already exited, which is what the user asked for.
    Gone,
    /// The pid no longer names the process the scan listed.
    Changed,
    Failed,
}

/// Signals the listed process as this user, never as root. `lsof` without
/// root lists only this user's processes, so a pid this user may not signal
/// has been reused by another program since the scan, and is left alone.
/// The identity is checked right before the signal; the gap between the two
/// is a few microseconds, far shorter than a scan's age.
fn deliver(target: &ListeningPort, force: bool) -> Delivery {
    match presence_of(target) {
        Presence::Same => {}
        Presence::Replaced => return Delivery::Changed,
        Presence::Gone => return Delivery::Gone,
    }
    let signal = if force { libc::SIGKILL } else { libc::SIGTERM };
    // SAFETY: plain syscall; `pid` is positive, so it never addresses a group.
    if unsafe { libc::kill(target.pid, signal) } == 0 {
        return Delivery::Sent;
    }
    match std::io::Error::last_os_error().raw_os_error() {
        Some(libc::ESRCH) => Delivery::Gone,
        Some(libc::EPERM) => Delivery::Changed,
        _ => Delivery::Failed,
    }
}

/// The outcome and the toast for a stop, once it is known whether the process
/// is still `alive` (checked only after a delivered signal).
fn conclude(
    target: &ListeningPort,
    force: bool,
    delivery: Delivery,
    alive: bool,
) -> (KillOutcome, (String, ToastKind)) {
    let process = &target.process;
    let freed = (
        KillOutcome::Freed,
        (format!("Freed port {}", target.port), ToastKind::Success),
    );
    match delivery {
        Delivery::Gone => freed,
        Delivery::Changed => (
            KillOutcome::Failed,
            (
                format!(
                    "Process {process} (pid {}) changed since the last scan. HostSmith scanned again.",
                    target.pid
                ),
                ToastKind::Info,
            ),
        ),
        Delivery::Failed => (
            KillOutcome::Failed,
            (
                format!("Could not signal {process} (pid {})", target.pid),
                ToastKind::Error,
            ),
        ),
        Delivery::Sent if alive => (
            KillOutcome::StillRunning,
            (
                format!(
                    "{process} (pid {}) is still running after {}",
                    target.pid,
                    if force { "SIGKILL" } else { "SIGTERM" }
                ),
                ToastKind::Info,
            ),
        ),
        Delivery::Sent => freed,
    }
}

#[derive(Debug, PartialEq, Eq)]
enum StopError {
    NoDocker,
    Failed(String),
}

fn stop_container(name: &str) -> Result<(), StopError> {
    let docker = docker().ok_or(StopError::NoDocker)?;
    let ran = run_tool(&docker, &["stop", name], DOCKER_STOP_LIMIT)
        .map_err(|e| StopError::Failed(e.to_string()))?;
    if ran.status.success() {
        Ok(())
    } else {
        let stderr = ran.stderr.trim().to_string();
        Err(StopError::Failed(if stderr.is_empty() {
            format!(
                "docker exited with code {}",
                ran.status.code().unwrap_or(-1)
            )
        } else {
            stderr
        }))
    }
}

fn now_ms() -> i64 {
    jiff::Timestamp::now().as_millisecond()
}

async fn rescan(app: &AppHandle) -> PortsSnapshot {
    let state = app.state::<PortsState>();
    let seq = mutate(app, &state, Ports::begin_scan);
    let result = tauri::async_runtime::spawn_blocking(scan_system)
        .await
        .unwrap_or_else(|error| Err(error.to_string()));
    mutate(app, &state, |ports| {
        ports.finish_scan(seq, result, now_ms());
        ports.snapshot()
    })
}

// MARK: - Commands

#[tauri::command]
pub fn ports_snapshot(state: State<'_, PortsState>) -> PortsSnapshot {
    state.lock().snapshot()
}

#[tauri::command]
pub async fn ports_scan(app: AppHandle) -> PortsSnapshot {
    rescan(&app).await
}

/// Sends SIGTERM, or SIGKILL when `force`, waits for the process to leave,
/// then rescans. Either signal goes only to the process the last scan listed
/// under that pid.
#[tauri::command]
pub async fn ports_kill(app: AppHandle, pid: i32, force: bool) -> Result<KillOutcome, String> {
    let state = app.state::<PortsState>();
    let target = state
        .lock()
        .listed_pid(pid)
        .filter(|p| p.pid > 0)
        .cloned()
        .ok_or_else(|| format!("PID {pid} was not in the last scan. Scan again, then retry."))?;

    let signalled = target.clone();
    let delivery = tauri::async_runtime::spawn_blocking(move || deliver(&signalled, force))
        .await
        .unwrap_or(Delivery::Failed);
    let alive = if delivery == Delivery::Sent {
        tokio::time::sleep(SETTLE).await;
        presence_of(&target) == Presence::Same
    } else {
        false
    };
    let (outcome, (message, kind)) = conclude(&target, force, delivery, alive);
    mutate(&app, &state, |p| p.show_toast(message, kind));
    rescan(&app).await;
    Ok(outcome)
}

/// Stops the container that publishes a port, by the name the last scan
/// showed, then rescans.
#[tauri::command]
pub async fn ports_stop_container(app: AppHandle, name: String) -> Result<(), String> {
    let state = app.state::<PortsState>();
    let port = state.lock().container_port(&name).ok_or_else(|| {
        format!("Container {name} was not in the last scan. Scan again, then retry.")
    })?;
    let stop_name = name.clone();
    let stopped = tauri::async_runtime::spawn_blocking(move || stop_container(&stop_name))
        .await
        .unwrap_or_else(|error| Err(StopError::Failed(error.to_string())));
    let (message, kind) = match stopped {
        Ok(()) => (format!("Stopped container {name}"), ToastKind::Success),
        Err(StopError::NoDocker) => (
            format!("Could not find the docker CLI to stop the container on port {port}"),
            ToastKind::Error,
        ),
        Err(StopError::Failed(error)) => (
            format!("Could not stop container {name}: {error}"),
            ToastKind::Error,
        ),
    };
    mutate(&app, &state, |p| p.show_toast(message, kind));
    rescan(&app).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTING: &str = include_str!("../tests/fixtures/lsof-listen.txt");
    const CWDS: &str = include_str!("../tests/fixtures/lsof-cwd.txt");
    const DOCKER_PS: &str = include_str!("../tests/fixtures/docker-ps.txt");

    /// Projects by folder prefix, standing in for the git lookup.
    fn projects(roots: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let roots: Vec<(String, String)> = roots
            .iter()
            .map(|(name, path)| (name.to_string(), path.to_string()))
            .collect();
        move |cwd: &str| {
            roots
                .iter()
                .find(|(_, path)| cwd == path || cwd.starts_with(&format!("{path}/")))
                .map(|(name, _)| name.clone())
        }
    }

    fn record(command: &str, port: u16) -> Record {
        Record {
            pid: 10,
            command: command.into(),
            user: "dev".into(),
            port,
        }
    }

    fn listed(process: &str, pid: i32, port: u16) -> ListeningPort {
        ListeningPort {
            id: format!("{port}-{pid}"),
            port,
            process: process.into(),
            pid,
            user: "dev".into(),
            cwd: None,
            owner: Owner::Unknown,
            is_system: false,
            needs_attention: true,
            identity: None,
        }
    }

    #[test]
    fn parses_the_listing_once_per_pid_and_port() {
        let records = parse_listing(LISTING);
        let pairs: Vec<(i32, u16, &str, &str)> = records
            .iter()
            .map(|r| (r.pid, r.port, r.command.as_str(), r.user.as_str()))
            .collect();
        assert_eq!(
            pairs,
            [
                (628, 7000, "ControlCenter", "batuan"),
                (628, 5000, "ControlCenter", "batuan"),
                (807, 3000, "node", "batuan"),
                (912, 5432, "com.docker.backend", "batuan"),
                // No login name, so the uid stands in.
                (1044, 5433, "postgres", "0"),
                (1188, 8000, "python3.12", "batuan"),
            ]
        );
    }

    #[test]
    fn parses_working_directories() {
        let cwds = parse_cwds(CWDS);
        assert_eq!(cwds.len(), 3);
        assert_eq!(cwds[&628], "/");
        assert_eq!(cwds[&807], "/Users/dev/code/shop/web");
    }

    #[test]
    fn parses_published_docker_ports() {
        let containers = parse_containers(DOCKER_PS);
        assert_eq!(containers[&5432], "shop-db");
        assert_eq!(containers[&6379], "shop-cache");
        assert_eq!(containers[&8080], "proxy");
        assert_eq!(containers[&8081], "proxy");
        assert_eq!(containers[&9443], "proxy");
        assert_eq!(containers.len(), 5);
    }

    #[test]
    fn attributes_to_the_project_the_process_runs_in() {
        let project = projects(&[("shop", "/Users/dev/code/shop")]);
        let none = HashMap::new();
        let node = record("node", 3000);
        assert_eq!(
            owner(&node, Some("/Users/dev/code/shop/api"), &project, &none),
            Owner::Yours("shop".into())
        );
        assert_eq!(
            owner(&node, Some("/Users/dev/code/shop-admin"), &project, &none),
            Owner::Unknown
        );
        assert_eq!(owner(&node, None, &project, &none), Owner::Unknown);
    }

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hostsmith-ports-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        // The temporary folder may sit behind a symlink (`/var` on macOS);
        // the walk compares paths as lsof reports them, resolved.
        dir.canonicalize().unwrap()
    }

    #[test]
    fn finds_the_nearest_folder_that_holds_git() {
        let tmp = temp_dir();
        let home = tmp.join("home");
        let shop = home.join("code/shop");
        let ui = shop.join("packages/ui");
        for dir in [
            home.join(".git"),
            shop.join(".git"),
            shop.join("web/src"),
            ui.join("src"),
            home.join("scratch"),
        ] {
            std::fs::create_dir_all(dir).unwrap();
        }
        // Worktrees and submodules have a `.git` file instead of a folder.
        std::fs::write(ui.join(".git"), "gitdir: ../../.git/modules/ui\n").unwrap();

        assert_eq!(project_root(&shop, &home), Some(shop.clone()));
        assert_eq!(
            project_root(&shop.join("web/src"), &home),
            Some(shop.clone())
        );
        assert_eq!(project_root(&ui.join("src"), &home), Some(ui.clone()));
        assert_eq!(
            project_name(shop.join("web/src").to_str().unwrap(), &home).as_deref(),
            Some("shop")
        );
        assert_eq!(
            project_name(ui.to_str().unwrap(), &home).as_deref(),
            Some("ui")
        );
        let gone = tmp.join("missing");
        assert_eq!(project_name(gone.to_str().unwrap(), &home), None);
        std::fs::remove_dir_all(tmp).unwrap();
    }

    #[test]
    fn the_walk_stops_before_home_and_the_root() {
        let tmp = temp_dir();
        let home = tmp.join("home");
        let outside = tmp.join("outside");
        for dir in [
            home.join(".git"),
            home.join("scratch"),
            outside.join("repo/.git"),
            outside.join("repo/sub"),
        ] {
            std::fs::create_dir_all(dir).unwrap();
        }

        // The home folder's own repository claims nothing under it.
        assert_eq!(project_root(&home.join("scratch"), &home), None);
        assert_eq!(project_root(&home, &home), None);
        // A checkout outside the home folder, such as Homebrew's, is no project.
        assert_eq!(project_root(&outside.join("repo/sub"), &home), None);
        assert_eq!(project_root(Path::new("/"), &home), None);
        assert_eq!(project_root(Path::new("relative/dir"), &home), None);
        std::fs::remove_dir_all(tmp).unwrap();
    }

    #[test]
    fn docker_processes_are_containers_named_by_port() {
        let containers = parse_containers(DOCKER_PS);
        // Docker wins even over a project.
        let project = projects(&[("shop", "/Users/dev/code/shop")]);
        assert_eq!(
            owner(
                &record("com.docker.backend", 5432),
                Some("/Users/dev/code/shop"),
                &project,
                &containers
            ),
            Owner::Container(Some("shop-db".into()))
        );
        assert_eq!(
            owner(&record("Docker", 1234), None, &project, &containers),
            Owner::Container(None)
        );
    }

    #[test]
    fn recognises_system_processes_by_path_and_by_name() {
        assert!(is_system("anything", Some("/usr/libexec/sharingd")));
        assert!(is_system(
            "x",
            Some("/System/Library/CoreServices/ControlCenter.app/Contents/MacOS/ControlCenter")
        ));
        assert!(is_system("rapportd", None));
        assert!(!is_system("node", Some("/opt/homebrew/bin/node")));
        assert!(!is_system("node", None));
        assert!(!is_system("rapportd-clone", None));
    }

    #[test]
    fn only_unknown_user_processes_on_dev_ports_need_attention() {
        assert!(needs_attention(&Owner::Unknown, false, 3000));
        assert!(needs_attention(&Owner::Unknown, false, 1024));
        assert!(needs_attention(&Owner::Unknown, false, 49151));
        assert!(!needs_attention(&Owner::Unknown, false, 1023));
        assert!(!needs_attention(&Owner::Unknown, false, 49152));
        assert!(!needs_attention(&Owner::Unknown, true, 3000));
        assert!(!needs_attention(&Owner::Yours("shop".into()), false, 3000));
        assert!(!needs_attention(&Owner::Container(None), false, 3000));
    }

    #[test]
    fn assembles_sorted_attributed_ports() {
        let records = parse_listing(LISTING);
        let cwds = parse_cwds(CWDS);
        let containers = parse_containers(DOCKER_PS);
        let identity = |pid: i32| {
            Some(ProcessIdentity {
                uid: 501,
                started: (pid as u64, 0),
            })
        };
        let lookups = Lookups {
            executable: |pid: i32| (pid == 628).then(|| "/System/Library/ControlCenter".into()),
            identity,
            project: projects(&[("web", "/Users/dev/code/shop/web")]),
        };
        let ports = assemble(records, &cwds, lookups, &containers);

        let ids: Vec<&str> = ports.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "3000-807",
                "5000-628",
                "5432-912",
                "5433-1044",
                "7000-628",
                "8000-1188"
            ]
        );
        assert_eq!(ports[0].owner, Owner::Yours("web".into()));
        assert!(!ports[0].needs_attention);
        assert!(ports[1].is_system);
        assert_eq!(ports[2].owner, Owner::Container(Some("shop-db".into())));
        assert!(ports[3].needs_attention);
        assert_eq!(ports[5].cwd.as_deref(), Some("/Users/dev/code/shop-admin"));
        assert_eq!(ports[0].identity, identity(807));
        // The identity stays on this side; the window never sees it.
        let json = serde_json::to_value(&ports[0]).unwrap();
        assert!(json.get("identity").is_none());
    }

    #[test]
    fn finds_executables_on_path_before_the_fallbacks() {
        let dir = std::env::temp_dir().join(format!("hostsmith-ports-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let tool = dir.join("docker");
        std::fs::write(&tool, "#!/bin/sh\n").unwrap();
        let path = std::env::join_paths(["relative", dir.to_str().unwrap()]).unwrap();

        // Not executable yet, so the fallback is used.
        assert_eq!(
            find_executable("docker", Some(&path), &[PathBuf::from("/bin/sh")]),
            Some(PathBuf::from("/bin/sh"))
        );
        std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(
            find_executable("docker", Some(&path), &[PathBuf::from("/bin/sh")]),
            Some(tool)
        );
        assert_eq!(
            find_executable(
                "docker",
                None,
                &[
                    PathBuf::from("/nonexistent/docker"),
                    PathBuf::from("/bin/sh")
                ]
            ),
            Some(PathBuf::from("/bin/sh"))
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn concludes_a_stop_with_a_counted_fact() {
        let target = listed("node", 807, 3000);
        let message = |delivery, alive, force| {
            let (outcome, (message, _)) = conclude(&target, force, delivery, alive);
            (outcome, message)
        };
        assert_eq!(
            message(Delivery::Sent, false, false),
            (KillOutcome::Freed, "Freed port 3000".into())
        );
        assert_eq!(
            message(Delivery::Gone, false, true),
            (KillOutcome::Freed, "Freed port 3000".into())
        );
        assert_eq!(
            message(Delivery::Sent, true, false),
            (
                KillOutcome::StillRunning,
                "node (pid 807) is still running after SIGTERM".into()
            )
        );
        assert_eq!(
            message(Delivery::Sent, true, true).1,
            "node (pid 807) is still running after SIGKILL"
        );
        assert_eq!(
            message(Delivery::Changed, false, false),
            (
                KillOutcome::Failed,
                "Process node (pid 807) changed since the last scan. HostSmith scanned again."
                    .into()
            )
        );
        assert_eq!(
            message(Delivery::Failed, false, false),
            (
                KillOutcome::Failed,
                "Could not signal node (pid 807)".into()
            )
        );
    }

    #[test]
    fn a_pid_counts_as_the_listed_process_only_with_the_same_identity() {
        let at = |second| ProcessIdentity {
            uid: 501,
            started: (second, 250),
        };
        assert_eq!(presence(Some(at(10)), Some(at(10)), true), Presence::Same);
        // Same pid, started later: another program reused it.
        assert_eq!(
            presence(Some(at(10)), Some(at(11)), true),
            Presence::Replaced
        );
        let root = ProcessIdentity { uid: 0, ..at(10) };
        assert_eq!(presence(Some(at(10)), Some(root), true), Presence::Replaced);
        // Exists but cannot be inspected: not this user's process.
        assert_eq!(presence(Some(at(10)), None, true), Presence::Replaced);
        assert_eq!(presence(Some(at(10)), None, false), Presence::Gone);
        // Without an identity from the scan nothing can be confirmed.
        assert_eq!(presence(None, Some(at(10)), true), Presence::Replaced);
    }

    #[test]
    fn signals_only_the_process_the_scan_saw() {
        let mut child = Command::new("/bin/sleep").arg("30").spawn().unwrap();
        let pid = child.id() as i32;
        let mut target = listed("sleep", pid, 3000);
        target.identity = platform::process_identity(pid);
        assert!(target.identity.is_some());

        // A pid now running something else is left alone.
        let mut stale = target.clone();
        stale.identity = Some(ProcessIdentity {
            uid: 501,
            started: (1, 0),
        });
        assert_eq!(deliver(&stale, true), Delivery::Changed);
        assert_eq!(presence_of(&target), Presence::Same);

        assert_eq!(deliver(&target, false), Delivery::Sent);
        child.wait().unwrap();
        assert_eq!(presence_of(&target), Presence::Gone);
        assert_eq!(deliver(&target, true), Delivery::Gone);
    }

    #[test]
    fn launchd_is_never_this_users_process() {
        let mut launchd = listed("launchd", 1, 3000);
        launchd.identity = Some(ProcessIdentity {
            uid: 0,
            started: (0, 0),
        });
        assert_eq!(deliver(&launchd, true), Delivery::Changed);
    }

    #[test]
    fn collects_tool_output_and_status() {
        let ran = run_tool(
            Path::new("/bin/sh"),
            &["-c", "echo out; echo err >&2; exit 3"],
            Duration::from_secs(5),
        )
        .unwrap();
        assert_eq!(ran.stdout, "out\n");
        assert_eq!(ran.stderr, "err\n");
        assert_eq!(ran.status.code(), Some(3));

        // More than a pipe holds, read while the tool still writes.
        let big = output(
            Path::new("/bin/sh"),
            &[
                "-c",
                "i=0; while [ $i -lt 20000 ]; do echo 0123456789; i=$((i+1)); done",
            ],
            Duration::from_secs(10),
        )
        .unwrap();
        assert_eq!(big.len(), 220_000);
    }

    #[test]
    fn kills_a_tool_that_does_not_finish_in_time() {
        let started = Instant::now();
        let error = run_tool(Path::new("/bin/sleep"), &["30"], Duration::from_millis(300))
            .err()
            .expect("timed out");
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
        assert!(error.to_string().starts_with("did not respond after"));
    }

    #[test]
    fn an_older_scan_never_replaces_a_newer_one() {
        let mut ports = Ports::default();
        let first = ports.begin_scan();
        let second = ports.begin_scan();
        assert!(ports.snapshot().scanning);
        ports.finish_scan(second, Ok(vec![listed("node", 1, 3000)]), 2);
        ports.finish_scan(first, Ok(vec![]), 1);
        let snapshot = ports.snapshot();
        assert!(!snapshot.scanning);
        assert_eq!(snapshot.ports.len(), 1);
        assert_eq!(snapshot.scanned_at, Some(2));
    }

    #[test]
    fn a_failed_scan_keeps_the_last_list_and_says_why() {
        let mut ports = Ports::default();
        let seq = ports.begin_scan();
        ports.finish_scan(seq, Ok(vec![listed("node", 1, 3000)]), 1);
        let seq = ports.begin_scan();
        ports.finish_scan(seq, Err("Could not run lsof: x".into()), 2);
        let snapshot = ports.snapshot();
        assert_eq!(snapshot.ports.len(), 1);
        assert_eq!(snapshot.scanned_at, Some(1));
        assert_eq!(snapshot.toast.unwrap().message, "Could not run lsof: x");
    }

    #[test]
    fn only_listed_processes_and_containers_can_be_stopped() {
        let mut ports = Ports::default();
        let mut db = listed("com.docker.backend", 912, 5432);
        db.owner = Owner::Container(Some("shop-db".into()));
        let seq = ports.begin_scan();
        ports.finish_scan(seq, Ok(vec![listed("node", 807, 3000), db]), 1);
        assert_eq!(ports.listed_pid(807).map(|p| p.port), Some(3000));
        assert!(ports.listed_pid(1).is_none());
        assert_eq!(ports.container_port("shop-db"), Some(5432));
        assert_eq!(ports.container_port("other"), None);
    }
}
