use std::sync::Arc;

use tauri::webview::PageLoadEvent;
use tauri::{Manager, RunEvent};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_window_state::StateFlags;

mod appearance;
mod config;
mod env;
mod external_tools;
mod git_exposure;
mod hosts;
mod hosts_writer;
mod login_path;
mod menu;
mod platform;
mod ports;
mod profiles;
mod settings;
mod toast;
mod tray;
mod url_command;
mod watcher;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    login_path::adopt();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_positioner::init())
        .plugin(
            // The panel always opens under the menu bar icon.
            tauri_plugin_window_state::Builder::new()
                .with_denylist(&["panel"])
                .with_state_flags(StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED)
                .build(),
        )
        .manage(menu::MenuState::default())
        .manage(tray::TrayState::default())
        .manage(url_command::UrlQueue::default())
        .on_menu_event(menu::handle)
        .on_window_event(tray::on_window_event)
        .on_page_load(|webview, payload| {
            if webview.label() == "main" && payload.event() == PageLoadEvent::Finished {
                url_command::window_loaded(webview.app_handle());
            }
        })
        .setup(|app| {
            let settings = settings::SettingsState::load();
            settings::apply(app.handle(), &settings.get())?;
            let show_tray = settings.get().show_menu_bar_extra;
            app.manage(settings);
            let runner: platform::SharedRunner = Arc::new(platform::SystemRunner::new());
            app.manage(hosts::HostsServices {
                writer: Arc::new(hosts_writer::SystemHostsWriter::new(runner.clone())),
                resolve: platform::resolve_host,
            });
            app.manage(runner);
            app.manage(ports::PortsState::new());
            // Looks the tools up now, so the first menu that names them does
            // not wait for the lookup.
            tauri::async_runtime::spawn_blocking(|| {
                external_tools::detected();
            });
            app.manage(hosts::HostsState::load());
            menu::sync(app.handle(), &app.state::<hosts::HostsState>().lock());
            app.manage(env::EnvState::load());
            tray::set_enabled(app.handle(), show_tray);
            listen_for_links(app.handle());
            match hosts::watch(app.handle()) {
                Ok(watch) => {
                    app.manage(watch);
                }
                Err(error) => eprintln!("hosven: not watching the hosts file: {error}"),
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            hosts::hosts_snapshot,
            hosts::hosts_reload,
            hosts::hosts_add_entry,
            hosts::hosts_update_entry,
            hosts::hosts_duplicate_entry,
            hosts::hosts_toggle_entry,
            hosts::hosts_delete_entry,
            hosts::hosts_set_tag_enabled,
            hosts::hosts_import,
            hosts::hosts_undo,
            hosts::hosts_redo,
            hosts::hosts_discard,
            hosts::hosts_set_raw_draft,
            hosts::hosts_apply,
            hosts::hosts_apply_raw,
            hosts::hosts_switch_environment,
            hosts::hosts_confirm_pending_switch,
            hosts::hosts_cancel_pending_switch,
            hosts::hosts_switch_preview,
            hosts::hosts_dismiss_verification,
            hosts::hosts_export,
            hosts::hosts_backup,
            hosts::profiles_add,
            hosts::profiles_rename,
            hosts::profiles_remove,
            hosts::profiles_set_exclusive,
            hosts::profiles_set_color,
            settings::settings_get,
            settings::settings_update,
            settings::settings_preview_transparency,
            tray::panel_toggle_block,
            tray::panel_open_main,
            tray::panel_hide,
            tray::panel_quit,
            ports::ports_snapshot,
            ports::ports_scan,
            ports::ports_kill,
            ports::ports_stop_container,
            external_tools::tools_detect,
            external_tools::tools_open_in_terminal,
            external_tools::tools_open_in_editor,
            external_tools::tools_reveal_in_finder,
            external_tools::tools_open_chrome_dns,
            git_exposure::git_exposure,
            git_exposure::git_add_to_gitignore,
            env::env_snapshot,
            env::env_add_repo,
            env::env_remove_repo,
            env::env_rename_repo,
            env::env_select,
            env::env_discover,
            env::env_add_entry,
            env::env_update_entry,
            env::env_duplicate_entry,
            env::env_toggle_entry,
            env::env_delete_entry,
            env::env_select_variant,
            env::env_add_missing,
            env::env_discard,
            env::env_set_raw_draft,
            env::env_merge_raw_draft,
            env::env_apply,
            env::env_apply_raw,
            env::env_reload_from_disk,
            env::env_refresh_from_disk,
            env::env_dismiss_conflict,
            env::env_profile_save,
            env::env_profile_apply,
            env::env_profile_rename,
            env::env_profile_delete,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| match event {
            // A click on the Dock icon.
            #[cfg(target_os = "macos")]
            RunEvent::Reopen { .. } => tray::show_main(app),
            // Only closing every window asks without a code; quitting does not.
            RunEvent::ExitRequested {
                code: None, api, ..
            } if tray::keeps_running(app) => api.prevent_exit(),
            RunEvent::Exit => {
                // Root rights must not outlive the app.
                if let Some(runner) = app.try_state::<platform::SharedRunner>() {
                    runner.release();
                }
            }
            _ => {}
        });
}

/// Routes `hosven://` links to the app. Setup runs before the event loop,
/// so on macOS a link that launched the app arrives later through the
/// listener; `get_current` covers the platforms that pass it as an argument,
/// which the listener never sees. Neither path repeats the other.
fn listen_for_links(app: &tauri::AppHandle) {
    if let Ok(Some(urls)) = app.deep_link().get_current() {
        for url in urls {
            url_command::receive(app, url.into());
        }
    }
    let handle = app.clone();
    app.deep_link().on_open_url(move |event| {
        for url in event.urls() {
            url_command::receive(&handle, url.into());
        }
    });
}
