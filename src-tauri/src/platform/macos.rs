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

/// Never copy from whatever happens to be in front if the source app could not be brought back:
/// that could send text from a different app to the AI.
pub(super) fn before_copy(t: &Target) -> bool { reactivate(t) }

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceFlagsState(state_id: i32) -> u64;
}

pub(super) fn modifiers_held() -> bool {
    // kCGEventSourceStateCombinedSessionState = 0; shift | control | alternate | command
    let mask: u64 = 0x20000 | 0x40000 | 0x80000 | 0x100000;
    unsafe { CGEventSourceFlagsState(0) & mask != 0 }
}

// --- Accessibility (needed to post ⌘C / ⌘V into other apps) ---------------------------------

type CFTypeRef = *const std::ffi::c_void;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> u8; // C `Boolean`
    fn AXIsProcessTrustedWithOptions(options: CFTypeRef) -> u8;
    static kAXTrustedCheckOptionPrompt: CFTypeRef;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFBooleanTrue: CFTypeRef;
    static kCFTypeDictionaryKeyCallBacks: u8;
    static kCFTypeDictionaryValueCallBacks: u8;
    fn CFDictionaryCreate(alloc: CFTypeRef, keys: *const CFTypeRef, values: *const CFTypeRef, n: isize,
                          key_cb: *const u8, value_cb: *const u8) -> CFTypeRef;
    fn CFRelease(cf: CFTypeRef);
}

pub fn accessibility_trusted() -> bool { unsafe { AXIsProcessTrusted() != 0 } }

/// Ask macOS to show its "Retone would like to control this computer" prompt. This also adds
/// Retone to the Accessibility list (switched off), so the user only has to flip the switch.
pub fn request_accessibility() -> bool {
    unsafe {
        let keys = [kAXTrustedCheckOptionPrompt];
        let values = [kCFBooleanTrue];
        let dict = CFDictionaryCreate(std::ptr::null(), keys.as_ptr(), values.as_ptr(), 1,
                                      &kCFTypeDictionaryKeyCallBacks, &kCFTypeDictionaryValueCallBacks);
        if dict.is_null() { return accessibility_trusted(); }
        let ok = AXIsProcessTrustedWithOptions(dict) != 0;
        CFRelease(dict);
        ok
    }
}

/// Opens System Settings → Privacy & Security → Accessibility (from the Settings button).
pub fn open_accessibility_settings() {
    let _ = request_accessibility(); // makes sure Retone is listed there
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility").spawn();
}

// --- ⌘C / ⌘V ---------------------------------------------------------------------------------

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceCreate(state_id: i32) -> CFTypeRef;
    fn CGEventCreateKeyboardEvent(source: CFTypeRef, keycode: u16, key_down: bool) -> CFTypeRef;
    fn CGEventSetFlags(event: CFTypeRef, flags: u64);
    fn CGEventPost(tap: u32, event: CFTypeRef);
}

const CG_EVENT_FLAG_COMMAND: u64 = 0x100000;
const CG_SESSION_EVENT_TAP: u32 = 1; // kCGSessionEventTap
const CG_STATE_COMBINED_SESSION: i32 = 0; // kCGEventSourceStateCombinedSessionState

/// Post ⌘+key. Uses ANSI virtual key codes (so it never touches the keyboard-layout APIs,
/// which must run on the main thread); `c`/`v` are the only keys Retone sends.
pub(super) fn command_chord(c: char) {
    let keycode: u16 = match c { 'c' => 0x08, 'v' => 0x09, _ => return }; // kVK_ANSI_C / kVK_ANSI_V
    unsafe {
        let source = CGEventSourceCreate(CG_STATE_COMBINED_SESSION);
        for down in [true, false] {
            let ev = CGEventCreateKeyboardEvent(source, keycode, down);
            if ev.is_null() { continue; }
            CGEventSetFlags(ev, CG_EVENT_FLAG_COMMAND);
            CGEventPost(CG_SESSION_EVENT_TAP, ev);
            CFRelease(ev);
            std::thread::sleep(Duration::from_millis(15));
        }
        if !source.is_null() { CFRelease(source); }
    }
}

pub fn install_hook() {}

/// Track the last frontmost app that isn't Retone (clicking the pill activates Retone on macOS).
pub fn install_pill(_pill: isize, _events: mpsc::Sender<PillEvent>, _reinstall_on_ui: impl Fn() + Send + 'static) {
    std::thread::spawn(|| loop {
        if let Some(p) = frontmost_pid() { if p != own_pid() { LAST_EXTERNAL_PID.store(p, SeqCst); } }
        std::thread::sleep(Duration::from_millis(250));
    });
}
