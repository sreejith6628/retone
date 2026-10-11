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
    keys: enigo::Enigo,
}

impl Io {
    pub fn new() -> Option<Io> {
        let clipboard = arboard::Clipboard::new().ok()?;
        let keys = enigo::Enigo::new(&enigo::Settings::default()).ok()?;
        Some(Io { clipboard, keys })
    }

    fn chord(&mut self, c: char) {
        use enigo::{Direction::*, Key, Keyboard};
        let modifier = if cfg!(target_os = "macos") { Key::Meta } else { Key::Control };
        let _ = self.keys.key(modifier, Press);
        let _ = self.keys.key(Key::Unicode(c), Click);
        let _ = self.keys.key(modifier, Release);
    }
}

/// Clipboard contents saved before Retone borrows the clipboard.
pub enum SavedClip { Empty, Text(String), Image(arboard::ImageData<'static>) }

impl ClipboardIo for Io {
    type Saved = SavedClip;
    fn save(&mut self) -> SavedClip {
        if let Ok(t) = self.clipboard.get_text() { return SavedClip::Text(t); }
        if let Ok(i) = self.clipboard.get_image() { return SavedClip::Image(i); }
        SavedClip::Empty
    }
    fn restore(&mut self, saved: SavedClip) {
        let _ = match saved {
            SavedClip::Text(t) => self.clipboard.set_text(t),
            SavedClip::Image(i) => self.clipboard.set_image(i),
            SavedClip::Empty => self.clipboard.clear(),
        };
    }
    fn get_text(&mut self) -> Option<String> { self.clipboard.get_text().ok() }
    fn set_text(&mut self, t: &str) { let _ = self.clipboard.set_text(t); }
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
    if let Some(t) = target { before_copy(t); }
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
