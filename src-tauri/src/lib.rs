use tauri::Manager;

mod appearance;
mod config;
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
