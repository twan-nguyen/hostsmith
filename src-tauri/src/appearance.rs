use tauri::{
    WebviewWindow,
    window::{Effect, EffectState, EffectsBuilder},
};

/// Turns the macOS sidebar material on or off behind the window's content.
/// The window is always transparent; how much of the blur shows is up to the
/// UI, which tints the canvas over it.
pub fn apply_vibrancy(window: &WebviewWindow, enabled: bool) -> Result<(), String> {
    let effects = enabled.then(|| {
        EffectsBuilder::new()
            .effect(Effect::Sidebar)
            .state(EffectState::FollowsWindowActiveState)
            .build()
    });
    window.set_effects(effects).map_err(|e| e.to_string())
}
