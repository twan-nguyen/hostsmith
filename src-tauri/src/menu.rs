//! The native menu bar. The Profile menu lists the environments with ⌘1–⌘9,
//! so it is rebuilt whenever the list of environments changes.
//!
//! Undo and redo are custom items rather than the predefined ones: the
//! predefined items send text undo to the webview, which would never reach
//! the hosts model. The window decides which undo the user meant.

use std::sync::Mutex;

use tauri::menu::{AboutMetadata, Menu, MenuEvent, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_opener::OpenerExt;
use uuid::Uuid;

use crate::hosts::{self, Hosts, SwitchPolicy};
use crate::util::lock;

const HOMEPAGE: &str = "https://github.com/twan-nguyen/hosven";
const RELEASES: &str = "https://github.com/twan-nguyen/hosven/releases";

/// The menu actions the window carries out, sent as `menu` events.
const WINDOW_ACTIONS: [&str; 5] = ["undo", "redo", "find", "toggle-sidebar", "settings"];

/// What the Profile menu shows; the menu is rebuilt only when this changes.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ProfileItem {
    id: Uuid,
    name: String,
    shortcut: Option<u32>,
    has_entries: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Shape {
    environments: Vec<ProfileItem>,
    any_active: bool,
}

impl Shape {
    fn of(hosts: &Hosts) -> Self {
        Self {
            environments: hosts
                .profiles()
                .iter()
                .filter(|p| p.is_exclusive)
                .map(|p| ProfileItem {
                    id: p.id,
                    name: p.name.clone(),
                    shortcut: p.shortcut_number.filter(|n| (1..=9).contains(n)),
                    has_entries: hosts.tag_entry_count(&p.name) > 0,
                })
                .collect(),
            any_active: !matches!(hosts.active_state(), hosts::ActiveState::None),
        }
    }
}

#[derive(Default)]
pub struct MenuState(Mutex<Option<Shape>>);

/// Rebuilds the menu when the environments it lists have changed.
///
/// Called with the hosts model locked. From another thread the build is
/// posted to the main thread rather than waited for, since a command running
/// there may be waiting for that same lock; on the main thread it runs inline.
pub fn sync(app: &AppHandle, hosts: &Hosts) {
    let Some(state) = app.try_state::<MenuState>() else {
        return;
    };
    let shape = Shape::of(hosts);
    {
        let mut last = lock(&state.0);
        if last.as_ref() == Some(&shape) {
            return;
        }
        *last = Some(shape.clone());
    }
    let handle = app.clone();
    let posted = app.run_on_main_thread(move || {
        if let Err(error) = build(&handle, &shape).and_then(|menu| handle.set_menu(menu)) {
            eprintln!("hosven: could not build the menu: {error}");
            // Lets the next change try again.
            if let Some(state) = handle.try_state::<MenuState>() {
                *lock(&state.0) = None;
            }
        }
    });
    if let Err(error) = posted {
        eprintln!("hosven: could not schedule the menu: {error}");
    }
}

fn build(app: &AppHandle, shape: &Shape) -> tauri::Result<Menu<Wry>> {
    let item = |id: &str, text: &str, accelerator: Option<&str>| {
        let mut builder = MenuItemBuilder::with_id(id, text);
        if let Some(accelerator) = accelerator {
            builder = builder.accelerator(accelerator);
        }
        builder.build(app)
    };

    let about = AboutMetadata {
        name: Some("Hosven".into()),
        version: Some(app.package_info().version.to_string()),
        website: Some(HOMEPAGE.into()),
        ..Default::default()
    };
    let app_menu = SubmenuBuilder::new(app, "Hosven")
        .about_with_text("Giới thiệu Hosven", Some(about))
        .item(&item("check-updates", "Kiểm tra cập nhật…", None)?)
        .item(&item("feedback", "Gửi phản hồi", None)?)
        .item(&item("homepage", "Trang chủ dự án", None)?)
        .separator()
        .item(&item("settings", "Cài đặt…", Some("CmdOrCtrl+,"))?)
        .separator()
        .hide_with_text("Ẩn Hosven")
        .hide_others_with_text("Ẩn ứng dụng khác")
        .show_all_with_text("Hiện tất cả")
        .separator()
        .quit_with_text("Thoát Hosven")
        .build()?;

    let edit = SubmenuBuilder::new(app, "Sửa")
        .item(&item("undo", "Hoàn tác", Some("CmdOrCtrl+Z"))?)
        .item(&item("redo", "Làm lại", Some("CmdOrCtrl+Shift+Z"))?)
        .separator()
        .cut_with_text("Cắt")
        .copy_with_text("Sao chép")
        .paste_with_text("Dán")
        .select_all_with_text("Chọn tất cả")
        .separator()
        .item(&item("find", "Tìm kiếm", Some("CmdOrCtrl+F"))?)
        .build()?;

    let view = SubmenuBuilder::new(app, "Hiển thị")
        .item(&item(
            "toggle-sidebar",
            "Ẩn hoặc hiện thanh bên",
            Some("Ctrl+CmdOrCtrl+S"),
        )?)
        .build()?;

    let mut profile = SubmenuBuilder::new(app, "Profile");
    for env in &shape.environments {
        let accelerator = env.shortcut.map(|n| format!("CmdOrCtrl+{n}"));
        let entry = MenuItemBuilder::with_id(
            format!("switch:{}", env.id),
            format!("Chuyển sang {}", env.name),
        )
        .enabled(env.has_entries);
        let entry = match accelerator {
            Some(accelerator) => entry.accelerator(accelerator),
            None => entry,
        };
        profile = profile.item(&entry.build(app)?);
    }
    if !shape.environments.is_empty() {
        profile = profile.separator();
    }
    let deactivate = MenuItemBuilder::with_id("deactivate", "Tắt môi trường")
        .accelerator("CmdOrCtrl+0")
        .enabled(shape.any_active)
        .build(app)?;
    let profile = profile.item(&deactivate).build()?;

    let window = SubmenuBuilder::new(app, "Cửa sổ")
        .minimize_with_text("Thu nhỏ")
        .maximize_with_text("Phóng to")
        .separator()
        .close_window_with_text("Đóng cửa sổ")
        .build()?;

    Menu::with_items(app, &[&app_menu, &edit, &view, &profile, &window])
}

pub fn handle(app: &AppHandle, event: MenuEvent) {
    let id = event.id().as_ref();
    if WINDOW_ACTIONS.contains(&id) {
        let _ = app.emit("menu", id);
        if id == "settings" {
            show_main(app);
        }
        return;
    }
    match id {
        "check-updates" => open(app, RELEASES),
        "homepage" => open(app, HOMEPAGE),
        "feedback" => open(app, &feedback_url(&app.package_info().version.to_string())),
        "deactivate" => switch(app, None),
        _ => {
            if let Some(target) = id
                .strip_prefix("switch:")
                .and_then(|s| Uuid::parse_str(s).ok())
            {
                switch(app, Some(target));
            }
        }
    }
}

/// The menu acts for the window, so pending edits are asked about there.
fn switch(app: &AppHandle, target: Option<Uuid>) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Some(id) = target {
            let _ = app.emit("navigate", format!("profile:{id}"));
        }
        hosts::hosts_switch_environment(app, target, SwitchPolicy::Ask).await;
    });
}

pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn open(app: &AppHandle, url: &str) {
    if let Err(error) = app.opener().open_url(url, None::<&str>) {
        eprintln!("hosven: could not open {url}: {error}");
    }
}

/// A new GitHub issue with the version filled in, as Hosven 1.x opened it.
fn feedback_url(version: &str) -> String {
    let body = format!(
        "## Feedback\n\n\n---\nHosven {version}\nmacOS {}\n",
        macos_version()
    );
    format!(
        "{HOMEPAGE}/issues/new?title={}&body={}",
        percent_encode("Feedback: "),
        percent_encode(&body)
    )
}

fn macos_version() -> String {
    std::process::Command::new("/usr/bin/sw_vers")
        .arg("-productVersion")
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|v| v.trim().to_string())
        .unwrap_or_default()
}

fn percent_encode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_query_values() {
        assert_eq!(percent_encode("Feedback: "), "Feedback%3A%20");
        assert_eq!(percent_encode("a\nb#ế"), "a%0Ab%23%E1%BA%BF");
    }

    #[test]
    fn the_feedback_issue_carries_the_version() {
        let url = feedback_url("2.0.0");
        assert!(url.starts_with(
            "https://github.com/twan-nguyen/hosven/issues/new?title=Feedback%3A%20&body="
        ));
        assert!(url.contains("Hosven%202.0.0"));
    }
}
