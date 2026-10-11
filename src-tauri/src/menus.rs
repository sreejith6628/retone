//! Tray icon and the pill's right-click menu (one handler for both).
use crate::{flows, pill, winmgr};
use crate::state::AppState;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

pub fn pill_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    Menu::with_items(app, &[
        &MenuItem::with_id(app, "options", "Polish with options…", true, None::<&str>)?,
        &MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?,
        &MenuItem::with_id(app, "hide-pill", "Hide floating button", true, None::<&str>)?,
        &PredefinedMenuItem::separator(app)?,
        &MenuItem::with_id(app, "quit", "Quit Retone", true, None::<&str>)?,
    ])
}

pub fn handle(app: &AppHandle, id: &str) {
    match id {
        "options" => flows::open_options(app.clone()),
        "settings" => winmgr::open_settings(app),
        "hide-pill" | "toggle-pill" => {
            let visible = id == "toggle-pill" && !pill::window(app).and_then(|w| w.is_visible().ok()).unwrap_or(false);
            if let Ok(s) = app.state::<AppState>().update(|s| s.show_pill = visible) { pill::show(app, s.show_pill); }
        }
        "quit" => app.exit(0),
        _ => {}
    }
}

pub fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let menu = Menu::with_items(app, &[
        &MenuItem::with_id(app, "options", "Polish with options…", true, None::<&str>)?,
        &MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?,
        &MenuItem::with_id(app, "toggle-pill", "Show/hide floating button", true, None::<&str>)?,
        &PredefinedMenuItem::separator(app)?,
        &MenuItem::with_id(app, "quit", "Quit Retone", true, None::<&str>)?,
    ])?;
    let mut b = TrayIconBuilder::with_id("retone")
        .tooltip("Retone")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| handle(app, e.id().as_ref()))
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                winmgr::open_settings(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() { b = b.icon(icon.clone()); }
    b.build(app)?;
    Ok(())
}
