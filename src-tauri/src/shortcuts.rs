//! Global shortcuts for quick polish and the options panel.
use crate::flows;
use retone_core::settings::Settings;
use tauri::AppHandle;
use std::str::FromStr;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

/// Check both shortcuts parse, use a modifier, and differ — before anything is saved.
pub fn validate(quick: &str, options: &str) -> Result<(), String> {
    let parse = |name: &str, s: &str| -> Result<Shortcut, String> {
        let sc = Shortcut::from_str(s).map_err(|_| format!("{name} shortcut “{s}” isn't a valid key combination"))?;
        if sc.mods == Modifiers::empty() { return Err(format!("{name} shortcut “{s}” needs a modifier key (Ctrl, Alt, Shift or ⌘)")); }
        Ok(sc)
    };
    let q = parse("Quick polish", quick)?;
    let o = parse("Options", options)?;
    if q == o { return Err("The two shortcuts must be different".into()); }
    Ok(())
}

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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_shortcuts() {
        assert!(validate("Ctrl+Shift+P", "Ctrl+Shift+O").is_ok());
        assert!(validate("Alt+Super+P", "Alt+Super+O").is_ok());
        assert!(validate("Ctrl+Shift+P", "Ctrl+Shift+P").unwrap_err().contains("different"));
        assert!(validate("Ctrl+Shift+", "Ctrl+Shift+O").is_err());
        assert!(validate("P", "Ctrl+Shift+O").unwrap_err().contains("modifier"));
    }
}
