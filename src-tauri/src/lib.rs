use tauri::Manager;

mod appearance;
mod config;
mod env;
mod hosts;
mod login_path;
// Shared by the platform services, which register their commands next.
#[allow(dead_code)]
mod platform;
mod profiles;
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
            hosts::profiles_add,
            hosts::profiles_rename,
            hosts::profiles_remove,
            hosts::profiles_set_exclusive,
            hosts::profiles_set_color,
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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
