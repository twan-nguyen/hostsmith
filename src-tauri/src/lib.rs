use std::sync::Arc;

use tauri::{Manager, RunEvent};

mod appearance;
mod config;
mod external_tools;
mod git_exposure;
mod hosts;
mod hosts_writer;
mod login_path;
mod platform;
mod ports;
mod profiles;
// `suspend`, `resume` and `path` wait for the env watcher, which re-arms
// after its own atomic saves.
#[allow(dead_code)]
mod watcher;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    login_path::adopt();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_deep_link::init())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                appearance::apply_vibrancy(&window, true)?;
            }
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
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                // Root rights must not outlive the app.
                if let Some(runner) = app.try_state::<platform::SharedRunner>() {
                    runner.release();
                }
            }
        });
}
