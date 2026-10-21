use tauri::Manager;

mod appearance;
mod login_path;
// Shared by the hosts and env services, which register their commands next.
#[allow(dead_code)]
mod config;
#[allow(dead_code)]
mod platform;
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
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
