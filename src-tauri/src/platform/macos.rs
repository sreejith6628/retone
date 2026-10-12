//! macOS: the target is the last app (other than Retone) that was frontmost; we switch back
//! to it before copying and before pasting. The pill's clicks are handled in its webview.
use super::PillEvent;
use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication, NSWorkspace};
use std::sync::atomic::{AtomicI32, Ordering::SeqCst};
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct Target(i32);

static LAST_EXTERNAL_PID: AtomicI32 = AtomicI32::new(0);

fn frontmost_pid() -> Option<i32> {
    objc2::rc::autoreleasepool(|_| unsafe {
        NSWorkspace::sharedWorkspace().frontmostApplication().map(|a| a.processIdentifier())
    })
}

fn own_pid() -> i32 { std::process::id() as i32 }

pub fn current_target() -> Option<Target> {
    match frontmost_pid() {
        Some(p) if p != own_pid() => Some(Target(p)),
        _ => Some(LAST_EXTERNAL_PID.load(SeqCst)).filter(|p| *p > 0).map(Target),
    }
}

pub fn reactivate(t: &Target) -> bool {
    if frontmost_pid() == Some(t.0) { return true; }
    let ok = objc2::rc::autoreleasepool(|_| unsafe {
        match NSRunningApplication::runningApplicationWithProcessIdentifier(t.0) {
            // Deprecated (no effect) on macOS 14+, where plain activation already suffices.
            #[allow(deprecated)]
            Some(app) => app.activateWithOptions(NSApplicationActivationOptions::NSApplicationActivateIgnoringOtherApps),
            None => false,
        }
    });
    if !ok { return false; }
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(1) {
        if frontmost_pid() == Some(t.0) { std::thread::sleep(Duration::from_millis(120)); return true; }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

pub fn force_foreground(_window: isize) {}

pub(super) fn before_copy(t: &Target) { let _ = reactivate(t); }

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceFlagsState(state_id: i32) -> u64;
}

pub(super) fn modifiers_held() -> bool {
    // kCGEventSourceStateCombinedSessionState = 0; shift | control | alternate | command
    let mask: u64 = 0x20000 | 0x40000 | 0x80000 | 0x100000;
    unsafe { CGEventSourceFlagsState(0) & mask != 0 }
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> u8; // C `Boolean`
}

pub fn accessibility_trusted() -> bool { unsafe { AXIsProcessTrusted() != 0 } }

pub fn open_accessibility_settings() {
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility").status();
}

/// Track the last frontmost app that isn't Retone (clicking the pill activates Retone on macOS).
pub fn install_hook() {}

pub fn install_pill(_pill: isize, _events: mpsc::Sender<PillEvent>, _reinstall_on_ui: impl Fn() + Send + 'static) {
    std::thread::spawn(|| loop {
        if let Some(p) = frontmost_pid() { if p != own_pid() { LAST_EXTERNAL_PID.store(p, SeqCst); } }
        std::thread::sleep(Duration::from_millis(250));
    });
}
