//! On-demand windows (settings, options). Both are destroyed on close; only the pill lives on.
use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

pub const SETTINGS: &str = "settings";
pub const OPTIONS: &str = "options";

fn bring_app_forward(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    { let _ = app.show(); }
    let _ = app;
}

pub fn open_settings(app: &AppHandle) {
    let app = app.clone();
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || {
        bring_app_forward(&app2);
        if let Some(w) = app2.get_webview_window(SETTINGS) {
            let _ = w.unminimize();
            let _ = w.show();
            let _ = w.set_focus();
            return;
        }
        let _ = WebviewWindowBuilder::new(&app2, SETTINGS, WebviewUrl::App("index.html".into()))
            .title("Retone Settings")
            .inner_size(780.0, 640.0)
            .min_inner_size(640.0, 520.0)
            .center()
            .focused(true)
            .build();
    });
}

/// Open the options panel next to the mouse cursor.
pub fn open_options(app: &AppHandle) {
    let app = app.clone();
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || {
        bring_app_forward(&app2);
        if let Some(w) = app2.get_webview_window(OPTIONS) { let _ = w.destroy(); }
        let (w, h) = (540.0, 500.0);
        let mut b = WebviewWindowBuilder::new(&app2, OPTIONS, WebviewUrl::App("index.html".into()))
            .title("Polish with Retone")
            .inner_size(w, h)
            .resizable(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .focused(true);
        if let Ok(pos) = app2.cursor_position() {
            let monitor = app2.monitor_from_point(pos.x, pos.y).ok().flatten();
            let scale = monitor.as_ref().map(|m| m.scale_factor()).unwrap_or(1.0);
            let (pw, ph) = ((w * scale) as i32, (h * scale) as i32);
            let (mut x, mut y) = (pos.x as i32 - pw / 2, pos.y as i32 + (24.0 * scale) as i32);
            if let Some(m) = monitor {
                let (mx, my) = (m.position().x, m.position().y);
                let (mw, mh) = (m.size().width as i32, m.size().height as i32);
                x = x.clamp(mx, mx + mw - pw);
                y = y.clamp(my, my + mh - ph - (48.0 * scale) as i32);
            }
            b = b.position(0.0, 0.0);
            if let Ok(win) = b.build() { let _ = win.set_position(PhysicalPosition::new(x, y)); focus(&win); }
            return;
        }
        if let Ok(win) = b.center().build() { focus(&win); }
    });
}

/// Bring an already-open options panel to the front.
pub fn focus_options(app: &AppHandle) {
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(w) = app2.get_webview_window(OPTIONS) { focus(&w); }
    });
}

/// Options opened from a global shortcut must take keyboard focus from the app in front.
fn focus(win: &tauri::WebviewWindow) {
    let _ = win.set_focus();
    #[cfg(windows)]
    if let Ok(h) = win.hwnd() { crate::platform::force_foreground(h.0 as isize); }
}

pub fn close(app: &AppHandle, label: &str) {
    // destroy (not close): close() re-enters the webview's close-requested handler.
    if let Some(w) = app.get_webview_window(label) { let _ = w.destroy(); }
}
