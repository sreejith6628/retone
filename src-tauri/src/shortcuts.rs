//! Global shortcuts for quick polish and the options panel.
use crate::flows;
use retone_core::settings::Settings;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

/// (Re)register both shortcuts. Returns a message per shortcut that could not be registered.
pub fn register(app: &AppHandle, s: &Settings) -> Vec<String> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let mut errors = Vec::new();
    let quick = s.shortcut_quick.clone();
    let options = s.shortcut_options.clone();
    if let Err(e) = gs.on_shortcut(quick.as_str(), |app, _, ev| {
        if ev.state == ShortcutState::Pressed { flows::quick_polish(app.clone()); }
    }) { errors.push(format!("Quick polish shortcut “{quick}”: {e}")); }
    if let Err(e) = gs.on_shortcut(options.as_str(), |app, _, ev| {
        if ev.state == ShortcutState::Pressed { flows::open_options(app.clone()); }
    }) { errors.push(format!("Options shortcut “{options}”: {e}")); }
    errors
}
