//! OS adapters: which app the text came from, getting back to it, clipboard + keystrokes,
//! and the floating pill's mouse handling.
use retone_core::capture::{ClipboardIo, KeySender};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::*;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::*;

/// Real clipboard + keyboard, implementing the core capture traits.
pub struct Io {
    clipboard: arboard::Clipboard,
    /// Not used on macOS (keys go through CGEvent), where it would fail without Accessibility.
    keys: Option<enigo::Enigo>,
}

impl Io {
    pub fn new() -> Option<Io> {
        let clipboard = arboard::Clipboard::new().ok()?;
        let keys = if cfg!(target_os = "macos") { None }
                   else { Some(enigo::Enigo::new(&enigo::Settings::default()).ok()?) };
        Some(Io { clipboard, keys })
    }

    /// Ctrl+<c> on Windows; ⌘<c> on macOS (posted as CGEvents with an explicit Command flag,
    /// which is safe off the main thread and ignores any modifier the user is still holding).
    fn chord(&mut self, c: char) {
        #[cfg(target_os = "macos")]
        {
            let _ = &self.keys;
            command_chord(c);
        }
        #[cfg(not(target_os = "macos"))]
        {
            use enigo::{Direction::*, Key, Keyboard};
            let Some(keys) = self.keys.as_mut() else { return };
            let _ = keys.key(Key::Control, Press);
            let _ = keys.key(Key::Unicode(c), Click);
            let _ = keys.key(Key::Control, Release);
        }
    }
}

/// Clipboard contents saved before Retone borrows the clipboard.
pub enum SavedClip { Empty, Text(String), Image(arboard::ImageData<'static>) }

/// Everything Retone itself puts on the clipboard (the copy sentinel, the result being pasted,
/// the user's own clipboard being put back) is flagged as private. Windows: excluded from Win+V
/// history and cloud clipboard. macOS: marked `org.nspasteboard.ConcealedType`, which clipboard
/// managers that follow the convention skip; it does not stop Universal Clipboard/Handoff.
/// The selection itself is copied by the target app (simulated Ctrl+C/⌘C) and can't be flagged.
fn private_set(cb: &mut arboard::Clipboard) -> arboard::Set<'_> {
    #[cfg(windows)]
    { use arboard::SetExtWindows; cb.set().exclude_from_monitoring() }
    #[cfg(target_os = "macos")]
    { use arboard::SetExtApple; cb.set().exclude_from_history() }
    #[cfg(not(any(windows, target_os = "macos")))]
    { cb.set() }
}

impl ClipboardIo for Io {
    type Saved = SavedClip;
    fn save(&mut self) -> SavedClip {
        if let Ok(t) = self.clipboard.get_text() { return SavedClip::Text(t); }
        if let Ok(i) = self.clipboard.get_image() { return SavedClip::Image(i); }
        SavedClip::Empty
    }
    fn restore(&mut self, saved: SavedClip) {
        let _ = match saved {
            SavedClip::Text(t) => private_set(&mut self.clipboard).text(t),
            SavedClip::Image(i) => private_set(&mut self.clipboard).image(i),
            SavedClip::Empty => self.clipboard.clear(),
        };
    }
    fn get_text(&mut self) -> Option<String> { self.clipboard.get_text().ok() }
    fn set_text(&mut self, t: &str) { let _ = private_set(&mut self.clipboard).text(t); }
}

impl KeySender for Io {
    fn wait_modifiers_released(&mut self) {
        let start = std::time::Instant::now();
        while modifiers_held() && start.elapsed() < std::time::Duration::from_secs(1) {
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
    }
    fn copy(&mut self) { self.chord('c'); }
    fn paste(&mut self) { self.chord('v'); }
    fn sleep_ms(&mut self, ms: u64) { std::thread::sleep(std::time::Duration::from_millis(ms)); }
}

/// Copy the selection from `target` (switching back to it first where the platform needs that).
pub fn copy_from(target: Option<&Target>) -> Option<String> {
    if let Some(t) = target { if !before_copy(t) { return None; } }
    let mut io = Io::new()?;
    retone_core::capture::copy_selection(&mut io)
}

/// `job::Desktop` implementation: paste back into the app the text came from.
pub struct PlatformDesktop { pub target: Option<Target> }

impl retone_core::job::Desktop for PlatformDesktop {
    fn paste_into_target(&mut self, text: &str) -> bool {
        let Some(mut io) = Io::new() else { return false };
        match &self.target {
            Some(t) if reactivate(t) => { retone_core::capture::paste_text(&mut io, text); true }
            _ => { retone_core::capture::leave_on_clipboard(&mut io, text); false }
        }
    }
}

/// Mouse events from the pill, delivered off the OS hook thread.
#[derive(Debug, Clone, Copy)]
pub enum PillEvent { Click(retone_core::pillgeom::Region), RightClick, Moved(i32, i32) }
