use std::sync::Arc;

use tauri::{Manager, RunEvent};

mod appearance;
mod config;
mod env;
mod external_tools;
mod git_exposure;
mod hosts;
mod hosts_writer;
mod login_path;
mod platform;
mod ports;
mod profiles;
mod settings;
mod toast;
mod watcher;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    login_path::adopt();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_deep_link::init())
        .setup(|app| {
            let settings = settings::SettingsState::load();
            settings::apply(app.handle(), &settings.get())?;
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
            app.manage(env::EnvState::load());
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
        .run(|app, event| {
            if let RunEvent::Exit = event {
                // Root rights must not outlive the app.
                if let Some(runner) = app.try_state::<platform::SharedRunner>() {
                    runner.release();
                }
            }
        });
}
