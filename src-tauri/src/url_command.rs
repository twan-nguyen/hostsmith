//! Commands other tools send through `hosven://` links:
//!
//! - `hosven://activate/<profile>` switches to an environment and writes.
//! - `hosven://deactivate` turns every environment off and writes.
//! - `hosven://env/<repo>/profile/<name>` applies a saved env profile.
//! - `hosven://open/<repo>` shows a repo.
//! - `hosven://ports` shows the ports pane.
//!
//! Names match regardless of case. A link cannot answer a dialog, so a switch
//! refuses to write edits pending in the window and says so instead.

use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

use crate::env::{self, EnvState};
use crate::hosts::{self, Hosts, HostsState, Outcome, Step, SwitchPolicy};
use crate::profiles::Profile;
use crate::toast::ToastKind;
use crate::util::lock;

const SCHEME: &str = "hosven";

/// How long the window gets to show a pane before a toast for it arrives: a
/// pane that mounts with a toast already in its snapshot treats it as seen.
const SETTLE: Duration = Duration::from_millis(200);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UrlCommand {
    Activate(String),
    Deactivate,
    EnvProfile {
        repo: String,
        profile: String,
    },
    Open(String),
    Ports,
    /// Holds the link as it arrived, to say which one was not understood.
    Unknown(String),
}

impl UrlCommand {
    pub fn parse(url: &str) -> Self {
        let unknown = || Self::Unknown(url.to_string());
        let Some((scheme, rest)) = url.split_once(':') else {
            return unknown();
        };
        if !scheme.eq_ignore_ascii_case(SCHEME) {
            return unknown();
        }
        let rest = rest.trim_start_matches('/');
        let rest = rest.split(['?', '#']).next().unwrap_or_default();
        let segments: Vec<String> = rest
            .split('/')
            .filter(|s| !s.is_empty())
            .map(percent_decode)
            .collect();
        let Some((verb, args)) = segments.split_first() else {
            return unknown();
        };
        match (verb.to_lowercase().as_str(), args) {
            ("activate", [name]) => Self::Activate(name.clone()),
            ("deactivate", []) => Self::Deactivate,
            ("env", [repo, keyword, profile]) if keyword.eq_ignore_ascii_case("profile") => {
                Self::EnvProfile {
                    repo: repo.clone(),
                    profile: profile.clone(),
                }
            }
            ("open", [repo]) => Self::Open(repo.clone()),
            ("ports", []) => Self::Ports,
            _ => unknown(),
        }
    }
}

/// Decodes `%XX` escapes. A segment that does not decode to UTF-8 is kept as
/// it arrived.
fn percent_decode(segment: &str) -> String {
    let bytes = segment.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(byte) = bytes
                .get(i + 1..i + 3)
                .and_then(|hex| std::str::from_utf8(hex).ok())
                .and_then(|hex| u8::from_str_radix(hex, 16).ok())
        {
            out.push(byte);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| segment.to_string())
}

// MARK: - Hosts actions

fn find_profile<'a>(profiles: &'a [Profile], name: &str) -> Option<&'a Profile> {
    let wanted = name.to_lowercase();
    profiles.iter().find(|p| p.name.to_lowercase() == wanted)
}

/// The profile an activate link points at, or why it cannot be switched to.
/// The profile's id comes back even for a block, so its pane can be shown.
fn resolve_activate(profiles: &[Profile], name: &str) -> (Option<Uuid>, Result<Uuid, String>) {
    match find_profile(profiles, name) {
        None => (None, Err(format!("Không có profile tên {name}"))),
        Some(p) if !p.is_exclusive => (
            Some(p.id),
            Err(format!("{} là block, không phải môi trường", p.name)),
        ),
        Some(p) => (Some(p.id), Ok(p.id)),
    }
}

fn refused_message(count: usize) -> String {
    format!("Có {count} thay đổi chưa áp dụng. Áp dụng hoặc huỷ chúng trước khi đổi môi trường.")
}

/// Switches as a link may: pending edits refuse the switch with a toast.
fn begin_switch(hosts: &mut Hosts, target: Option<Uuid>) -> Step {
    let step = hosts.switch_environment(target, SwitchPolicy::Refuse);
    if let Step::Done(Outcome::Refused(count)) = step {
        hosts.show_toast(refused_message(count), ToastKind::Error);
    }
    step
}

/// Prepares what an activate link does to the model, and the profile pane to
/// show first.
fn begin_activate(hosts: &mut Hosts, name: &str) -> (Option<Uuid>, Step) {
    let (shown, target) = resolve_activate(hosts.profiles(), name);
    let step = match target {
        Ok(id) => begin_switch(hosts, Some(id)),
        Err(message) => {
            hosts.show_toast(message.clone(), ToastKind::Error);
            Step::Done(Outcome::Failed(message))
        }
    };
    (shown, step)
}

// MARK: - Delivery

/// Links that arrive before the window can listen wait here; `None` once
/// the window is ready.
pub struct UrlQueue(Mutex<Option<Vec<String>>>);

impl Default for UrlQueue {
    fn default() -> Self {
        Self(Mutex::new(Some(Vec::new())))
    }
}

/// Takes a link from the system, running it now or once the window is ready.
pub fn receive(app: &AppHandle, url: String) {
    let state = app.state::<UrlQueue>();
    let mut queue = lock(&state.0);
    match queue.as_mut() {
        Some(waiting) => waiting.push(url),
        None => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move { run(&app, &url).await });
        }
    }
}

/// The main window calls this once it listens for `navigate`: runs the links
/// that launched or reached the app before then.
#[tauri::command]
pub fn window_ready(app: AppHandle) {
    let waiting = {
        let state = app.state::<UrlQueue>();
        let mut queue = lock(&state.0);
        queue.take().unwrap_or_default()
    };
    tauri::async_runtime::spawn(async move {
        for url in waiting {
            run(&app, &url).await;
        }
    });
}

async fn run(app: &AppHandle, url: &str) {
    crate::tray::show_main(app);
    match UrlCommand::parse(url) {
        UrlCommand::Activate(name) => {
            let (shown, step) = begin_activate(&mut app.state::<HostsState>().lock(), &name);
            if let Some(id) = shown {
                navigate(app, &format!("profile:{id}")).await;
            }
            hosts::run(app.clone(), step).await;
        }
        UrlCommand::Deactivate => {
            let step = begin_switch(&mut app.state::<HostsState>().lock(), None);
            hosts::run(app.clone(), step).await;
        }
        UrlCommand::EnvProfile { repo, profile } => {
            let Some(id) = find_repo(app, &repo) else {
                return hosts_error(app, format!("Không có repo tên {repo}"));
            };
            navigate(app, &format!("repo:{id}")).await;
            env::mutate(app, &app.state::<EnvState>(), |e| {
                if let Err(error) = e.apply_profile_by_names(&repo, &profile) {
                    e.show_toast(error, ToastKind::Error);
                }
            });
        }
        UrlCommand::Open(repo) => match find_repo(app, &repo) {
            Some(id) => navigate(app, &format!("repo:{id}")).await,
            None => hosts_error(app, format!("Không có repo tên {repo}")),
        },
        UrlCommand::Ports => navigate(app, "ports").await,
        UrlCommand::Unknown(url) => hosts_error(app, format!("Không hiểu lệnh {url}")),
    }
}

/// Shows a sidebar selection, in the format of `src/lib/selection.ts`.
async fn navigate(app: &AppHandle, selection: &str) {
    let _ = app.emit_to("main", "navigate", selection);
    tokio::time::sleep(SETTLE).await;
}

fn find_repo(app: &AppHandle, name: &str) -> Option<Uuid> {
    app.state::<EnvState>().lock().repo_id_by_name(name)
}

fn hosts_error(app: &AppHandle, message: String) {
    hosts::mutate(app, &app.state::<HostsState>(), |h| {
        h.show_toast(message, ToastKind::Error)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiles::{MemoryStore, ProfileColor};

    #[test]
    fn parses_every_route() {
        assert_eq!(
            UrlCommand::parse("hosven://activate/Staging"),
            UrlCommand::Activate("Staging".into())
        );
        assert_eq!(
            UrlCommand::parse("hosven://deactivate"),
            UrlCommand::Deactivate
        );
        assert_eq!(
            UrlCommand::parse("hosven://env/api/profile/staging"),
            UrlCommand::EnvProfile {
                repo: "api".into(),
                profile: "staging".into()
            }
        );
        assert_eq!(
            UrlCommand::parse("hosven://open/api"),
            UrlCommand::Open("api".into())
        );
        assert_eq!(UrlCommand::parse("hosven://ports"), UrlCommand::Ports);
    }

    #[test]
    fn the_verb_and_scheme_ignore_case_but_names_keep_theirs() {
        assert_eq!(
            UrlCommand::parse("HOSVEN://Activate/Staging"),
            UrlCommand::Activate("Staging".into())
        );
        assert_eq!(
            UrlCommand::parse("hosven://ENV/Api/PROFILE/Dev"),
            UrlCommand::EnvProfile {
                repo: "Api".into(),
                profile: "Dev".into()
            }
        );
        assert_eq!(UrlCommand::parse("hosven://PORTS/"), UrlCommand::Ports);
    }

    #[test]
    fn segments_are_percent_decoded() {
        assert_eq!(
            UrlCommand::parse("hosven://activate/My%20Env"),
            UrlCommand::Activate("My Env".into())
        );
        assert_eq!(
            UrlCommand::parse("hosven://env/m%E1%BB%99t/profile/a%2Fb"),
            UrlCommand::EnvProfile {
                repo: "một".into(),
                profile: "a/b".into()
            }
        );
        // Escapes that are not hex, or not UTF-8, stay as they arrived.
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("a%zzb"), "a%zzb");
        assert_eq!(percent_decode("%FF"), "%FF");
    }

    #[test]
    fn queries_fragments_and_extra_slashes_are_ignored() {
        assert_eq!(
            UrlCommand::parse("hosven:///activate//Staging/?from=raycast#x"),
            UrlCommand::Activate("Staging".into())
        );
    }

    #[test]
    fn anything_else_is_unknown() {
        for url in [
            "hosven://",
            "hosven://xyz",
            "hosven://activate",
            "hosven://activate/a/b",
            "hosven://deactivate/now",
            "hosven://env/api/staging",
            "hosven://env/api/variant/staging",
            "hosven://open",
            "hosven://ports/80",
            "https://activate/Staging",
            "not a url",
        ] {
            assert_eq!(
                UrlCommand::parse(url),
                UrlCommand::Unknown(url.into()),
                "{url}"
            );
        }
    }

    const FILE: &str = "127.0.0.1\tlocalhost
## [tag:Local]
127.0.0.1\tapi.test
## [tag:Staging]
# 10.0.0.2\tapi.test
## [tag:Tools]
127.0.0.1\ttools.test
";

    fn model() -> Hosts {
        let profiles = vec![
            Profile::new("Local", ProfileColor::Blue, true),
            Profile::new("Staging", ProfileColor::Amber, true),
            Profile::new("Tools", ProfileColor::Purple, false),
        ];
        let mut hosts = Hosts::new(Box::new(MemoryStore::with(profiles)));
        hosts.load(FILE);
        hosts
    }

    fn id(hosts: &Hosts, name: &str) -> Uuid {
        hosts.profiles().iter().find(|p| p.name == name).unwrap().id
    }

    fn toast(hosts: &Hosts) -> Option<String> {
        hosts.snapshot().toast.map(|t| t.message)
    }

    #[test]
    fn activating_an_environment_writes_the_switch() {
        let mut hosts = model();
        let staging = id(&hosts, "Staging");
        let (shown, step) = begin_activate(&mut hosts, "staging");
        assert_eq!(shown, Some(staging));
        assert!(
            matches!(step, Step::Write(_)),
            "names match regardless of case"
        );
    }

    #[test]
    fn activating_a_block_is_an_error_on_its_pane() {
        let mut hosts = model();
        let tools = id(&hosts, "Tools");
        let (shown, step) = begin_activate(&mut hosts, "Tools");
        assert_eq!(shown, Some(tools));
        let message = "Tools là block, không phải môi trường".to_string();
        assert!(matches!(step, Step::Done(Outcome::Failed(m)) if m == message));
        assert_eq!(toast(&hosts), Some(message));
    }

    #[test]
    fn activating_a_missing_profile_says_so() {
        let mut hosts = model();
        let (shown, step) = begin_activate(&mut hosts, "Prod");
        assert_eq!(shown, None);
        assert!(matches!(step, Step::Done(Outcome::Failed(_))));
        assert_eq!(toast(&hosts).as_deref(), Some("Không có profile tên Prod"));
    }

    #[test]
    fn activating_with_pending_edits_is_refused() {
        let mut hosts = model();
        hosts.set_tag_enabled("Tools", false).unwrap();
        let (_, step) = begin_activate(&mut hosts, "Staging");
        assert!(matches!(step, Step::Done(Outcome::Refused(1))));
        assert_eq!(toast(&hosts), Some(refused_message(1)));
        assert!(hosts.dirty(), "the pending edit is left as it was");
    }

    #[test]
    fn deactivating_with_pending_edits_is_refused() {
        let mut hosts = model();
        hosts.set_tag_enabled("Tools", false).unwrap();
        assert!(matches!(
            begin_switch(&mut hosts, None),
            Step::Done(Outcome::Refused(1))
        ));
    }
}
