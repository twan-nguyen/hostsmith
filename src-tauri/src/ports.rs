//! Listening TCP ports: who holds each one, and stopping it. Scans shell out
//! to `lsof` and `docker` like the Swift app did; parsing and attribution are
//! pure, so they run against fixtures in unit tests. Every change sends the
//! whole snapshot as `ports-changed`.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::platform::{self, PrivilegedRunner, RunOutcome, SharedRunner};

const EVENT: &str = "ports-changed";
const LSOF: &str = "/usr/sbin/lsof";
/// Where Docker Desktop and Homebrew put the CLI, for when PATH lacks it.
const DOCKER_FALLBACKS: [&str; 3] = [
    "/usr/local/bin/docker",
    "/opt/homebrew/bin/docker",
    "/Applications/Docker.app/Contents/Resources/bin/docker",
];
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

// MARK: - Model

/// A repo Hosven knows, sent by the window, so this module does not depend on
/// the env module.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownRepo {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "name", rename_all = "camelCase")]
pub enum Owner {
    /// Started from a repo Hosven knows; carries the repo name.
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ToastKind {
    Success,
    Error,
    Info,
}

/// A counted fact for the pane's banner slot; `id` grows with every toast.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Toast {
    pub id: u64,
    pub message: String,
    pub kind: ToastKind,
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
    Cancelled,
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
    toast: Option<Toast>,
    toast_seq: u64,
}

impl Ports {
    pub fn snapshot(&self) -> PortsSnapshot {
        PortsSnapshot {
            ports: self.ports.clone(),
            scanned_at: self.scanned_at,
            scanning: self.applied < self.started,
            toast: self.toast.clone(),
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
        self.toast_seq += 1;
        self.toast = Some(Toast {
            id: self.toast_seq,
            message,
            kind,
        });
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
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
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

/// Docker first, then the most specific known repo the process runs in.
fn owner(
    record: &Record,
    cwd: Option<&str>,
    repos: &[KnownRepo],
    containers: &HashMap<u16, String>,
) -> Owner {
    if record.command.to_lowercase().contains("docker") {
        return Owner::Container(containers.get(&record.port).cloned());
    }
    let Some(cwd) = cwd else {
        return Owner::Unknown;
    };
    repos
        .iter()
        .filter_map(|repo| {
            let path = repo.path.trim_end_matches('/');
            // A repo at `/` would claim every process.
            let inside = !path.is_empty()
                && (cwd == path
                    || cwd
                        .strip_prefix(path)
                        .is_some_and(|rest| rest.starts_with('/')));
            inside.then_some((path.len(), &repo.name))
        })
        .max_by_key(|(length, _)| *length)
        .map_or(Owner::Unknown, |(_, name)| Owner::Yours(name.clone()))
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

fn assemble(
    records: Vec<Record>,
    cwds: &HashMap<i32, String>,
    executable: impl Fn(i32) -> Option<String>,
    repos: &[KnownRepo],
    containers: &HashMap<u16, String>,
) -> Vec<ListeningPort> {
    let mut ports: Vec<ListeningPort> = records
        .into_iter()
        .map(|record| {
            let cwd = cwds.get(&record.pid).cloned();
            let owner = owner(&record, cwd.as_deref(), repos, containers);
            let is_system = is_system(&record.command, executable(record.pid).as_deref());
            ListeningPort {
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

/// A tool's standard output. The exit status is ignored on purpose: `lsof`
/// exits 1 when nothing listens or when one of the pids cannot be inspected,
/// and still prints everything it could read.
fn output(program: &Path, args: &[&str]) -> std::io::Result<String> {
    let out = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn scan_system(repos: &[KnownRepo]) -> Result<Vec<ListeningPort>, String> {
    let listing = output(
        Path::new(LSOF),
        &["-nP", "-iTCP", "-sTCP:LISTEN", "-F", "pcuLn"],
    )
    .map_err(|e| format!("Không chạy được lsof: {e}"))?;
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
        )
        .map(|text| parse_cwds(&text))
        .unwrap_or_default()
    };
    // Docker is optional: without it, or with the daemon stopped, containers
    // just stay unnamed.
    let containers = docker()
        .and_then(|docker| output(&docker, &["ps", "--format", "{{.Names}}\t{{.Ports}}"]).ok())
        .map(|text| parse_containers(&text))
        .unwrap_or_default();
    Ok(assemble(
        records,
        &cwds,
        platform::process_path,
        repos,
        &containers,
    ))
}

fn docker() -> Option<PathBuf> {
    find_executable(
        "docker",
        std::env::var_os("PATH").as_deref(),
        &DOCKER_FALLBACKS,
    )
}

/// The first executable `name` on `path`, then the first executable fallback.
fn find_executable(name: &str, path: Option<&OsStr>, fallbacks: &[&str]) -> Option<PathBuf> {
    path.into_iter()
        .flat_map(std::env::split_paths)
        .filter(|dir| dir.is_absolute())
        .map(|dir| dir.join(name))
        .chain(fallbacks.iter().map(PathBuf::from))
        .find(|candidate| {
            std::fs::metadata(candidate)
                .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        })
}

/// How sending the signal went, before checking whether the process left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Delivery {
    Sent,
    Cancelled,
    Denied,
    Failed,
}

fn kill_script(pid: i32, force: bool) -> String {
    format!("kill -{} {pid}", if force { "KILL" } else { "TERM" })
}

/// Signals in-process first; a process of another user (EPERM) is signalled
/// as root through the shared runner, under the same cached rights.
fn deliver(pid: i32, force: bool, runner: &dyn PrivilegedRunner) -> Delivery {
    let signal = if force { libc::SIGKILL } else { libc::SIGTERM };
    // SAFETY: plain syscall; `pid` is positive, so it never addresses a group.
    if unsafe { libc::kill(pid, signal) } == 0 {
        return Delivery::Sent;
    }
    match std::io::Error::last_os_error().raw_os_error() {
        // Gone already, which is what the user asked for.
        Some(libc::ESRCH) => Delivery::Sent,
        Some(libc::EPERM) => match runner.run(&kill_script(pid, force)) {
            RunOutcome::Ok => Delivery::Sent,
            RunOutcome::Cancelled => Delivery::Cancelled,
            RunOutcome::Denied(_) => Delivery::Denied,
            RunOutcome::Failed { .. } => Delivery::Failed,
        },
        _ => Delivery::Failed,
    }
}

/// EPERM means the process exists but belongs to someone else, so it counts
/// as alive.
fn is_alive(pid: i32) -> bool {
    // SAFETY: signal 0 only checks for existence and permission.
    let probed = unsafe { libc::kill(pid, 0) };
    probed == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// The outcome and the toast for a stop, once it is known whether the process
/// is still `alive` (checked only after a delivered signal).
fn conclude(
    target: &ListeningPort,
    force: bool,
    delivery: Delivery,
    alive: bool,
) -> (KillOutcome, Option<(String, ToastKind)>) {
    let process = &target.process;
    match delivery {
        Delivery::Cancelled => (KillOutcome::Cancelled, None),
        Delivery::Denied => (
            KillOutcome::Failed,
            Some((
                format!("Không dừng được {process} vì quyền quản trị bị từ chối"),
                ToastKind::Error,
            )),
        ),
        Delivery::Failed => (
            KillOutcome::Failed,
            Some((
                format!("Không gửi được tín hiệu tới {process} (pid {})", target.pid),
                ToastKind::Error,
            )),
        ),
        Delivery::Sent if alive => (
            KillOutcome::StillRunning,
            Some((
                format!(
                    "{process} (pid {}) vẫn chạy sau {}",
                    target.pid,
                    if force { "SIGKILL" } else { "SIGTERM" }
                ),
                ToastKind::Info,
            )),
        ),
        Delivery::Sent => (
            KillOutcome::Freed,
            Some((
                format!("Đã giải phóng cổng {}", target.port),
                ToastKind::Success,
            )),
        ),
    }
}

#[derive(Debug, PartialEq, Eq)]
enum StopError {
    NoDocker,
    Failed(String),
}

fn stop_container(name: &str) -> Result<(), StopError> {
    let docker = docker().ok_or(StopError::NoDocker)?;
    let out = Command::new(docker)
        .args(["stop", name])
        .stdin(Stdio::null())
        .output()
        .map_err(|e| StopError::Failed(e.to_string()))?;
    if out.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(StopError::Failed(if stderr.is_empty() {
            format!("docker thoát với mã {}", out.status.code().unwrap_or(-1))
        } else {
            stderr
        }))
    }
}

fn now_ms() -> i64 {
    jiff::Timestamp::now().as_millisecond()
}

async fn rescan(app: &AppHandle, repos: Vec<KnownRepo>) -> PortsSnapshot {
    let state = app.state::<PortsState>();
    let seq = mutate(app, &state, Ports::begin_scan);
    let result = tauri::async_runtime::spawn_blocking(move || scan_system(&repos))
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
pub async fn ports_scan(app: AppHandle, repos: Vec<KnownRepo>) -> PortsSnapshot {
    rescan(&app, repos).await
}

/// Sends SIGTERM, or SIGKILL when `force`, waits for the process to leave,
/// then rescans.
#[tauri::command]
pub async fn ports_kill(
    app: AppHandle,
    pid: i32,
    force: bool,
    repos: Vec<KnownRepo>,
) -> Result<KillOutcome, String> {
    let state = app.state::<PortsState>();
    let target = state
        .lock()
        .listed_pid(pid)
        .filter(|p| p.pid > 0)
        .cloned()
        .ok_or_else(|| {
            format!("Không thấy pid {pid} trong lần quét gần nhất. Quét lại rồi thử.")
        })?;
    let runner = app
        .try_state::<SharedRunner>()
        .ok_or("Hosven chưa sẵn sàng dừng tiến trình")?
        .inner()
        .clone();

    let delivery =
        tauri::async_runtime::spawn_blocking(move || deliver(pid, force, runner.as_ref()))
            .await
            .unwrap_or(Delivery::Failed);
    let alive = if delivery == Delivery::Sent {
        tokio::time::sleep(SETTLE).await;
        is_alive(pid)
    } else {
        false
    };
    let (outcome, toast) = conclude(&target, force, delivery, alive);
    if let Some((message, kind)) = toast {
        mutate(&app, &state, |p| p.show_toast(message, kind));
    }
    rescan(&app, repos).await;
    Ok(outcome)
}

/// Stops the container that publishes a port, by the name the last scan
/// showed, then rescans.
#[tauri::command]
pub async fn ports_stop_container(
    app: AppHandle,
    name: String,
    repos: Vec<KnownRepo>,
) -> Result<(), String> {
    let state = app.state::<PortsState>();
    let port = state.lock().container_port(&name).ok_or_else(|| {
        format!("Không thấy container {name} trong lần quét gần nhất. Quét lại rồi thử.")
    })?;
    let stop_name = name.clone();
    let stopped = tauri::async_runtime::spawn_blocking(move || stop_container(&stop_name))
        .await
        .unwrap_or_else(|error| Err(StopError::Failed(error.to_string())));
    let (message, kind) = match stopped {
        Ok(()) => (format!("Đã dừng container {name}"), ToastKind::Success),
        Err(StopError::NoDocker) => (
            format!("Không tìm thấy docker CLI để dừng container trên cổng {port}"),
            ToastKind::Error,
        ),
        Err(StopError::Failed(error)) => (
            format!("Không dừng được container {name}: {error}"),
            ToastKind::Error,
        ),
    };
    mutate(&app, &state, |p| p.show_toast(message, kind));
    rescan(&app, repos).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTING: &str = include_str!("../tests/fixtures/lsof-listen.txt");
    const CWDS: &str = include_str!("../tests/fixtures/lsof-cwd.txt");
    const DOCKER_PS: &str = include_str!("../tests/fixtures/docker-ps.txt");

    fn repo(name: &str, path: &str) -> KnownRepo {
        KnownRepo {
            name: name.into(),
            path: path.into(),
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
    fn attributes_to_the_repo_the_process_runs_in() {
        let repos = [
            repo("shop", "/Users/dev/code/shop"),
            repo("web", "/Users/dev/code/shop/web/"),
        ];
        let none = HashMap::new();
        let node = record("node", 3000);
        let at = |cwd: &str| owner(&node, Some(cwd), &repos, &none);
        assert_eq!(at("/Users/dev/code/shop"), Owner::Yours("shop".into()));
        assert_eq!(at("/Users/dev/code/shop/api"), Owner::Yours("shop".into()));
        // The most specific repo wins, whatever the order.
        assert_eq!(
            at("/Users/dev/code/shop/web/src"),
            Owner::Yours("web".into())
        );
        // A sibling that only shares a prefix is not inside.
        assert_eq!(at("/Users/dev/code/shop-admin"), Owner::Unknown);
        assert_eq!(owner(&node, None, &repos, &none), Owner::Unknown);
        let root = [repo("everything", "/")];
        assert_eq!(owner(&node, Some("/tmp"), &root, &none), Owner::Unknown);
    }

    #[test]
    fn docker_processes_are_containers_named_by_port() {
        let containers = parse_containers(DOCKER_PS);
        let repos = [repo("shop", "/")];
        assert_eq!(
            owner(
                &record("com.docker.backend", 5432),
                Some("/"),
                &repos,
                &containers
            ),
            Owner::Container(Some("shop-db".into()))
        );
        assert_eq!(
            owner(&record("Docker", 1234), None, &repos, &containers),
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
        let repos = [repo("web", "/Users/dev/code/shop/web")];
        let executable = |pid: i32| (pid == 628).then(|| "/System/Library/ControlCenter".into());
        let ports = assemble(records, &cwds, executable, &repos, &containers);

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
    }

    #[test]
    fn finds_executables_on_path_before_the_fallbacks() {
        let dir = std::env::temp_dir().join(format!("hosven-ports-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let tool = dir.join("docker");
        std::fs::write(&tool, "#!/bin/sh\n").unwrap();
        let path = std::env::join_paths(["relative", dir.to_str().unwrap()]).unwrap();

        // Not executable yet, so the fallback is used.
        assert_eq!(
            find_executable("docker", Some(&path), &["/bin/sh"]),
            Some(PathBuf::from("/bin/sh"))
        );
        std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(
            find_executable("docker", Some(&path), &["/bin/sh"]),
            Some(tool)
        );
        assert_eq!(
            find_executable("docker", None, &["/nonexistent/docker", "/bin/sh"]),
            Some(PathBuf::from("/bin/sh"))
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn builds_kill_scripts_from_numbers_only() {
        assert_eq!(kill_script(4242, false), "kill -TERM 4242");
        assert_eq!(kill_script(4242, true), "kill -KILL 4242");
    }

    #[test]
    fn concludes_a_stop_with_a_counted_fact() {
        let target = listed("node", 807, 3000);
        let message = |delivery, alive, force| {
            let (outcome, toast) = conclude(&target, force, delivery, alive);
            (outcome, toast.map(|(m, _)| m))
        };
        assert_eq!(
            message(Delivery::Sent, false, false),
            (KillOutcome::Freed, Some("Đã giải phóng cổng 3000".into()))
        );
        assert_eq!(
            message(Delivery::Sent, true, false),
            (
                KillOutcome::StillRunning,
                Some("node (pid 807) vẫn chạy sau SIGTERM".into())
            )
        );
        assert_eq!(
            message(Delivery::Sent, true, true).1.as_deref(),
            Some("node (pid 807) vẫn chạy sau SIGKILL")
        );
        assert_eq!(
            message(Delivery::Denied, false, false),
            (
                KillOutcome::Failed,
                Some("Không dừng được node vì quyền quản trị bị từ chối".into())
            )
        );
        assert_eq!(
            message(Delivery::Failed, false, false),
            (
                KillOutcome::Failed,
                Some("Không gửi được tín hiệu tới node (pid 807)".into())
            )
        );
        assert_eq!(
            message(Delivery::Cancelled, false, false),
            (KillOutcome::Cancelled, None)
        );
    }

    #[test]
    fn a_process_that_exists_is_alive() {
        assert!(is_alive(std::process::id() as i32));
        // launchd belongs to root: EPERM still means it exists.
        assert!(is_alive(1));
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
        ports.finish_scan(seq, Err("Không chạy được lsof: x".into()), 2);
        let snapshot = ports.snapshot();
        assert_eq!(snapshot.ports.len(), 1);
        assert_eq!(snapshot.scanned_at, Some(1));
        assert_eq!(snapshot.toast.unwrap().message, "Không chạy được lsof: x");
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
