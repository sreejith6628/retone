//! The floating pill: showing/positioning it, status messages inside it, and its mouse events.
use crate::platform::PillEvent;
use crate::state::AppState;
use crate::{flows, menus, winmgr};
use retone_core::job::{PillStatus, StatusSink};
use retone_core::pillgeom::{self, Region, PILL_H, PILL_W};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewWindow};

pub const LABEL: &str = "pill";

pub fn window(app: &AppHandle) -> Option<WebviewWindow> { app.get_webview_window(LABEL) }

#[derive(Serialize, Clone)]
struct StatusPayload { text: String, kind: &'static str }

static GENERATION: AtomicU64 = AtomicU64::new(0);

/// Shows job progress inside the pill; transient messages revert to idle after a few seconds.
pub struct PillSink(pub AppHandle);

impl StatusSink for PillSink {
    fn status(&self, s: PillStatus) { set_status(&self.0, s); }
}

pub fn set_status(app: &AppHandle, s: PillStatus) {
    let (text, kind) = match &s {
        PillStatus::Idle => ("✨ Polish".to_string(), "idle"),
        PillStatus::Working => ("⏳ Working".to_string(), "working"),
        PillStatus::Done => ("✅ Done".to_string(), "done"),
        PillStatus::Copied => ("📋 Result copied — paste it".to_string(), "done"),
        PillStatus::Hint(h) => (h.clone(), "hint"),
        PillStatus::Error(e) => (format!("⚠ {e}"), "error"),
    };
    *app.state::<AppState>().error_showing.lock().unwrap() = kind == "error";
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    if let Some(w) = window(app) {
        let width = if kind == "idle" || kind == "working" { PILL_W } else { pillgeom::status_width(&text) };
        let _ = w.set_size(LogicalSize::new(width as f64, PILL_H as f64));
        let _ = w.emit_to(LABEL, "pill-status", StatusPayload { text, kind });
    }
    if !matches!(s, PillStatus::Idle | PillStatus::Working) {
        let app = app.clone();
        let secs = if kind == "error" { 8 } else { 4 };
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(secs));
            if GENERATION.load(Ordering::SeqCst) == generation { set_status(&app, PillStatus::Idle); }
        });
    }
}

/// Show or hide the pill, restoring its saved position (clamped onto a connected screen).
pub fn show(app: &AppHandle, visible: bool) {
    let Some(w) = window(app) else { return };
    if !visible { let _ = w.hide(); return; }
    let screens: Vec<(i32, i32, i32, i32)> = w.available_monitors().unwrap_or_default().iter()
        .map(|m| (m.position().x, m.position().y, m.size().width as i32, m.size().height as i32)).collect();
    let scale = w.scale_factor().unwrap_or(1.0);
    let size = ((PILL_W as f64 * scale) as i32, (PILL_H as f64 * scale) as i32);
    let saved = app.state::<AppState>().snapshot().pill_pos;
    let default = screens.first().map(|&(x, y, sw, _)| (x + (sw - size.0) / 2, y + 8)).unwrap_or((800, 8));
    let (x, y) = pillgeom::clamp_to_visible(saved.unwrap_or(default), size, &screens);
    let _ = w.set_position(PhysicalPosition::new(x, y));
    let _ = w.show();
}

pub fn handle_event(app: &AppHandle, ev: PillEvent) {
    match ev {
        PillEvent::Click(region) => click(app, region),
        PillEvent::RightClick => {
            let app2 = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let (Some(w), Ok(menu)) = (window(&app2), menus::pill_menu(&app2)) { let _ = w.popup_menu(&menu); }
            });
        }
        PillEvent::Moved(x, y) => {
            let _ = app.state::<AppState>().update(|s| s.pill_pos = Some((x, y)));
        }
    }
}

pub fn click(app: &AppHandle, region: Region) {
    let error = *app.state::<AppState>().error_showing.lock().unwrap();
    if error {
        set_status(app, PillStatus::Idle);
        winmgr::open_settings(app);
        return;
    }
    match region {
        Region::Main => flows::quick_polish(app.clone()),
        Region::Menu => flows::open_options(app.clone()),
    }
}
