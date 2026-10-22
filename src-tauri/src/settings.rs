//! Preferences the Rust side acts on: appearance, editor font size, the menu
//! bar icon and window transparency. Kept in `settings.json` in the config
//! directory; layout state such as the sidebar width stays in the webview.

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::window::{Effect, EffectState, EffectsBuilder};
use tauri::{AppHandle, Emitter, Manager, State, Theme, WebviewWindow};

use crate::config;
use crate::util::lock;

pub const EVENT: &str = "settings-changed";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppearanceMode {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub appearance_mode: AppearanceMode,
    pub editor_font_size: u8,
    pub show_menu_bar_extra: bool,
    /// How much of the window's blur shows, 0 (solid) to 100.
    pub transparency: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            appearance_mode: AppearanceMode::System,
            editor_font_size: 13,
            show_menu_bar_extra: true,
            transparency: 0,
        }
    }
}

impl Settings {
    /// Keeps hand-edited values inside what the UI offers.
    fn clamped(mut self) -> Self {
        self.editor_font_size = self.editor_font_size.clamp(10, 20);
        self.transparency = self.transparency.min(100);
        self
    }

    /// The preferences Hosven 1.x kept in UserDefaults.
    fn from_legacy(plist: &std::path::Path) -> Option<Self> {
        let value = plist::Value::from_file(plist).ok()?;
        let defaults = value.as_dictionary()?;
        let mut settings = Self::default();
        if let Some(mode) = defaults.get("appearanceMode").and_then(|v| v.as_string()) {
            settings.appearance_mode = match mode {
                "light" => AppearanceMode::Light,
                "dark" => AppearanceMode::Dark,
                _ => AppearanceMode::System,
            };
        }
        if let Some(size) = defaults.get("editorFontSize").and_then(|v| v.as_real()) {
            settings.editor_font_size = size.round().clamp(10.0, 20.0) as u8;
        }
        if let Some(show) = defaults
            .get("showMenuBarExtra")
            .and_then(|v| v.as_boolean())
        {
            settings.show_menu_bar_extra = show;
        }
        Some(settings)
    }
}

/// Fields the UI changes; the rest stay as they are.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    pub appearance_mode: Option<AppearanceMode>,
    pub editor_font_size: Option<u8>,
    pub show_menu_bar_extra: Option<bool>,
    pub transparency: Option<u8>,
}

pub struct SettingsState {
    path: PathBuf,
    settings: Mutex<Settings>,
}

impl SettingsState {
    pub fn load() -> Self {
        Self::load_from(
            config::config_dir().join("settings.json"),
            &config::legacy_defaults_path(),
        )
    }

    fn load_from(path: PathBuf, legacy: &std::path::Path) -> Self {
        let settings = match std::fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<Settings>(&bytes) {
                Ok(settings) => settings.clamped(),
                Err(error) => {
                    // Kept aside rather than overwritten by the next change.
                    let kept = config::set_aside(&path);
                    eprintln!(
                        "hostsmith: settings.json could not be read ({error}); moved to {kept:?}"
                    );
                    Settings::default()
                }
            },
            Err(_) => {
                let migrated = Settings::from_legacy(legacy).unwrap_or_default();
                let _ = config::write_json_atomic(&path, &migrated);
                migrated
            }
        };
        Self {
            path,
            settings: Mutex::new(settings),
        }
    }

    pub fn get(&self) -> Settings {
        lock(&self.settings).clone()
    }

    fn update(&self, patch: SettingsPatch) -> Result<Settings, String> {
        let mut settings = lock(&self.settings);
        let mut next = settings.clone();
        if let Some(mode) = patch.appearance_mode {
            next.appearance_mode = mode;
        }
        if let Some(size) = patch.editor_font_size {
            next.editor_font_size = size;
        }
        if let Some(show) = patch.show_menu_bar_extra {
            next.show_menu_bar_extra = show;
        }
        if let Some(percent) = patch.transparency {
            next.transparency = percent;
        }
        let next = next.clamped();
        config::write_json_atomic(&self.path, &next)?;
        *settings = next.clone();
        Ok(next)
    }
}

/// Sets the window's theme and blur to match `settings`.
pub fn apply(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };
    let theme = match settings.appearance_mode {
        AppearanceMode::System => None,
        AppearanceMode::Light => Some(Theme::Light),
        AppearanceMode::Dark => Some(Theme::Dark),
    };
    window.set_theme(theme).map_err(|e| e.to_string())?;
    apply_vibrancy(&window, settings.transparency > 0)
}

/// Turns the macOS sidebar material on or off behind the window's content.
/// The window is always transparent; how much of the blur shows is up to the
/// UI, which tints the canvas over it.
fn apply_vibrancy(window: &WebviewWindow, enabled: bool) -> Result<(), String> {
    let effects = enabled.then(|| {
        EffectsBuilder::new()
            .effect(Effect::Sidebar)
            .state(EffectState::FollowsWindowActiveState)
            .build()
    });
    window.set_effects(effects).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn settings_get(state: State<'_, SettingsState>) -> Settings {
    state.get()
}

#[tauri::command]
pub fn settings_update(
    app: AppHandle,
    state: State<'_, SettingsState>,
    patch: SettingsPatch,
) -> Result<Settings, String> {
    let settings = state.update(patch)?;
    // Saved already, so every window and the tray hear about it even when
    // the window cannot take the new look.
    crate::tray::set_enabled(&app, settings.show_menu_bar_extra);
    let _ = app.emit(EVENT, settings.clone());
    apply(&app, &settings)?;
    Ok(settings)
}

/// Follows the transparency slider while it moves, without saving.
#[tauri::command]
pub fn settings_preview_transparency(app: AppHandle, percent: u8) -> Result<(), String> {
    match app.get_webview_window("main") {
        Some(window) => apply_vibrancy(&window, percent > 0),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hostsmith-settings-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn defaults_when_nothing_exists() {
        let dir = temp_dir();
        let state = SettingsState::load_from(dir.join("settings.json"), &dir.join("none.plist"));
        assert_eq!(state.get(), Settings::default());
        assert!(
            dir.join("settings.json").exists(),
            "the defaults are written once"
        );
    }

    #[test]
    fn imports_the_hosven_1_preferences() {
        let dir = temp_dir();
        let mut defaults = plist::Dictionary::new();
        defaults.insert("appearanceMode".into(), plist::Value::String("dark".into()));
        defaults.insert("editorFontSize".into(), plist::Value::Real(15.0));
        defaults.insert("showMenuBarExtra".into(), plist::Value::Boolean(false));
        defaults.insert("sidebarWidth".into(), plist::Value::Real(300.0));
        let legacy = dir.join("legacy.plist");
        plist::Value::Dictionary(defaults)
            .to_file_binary(&legacy)
            .unwrap();

        let state = SettingsState::load_from(dir.join("settings.json"), &legacy);
        assert_eq!(
            state.get(),
            Settings {
                appearance_mode: AppearanceMode::Dark,
                editor_font_size: 15,
                show_menu_bar_extra: false,
                transparency: 0,
            }
        );
    }

    #[test]
    fn an_update_changes_only_what_it_names_and_persists() {
        let dir = temp_dir();
        let path = dir.join("settings.json");
        let state = SettingsState::load_from(path.clone(), &dir.join("none.plist"));
        let updated = state
            .update(SettingsPatch {
                editor_font_size: Some(40),
                transparency: Some(35),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(
            updated.editor_font_size, 20,
            "clamped to what the UI offers"
        );
        assert_eq!(updated.transparency, 35);
        assert!(updated.show_menu_bar_extra);

        let reread = SettingsState::load_from(path, &dir.join("none.plist"));
        assert_eq!(reread.get(), updated);
    }

    #[test]
    fn missing_fields_take_their_defaults() {
        let dir = temp_dir();
        let path = dir.join("settings.json");
        std::fs::write(&path, r#"{"appearanceMode":"light"}"#).unwrap();
        let state = SettingsState::load_from(path, &dir.join("none.plist"));
        assert_eq!(state.get().appearance_mode, AppearanceMode::Light);
        assert_eq!(state.get().editor_font_size, 13);
    }
}
