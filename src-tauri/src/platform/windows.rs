//! Windows: foreground-window targets, the pill's low-level mouse hook, topmost upkeep.
use super::PillEvent;
use retone_core::pillgeom;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, Ordering::SeqCst};
use std::sync::{mpsc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// The window the text came from.
#[derive(Debug, Clone, Copy)]
pub struct Target(isize);

pub fn current_target() -> Option<Target> {
    let h = unsafe { GetForegroundWindow() };
    (!h.is_null()).then(|| Target(h as isize))
}

pub(super) fn before_copy(_t: &Target) {}

fn hwnd(t: &Target) -> HWND { t.0 as HWND }

fn is_foreground(h: HWND) -> bool { unsafe { GetForegroundWindow() == h } }

fn wait_foreground(h: HWND, ms: u64) -> bool {
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(ms) {
        if is_foreground(h) { return true; }
        std::thread::sleep(Duration::from_millis(20));
    }
    is_foreground(h)
}

/// Bring the target back to the foreground (≤1 s). `false` if it is gone or refuses.
pub fn reactivate(t: &Target) -> bool {
    let h = hwnd(t);
    unsafe {
        if IsWindow(h) == 0 { return false; }
        if is_foreground(h) { return true; }
        SetForegroundWindow(h);
        if wait_foreground(h, 500) { return true; }
        // Fallback: attach to the current foreground thread so Windows allows the switch.
        let fg = GetForegroundWindow();
        let fg_thread = GetWindowThreadProcessId(fg, std::ptr::null_mut());
        let me = GetCurrentThreadId();
        AttachThreadInput(me, fg_thread, 1);
        BringWindowToTop(h);
        SetForegroundWindow(h);
        AttachThreadInput(me, fg_thread, 0);
        wait_foreground(h, 500)
    }
}

/// Give one of our own windows keyboard focus even when another app is in front
/// (Windows blocks plain SetForegroundWindow from background processes).
pub fn force_foreground(window: isize) {
    let h = window as HWND;
    unsafe {
        // Windows lets the process that produced the most recent input take the foreground;
        // a zero-distance mouse move makes that us without side effects.
        let mut input: INPUT = std::mem::zeroed();
        input.r#type = INPUT_MOUSE;
        input.Anonymous.mi.dwFlags = MOUSEEVENTF_MOVE;
        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
        SetForegroundWindow(h);
        if GetForegroundWindow() == h { return; }
        let fg = GetForegroundWindow();
        let fg_thread = GetWindowThreadProcessId(fg, std::ptr::null_mut());
        let me = GetCurrentThreadId();
        AttachThreadInput(me, fg_thread, 1);
        BringWindowToTop(h);
        SetForegroundWindow(h);
        AttachThreadInput(me, fg_thread, 0);
    }
}

pub(super) fn modifiers_held() -> bool {
    [VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN]
        .iter().any(|k| unsafe { GetAsyncKeyState(*k as i32) } as u16 & 0x8000 != 0)
}

pub fn accessibility_trusted() -> bool { true }
pub fn open_accessibility_settings() {}

// ---------- Pill: display-only webview + WH_MOUSE_LL hook (see docs/spike-notes.md) ----------

static PILL: AtomicIsize = AtomicIsize::new(0);
static DOWN: AtomicBool = AtomicBool::new(false);
static MOVED: AtomicBool = AtomicBool::new(false);
static RDOWN: AtomicBool = AtomicBool::new(false);
static X0: AtomicI32 = AtomicI32::new(0);
static Y0: AtomicI32 = AtomicI32::new(0);
static DX: AtomicI32 = AtomicI32::new(0);
static DY: AtomicI32 = AtomicI32::new(0);
static EVENTS: OnceLock<Mutex<mpsc::Sender<PillEvent>>> = OnceLock::new();

fn pill() -> HWND { PILL.load(SeqCst) as HWND }

fn pill_rect() -> Option<RECT> {
    let h = pill();
    unsafe {
        if h.is_null() || IsWindowVisible(h) == 0 { return None; }
        let mut r: RECT = std::mem::zeroed();
        (GetWindowRect(h, &mut r) != 0).then_some(r)
    }
}

fn inside(p: POINT) -> Option<RECT> {
    pill_rect().filter(|r| p.x >= r.left && p.x < r.right && p.y >= r.top && p.y < r.bottom)
}

fn send(ev: PillEvent) {
    if let Some(tx) = EVENTS.get() { let _ = tx.lock().unwrap().send(ev); }
}

unsafe extern "system" fn hook(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        if code == HC_ACTION as i32 {
            let m = &*(lp as *const MSLLHOOKSTRUCT);
            match wp as u32 {
                WM_LBUTTONDOWN => if let Some(r) = inside(m.pt) {
                    DOWN.store(true, SeqCst);
                    MOVED.store(false, SeqCst);
                    X0.store(m.pt.x, SeqCst);
                    Y0.store(m.pt.y, SeqCst);
                    DX.store(m.pt.x - r.left, SeqCst);
                    DY.store(m.pt.y - r.top, SeqCst);
                    return 1; // swallow: the app being typed in keeps focus
                },
                WM_MOUSEMOVE if DOWN.load(SeqCst) => {
                    if (m.pt.x - X0.load(SeqCst)).abs() > 4 || (m.pt.y - Y0.load(SeqCst)).abs() > 4 {
                        MOVED.store(true, SeqCst);
                    }
                    if MOVED.load(SeqCst) {
                        SetWindowPos(pill(), HWND_TOPMOST, m.pt.x - DX.load(SeqCst), m.pt.y - DY.load(SeqCst), 0, 0,
                                     SWP_NOSIZE | SWP_NOACTIVATE);
                    }
                }
                WM_LBUTTONUP if DOWN.load(SeqCst) => {
                    DOWN.store(false, SeqCst);
                    if MOVED.load(SeqCst) {
                        let (x, y) = (m.pt.x - DX.load(SeqCst), m.pt.y - DY.load(SeqCst));
                        SetWindowPos(pill(), HWND_TOPMOST, x, y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
                        send(PillEvent::Moved(x, y));
                    } else if let Some(r) = pill_rect() {
                        send(PillEvent::Click(pillgeom::region(X0.load(SeqCst) - r.left, r.right - r.left)));
                    }
                    return 1;
                }
                WM_RBUTTONDOWN if inside(m.pt).is_some() => { RDOWN.store(true, SeqCst); return 1; }
                WM_RBUTTONUP if RDOWN.swap(false, SeqCst) => { send(PillEvent::RightClick); return 1; }
                _ => {}
            }
        }
        CallNextHookEx(std::ptr::null_mut(), code, wp, lp)
    }
}

static HOOK: AtomicIsize = AtomicIsize::new(0);

/// (Re)install the low-level mouse hook. Must run on the UI thread: it pumps messages, and
/// LL hook callbacks are delivered through the installing thread's message loop.
/// Re-installing periodically heals the case where Windows silently drops a hook that once
/// exceeded LowLevelHooksTimeout.
pub fn install_hook() {
    if DOWN.load(SeqCst) { return; } // never swap hooks in the middle of a click/drag
    unsafe {
        let hk = SetWindowsHookExW(WH_MOUSE_LL, Some(hook), std::ptr::null_mut(), 0);
        if hk.is_null() {
            crate::applog::line(format!("mouse hook install failed (err {})", GetLastError()));
            return;
        }
        let old = HOOK.swap(hk as isize, SeqCst);
        if old != 0 { UnhookWindowsHookEx(old as HHOOK); }
    }
}

/// Tauri re-applies its own window styles (e.g. on show/resize), so keep ours present.
fn ensure_pill_styles() {
    let h = pill();
    if h.is_null() { return; }
    unsafe {
        let want = (WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW) as isize;
        let ex = GetWindowLongPtrW(h, GWL_EXSTYLE);
        if ex & want != want { SetWindowLongPtrW(h, GWL_EXSTYLE, ex | want); }
    }
}

/// Make the pill non-activating, route its clicks through a low-level mouse hook (installed
/// on the calling UI thread), and keep it topmost. `reinstall_on_ui` must run `install_hook`
/// on the UI thread; it is called every 30 s.
pub fn install_pill(pill_hwnd: isize, events: mpsc::Sender<PillEvent>, reinstall_on_ui: impl Fn() + Send + 'static) {
    PILL.store(pill_hwnd, SeqCst);
    let _ = EVENTS.set(Mutex::new(events));
    ensure_pill_styles();
    install_hook();
    std::thread::spawn(move || {
        let mut ticks = 0u32;
        loop {
            std::thread::sleep(Duration::from_secs(1));
            ticks += 1;
            if pill_rect().is_some() {
                ensure_pill_styles();
                // Some apps (Electron, full-screen windows) push past topmost — re-assert it quietly.
                unsafe { SetWindowPos(pill(), HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE) };
            }
            if ticks % 30 == 0 { reinstall_on_ui(); }
        }
    });
}
