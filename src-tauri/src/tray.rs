//! The menu bar icon and the panel that drops from it, and the app's place in
//! the Dock. With the icon on, closing the main window leaves Hosven running
//! in the menu bar only; with it off, Hosven stays in the Dock instead.

use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use tauri::image::Image;
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Window, WindowEvent};
use tauri_plugin_positioner::{Position, WindowExt};

use crate::hosts::{self, ActiveState, Hosts, HostsState, Outcome};

const TRAY_ID: &str = "hosven";
const MAIN: &str = "main";
const PANEL: &str = "panel";

/// Template images, drawn at twice the 18 pt the menu bar shows them at.
const ICON_IDLE: &[u8] = include_bytes!("../icons/tray-network@2x.png");
const ICON_ACTIVE: &[u8] = include_bytes!("../icons/tray-network-active@2x.png");

/// Clicking the icon while the panel is open first takes focus from the
/// panel, which hides it, and then delivers the click. A click this soon
/// after such a hide was meant to close the panel, not to open it again.
const REOPEN_GUARD: Duration = Duration::from_millis(250);

#[derive(Debug, Default)]
struct ClickGate {
    hidden_on_blur: Option<Instant>,
}

impl ClickGate {
    fn hidden_on_blur(&mut self, at: Instant) {
        self.hidden_on_blur = Some(at);
    }

    /// Whether a click on the icon at `at` should open the panel.
    fn should_open(&mut self, at: Instant) -> bool {
        match self.hidden_on_blur.take() {
            Some(hidden) => at.saturating_duration_since(hidden) >= REOPEN_GUARD,
            None => true,
        }
    }
}

#[derive(Default)]
struct Inner {
    /// The icon is in the menu bar.
    enabled: bool,
    /// Which image the icon shows: an environment or block is on.
    showing_active: Option<bool>,
    gate: ClickGate,
    /// The app is out of the Dock.
    accessory: bool,
}

#[derive(Default)]
pub struct TrayState(Mutex<Inner>);

impl TrayState {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        crate::util::lock(&self.0)
    }
}

fn is_active(hosts: &Hosts) -> bool {
    !matches!(hosts.active_state(), ActiveState::None)
}

/// Shows the image for the hosts state. Called with every `hosts-changed`,
/// so it only touches the icon when the image changes, and never waits for
/// the main thread: the caller holds the hosts lock.
pub fn sync(app: &AppHandle, hosts: &Hosts) {
    let Some(state) = app.try_state::<TrayState>() else {
        return;
    };
    let active = is_active(hosts);
    let mut inner = state.lock();
    if !inner.enabled || inner.showing_active == Some(active) {
        return;
    }
    inner.showing_active = Some(active);
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(tray) = handle.tray_by_id(TRAY_ID)
            && let Err(error) = tray.set_icon(Some(icon(active)))
        {
            eprintln!("hosven: could not change the menu bar icon: {error}");
        }
    });
}

fn icon(active: bool) -> Image<'static> {
    let bytes = if active { ICON_ACTIVE } else { ICON_IDLE };
    // The images are part of the binary, so they always decode.
    Image::from_bytes(bytes).expect("the menu bar icons are valid PNG")
}

/// Puts the icon in the menu bar or takes it out, following the setting.
pub fn set_enabled(app: &AppHandle, enabled: bool) {
    // Read before taking the tray lock: `sync` takes the two the other way round.
    let active = app
        .try_state::<HostsState>()
        .is_some_and(|state| is_active(&state.lock()));
    let state = app.state::<TrayState>();
    let mut inner = state.lock();
    if inner.enabled == enabled {
        return;
    }
    if enabled {
        let built = TrayIconBuilder::with_id(TRAY_ID)
            .icon(icon(active))
            .icon_as_template(true)
            .show_menu_on_left_click(false)
            .tooltip("Hosven")
            .on_tray_icon_event(on_tray_event)
            .build(app);
        match built {
            Ok(_) => {
                inner.enabled = true;
                inner.showing_active = Some(active);
            }
            Err(error) => eprintln!("hosven: could not add the menu bar icon: {error}"),
        }
    } else {
        app.remove_tray_by_id(TRAY_ID);
        inner.enabled = false;
        inner.showing_active = None;
        if let Some(panel) = app.get_webview_window(PANEL) {
            let _ = panel.hide();
        }
        // Without the icon, the Dock is the only way back to the app.
        set_accessory(app, &mut inner, false);
    }
}

fn set_accessory(app: &AppHandle, inner: &mut Inner, accessory: bool) {
    if inner.accessory == accessory {
        return;
    }
    #[cfg(target_os = "macos")]
    {
        let policy = if accessory {
            tauri::ActivationPolicy::Accessory
        } else {
            tauri::ActivationPolicy::Regular
        };
        if let Err(error) = app.set_activation_policy(policy) {
            eprintln!("hosven: could not change the Dock presence: {error}");
            return;
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = app;
    inner.accessory = accessory;
}

fn on_tray_event(tray: &TrayIcon, event: TrayIconEvent) {
    let app = tray.app_handle();
    // Records where the icon is, which the panel's position is worked out from.
    tauri_plugin_positioner::on_tray_event(app, &event);
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        toggle_panel(app);
    }
}

fn toggle_panel(app: &AppHandle) {
    let Some(panel) = app.get_webview_window(PANEL) else {
        return;
    };
    if panel.is_visible().unwrap_or(false) {
        let _ = panel.hide();
        return;
    }
    if !app
        .state::<TrayState>()
        .lock()
        .gate
        .should_open(Instant::now())
    {
        return;
    }
    if let Err(error) = panel.move_window_constrained(Position::TrayCenter) {
        eprintln!("hosven: could not place the panel: {error}");
    }
    let _ = panel.show();
    let _ = panel.set_focus();
}

/// Brings the main window back, and the app into the Dock with it.
pub fn show_main(app: &AppHandle) {
    if let Some(panel) = app.get_webview_window(PANEL) {
        let _ = panel.hide();
    }
    set_accessory(app, &mut app.state::<TrayState>().lock(), false);
    crate::menu::show_main(app);
}

/// Window behaviour the app needs: the panel hides when it loses focus, and
/// closing the main window hides it, leaving the app in the menu bar.
pub fn on_window_event(window: &Window, event: &WindowEvent) {
    let app = window.app_handle();
    match (window.label(), event) {
        (PANEL, WindowEvent::Focused(false)) => {
            if window.is_visible().unwrap_or(false) {
                let _ = window.hide();
                app.state::<TrayState>()
                    .lock()
                    .gate
                    .hidden_on_blur(Instant::now());
            }
        }
        (MAIN, WindowEvent::CloseRequested { api, .. }) => {
            api.prevent_close();
            let _ = window.hide();
            let state = app.state::<TrayState>();
            let mut inner = state.lock();
            if inner.enabled {
                set_accessory(app, &mut inner, true);
            }
        }
        // Shown some other way, such as from the app menu while the panel
        // had focus: the window belongs in the Dock.
        (MAIN, WindowEvent::Focused(true)) => {
            let state = app.state::<TrayState>();
            let mut inner = state.lock();
            if inner.accessory {
                set_accessory(app, &mut inner, false);
                // The menu bar can stay stale after the switch until the
                // app is activated again.
                let _ = window.set_focus();
            }
        }
        _ => {}
    }
}

/// Whether the app keeps running with no window open.
pub fn keeps_running(app: &AppHandle) -> bool {
    app.try_state::<TrayState>()
        .is_some_and(|state| state.lock().enabled)
}

// MARK: - Panel commands

#[tauri::command]
pub async fn panel_toggle_block(app: AppHandle, name: String, enabled: bool) -> Outcome {
    let step = app
        .state::<HostsState>()
        .lock()
        .begin_block_write(&name, enabled);
    hosts::run(app.clone(), step).await
}

/// Shows the main window, on `selection` when given (the sidebar's format).
#[tauri::command]
pub fn panel_open_main(app: AppHandle, selection: Option<String>) {
    show_main(&app);
    if let Some(selection) = selection {
        let _ = app.emit_to(MAIN, "navigate", selection);
    }
}

#[tauri::command]
pub fn panel_hide(app: AppHandle) {
    if let Some(panel) = app.get_webview_window(PANEL) {
        let _ = panel.hide();
    }
}

#[tauri::command]
pub fn panel_quit(app: AppHandle) {
    app.exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hosts::Step;
    use crate::platform::RunOutcome;
    use crate::profiles::{MemoryStore, Profile, ProfileColor};

    #[test]
    fn a_click_right_after_a_blur_hide_does_not_reopen() {
        let mut gate = ClickGate::default();
        let hidden = Instant::now();
        gate.hidden_on_blur(hidden);
        assert!(!gate.should_open(hidden + Duration::from_millis(120)));
    }

    #[test]
    fn a_later_click_opens() {
        let mut gate = ClickGate::default();
        assert!(gate.should_open(Instant::now()), "nothing was hidden");
        let hidden = Instant::now();
        gate.hidden_on_blur(hidden);
        assert!(gate.should_open(hidden + REOPEN_GUARD));
    }

    #[test]
    fn the_guard_swallows_one_click_only() {
        let mut gate = ClickGate::default();
        let hidden = Instant::now();
        gate.hidden_on_blur(hidden);
        assert!(!gate.should_open(hidden + Duration::from_millis(50)));
        assert!(gate.should_open(hidden + Duration::from_millis(100)));
    }

    const FILE: &str = "127.0.0.1\tlocalhost
## [tag:Tools]
127.0.0.1\ttools.test
## [tag:Mocks]
# 127.0.0.1\tmock.test
";

    fn model() -> Hosts {
        let profiles = vec![
            Profile::new("Tools", ProfileColor::Purple, false),
            Profile::new("Mocks", ProfileColor::Purple, false),
        ];
        let mut hosts = Hosts::new(Box::new(MemoryStore::with(profiles)));
        hosts.load(FILE);
        hosts
    }

    fn finish(hosts: &mut Hosts, step: Step, written: RunOutcome) -> Outcome {
        match step {
            Step::Write(job) => hosts.finish_write(job, written),
            Step::Done(outcome) => outcome,
        }
    }

    #[test]
    fn a_flip_writes_at_once_as_one_undo_step() {
        let mut hosts = model();
        let step = hosts.begin_block_write("Mocks", true);
        assert_eq!(finish(&mut hosts, step, RunOutcome::Ok), Outcome::Written);
        assert!(!hosts.dirty());
        assert!(hosts.serialize().contains("\n127.0.0.1\tmock.test"));
        assert!(hosts.snapshot().can_undo);
    }

    #[test]
    fn a_flip_that_does_not_land_leaves_the_model_and_history_alone() {
        for written in [RunOutcome::Cancelled, RunOutcome::Denied(-60005)] {
            let mut hosts = model();
            let before = hosts.serialize();
            let step = hosts.begin_block_write("Tools", false);
            assert_ne!(finish(&mut hosts, step, written), Outcome::Written);
            assert_eq!(hosts.serialize(), before);
            assert!(!hosts.dirty());
            let snapshot = hosts.snapshot();
            assert!(!snapshot.can_undo && !snapshot.can_redo);
        }
    }

    #[test]
    fn pending_edits_in_the_window_refuse_a_flip() {
        let mut hosts = model();
        hosts.set_tag_enabled("Mocks", true).unwrap();
        let before = hosts.serialize();
        assert!(matches!(
            hosts.begin_block_write("Tools", false),
            Step::Done(Outcome::Refused(1))
        ));
        assert_eq!(hosts.serialize(), before, "nothing was flipped");
    }

    #[test]
    fn a_flip_to_the_state_the_block_has_does_nothing() {
        let mut hosts = model();
        assert!(matches!(
            hosts.begin_block_write("Tools", true),
            Step::Done(Outcome::NothingToDo)
        ));
        assert!(matches!(
            hosts.begin_block_write("Nope", true),
            Step::Done(Outcome::NothingToDo)
        ));
        assert!(!hosts.snapshot().can_undo);
    }
}
