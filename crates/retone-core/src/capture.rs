//! Copy the user's selection and paste results back, keeping their clipboard intact.
//! Platform-free: the OS clipboard and keystrokes come in through two small traits.

pub trait ClipboardIo {
    /// Whatever the clipboard holds now (text, image or nothing), to be put back later.
    type Saved;
    fn save(&mut self) -> Self::Saved;
    fn restore(&mut self, saved: Self::Saved);
    fn get_text(&mut self) -> Option<String>;
    fn set_text(&mut self, t: &str);
}

pub trait KeySender {
    /// Block (≤1 s) until the user has let go of the shortcut's modifier keys.
    fn wait_modifiers_released(&mut self);
    fn copy(&mut self);
    fn paste(&mut self);
    fn sleep_ms(&mut self, ms: u64);
}

pub const SENTINEL_PREFIX: &str = "\u{2063}retone-sentinel-";
const COPY_WAIT_MS: u64 = 600;
const POLL_MS: u64 = 20;
const PASTE_SETTLE_MS: u64 = 400;

/// Copy the current selection. `None` when nothing (or only whitespace) is selected.
/// Copy is detected by replacing the clipboard with a unique sentinel first.
pub fn copy_selection(io: &mut (impl ClipboardIo + KeySender)) -> Option<String> {
    io.wait_modifiers_released();
    let saved = io.save();
    let sentinel = format!("{SENTINEL_PREFIX}{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
    io.set_text(&sentinel);
    io.copy();
    let mut copied = None;
    let mut waited = 0;
    loop {
        match io.get_text() {
            Some(t) if t != sentinel => { copied = Some(t); break; }
            _ if waited >= COPY_WAIT_MS => break,
            _ => { io.sleep_ms(POLL_MS); waited += POLL_MS; }
        }
    }
    io.restore(saved);
    copied.filter(|t| !t.trim().is_empty())
}

/// Paste `text` into the focused app, then put the user's clipboard back.
pub fn paste_text(io: &mut (impl ClipboardIo + KeySender), text: &str) {
    io.wait_modifiers_released();
    let saved = io.save();
    io.set_text(text);
    io.sleep_ms(30);
    io.paste();
    io.sleep_ms(PASTE_SETTLE_MS);
    io.restore(saved);
}

/// When the target app can't be reactivated: leave the result for the user to paste.
pub fn leave_on_clipboard(cb: &mut impl ClipboardIo, text: &str) { cb.set_text(text); }

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Clip { Empty, Text(String), Image(Vec<u8>) }

    /// Fake desktop: `selection` is what the focused app would copy (None = nothing selected).
    struct Fake { clip: Clip, selection: Option<String>, pasted: Vec<String>, log: Vec<String> }
    impl Fake {
        fn new(clip: Option<&str>, selection: Option<&str>) -> Self {
            Fake { clip: clip.map(|c| Clip::Text(c.into())).unwrap_or(Clip::Empty),
                   selection: selection.map(String::from), pasted: vec![], log: vec![] }
        }
        fn text(&self) -> Option<&str> { if let Clip::Text(t) = &self.clip { Some(t) } else { None } }
    }
    impl ClipboardIo for Fake {
        type Saved = Clip;
        fn save(&mut self) -> Clip { self.clip.clone() }
        fn restore(&mut self, saved: Clip) { self.clip = saved; }
        fn get_text(&mut self) -> Option<String> { self.text().map(String::from) }
        fn set_text(&mut self, t: &str) { self.clip = Clip::Text(t.into()); }
    }
    impl KeySender for Fake {
        fn wait_modifiers_released(&mut self) { self.log.push("wait".into()); }
        fn copy(&mut self) { self.log.push("copy".into()); if let Some(s) = self.selection.clone() { self.clip = Clip::Text(s); } }
        fn paste(&mut self) { self.log.push("paste".into()); let c = self.text().unwrap_or_default().to_string(); self.pasted.push(c); }
        fn sleep_ms(&mut self, _ms: u64) {}
    }

    #[test]
    fn copies_selection_and_restores_clipboard() {
        let mut f = Fake::new(Some("old clip"), Some("hi team"));
        assert_eq!(copy_selection(&mut f), Some("hi team".into()));
        assert_eq!(f.text(), Some("old clip"));
        assert_eq!(f.log[0], "wait");
    }

    #[test]
    fn no_selection_returns_none_and_restores() {
        let mut f = Fake::new(Some("old clip"), None);
        assert_eq!(copy_selection(&mut f), None);
        assert_eq!(f.text(), Some("old clip"));
    }

    #[test]
    fn whitespace_selection_is_none() {
        let mut f = Fake::new(Some("old"), Some("  \r\n"));
        assert_eq!(copy_selection(&mut f), None);
        assert_eq!(f.text(), Some("old"));
    }

    #[test]
    fn empty_prior_clipboard_is_empty_after() {
        let mut f = Fake::new(None, Some("hi"));
        assert_eq!(copy_selection(&mut f), Some("hi".into()));
        assert_eq!(f.clip, Clip::Empty);
    }

    #[test]
    fn image_on_clipboard_survives_copy_and_paste() {
        let mut f = Fake::new(None, Some("hi"));
        f.clip = Clip::Image(vec![1, 2, 3]);
        assert_eq!(copy_selection(&mut f), Some("hi".into()));
        assert_eq!(f.clip, Clip::Image(vec![1, 2, 3]));
        paste_text(&mut f, "Hi.");
        assert_eq!(f.pasted, vec!["Hi.".to_string()]);
        assert_eq!(f.clip, Clip::Image(vec![1, 2, 3]));
    }

    #[test]
    fn paste_puts_text_then_restores_prior_clipboard() {
        let mut f = Fake::new(Some("old clip"), None);
        paste_text(&mut f, "Hi team, polished.");
        assert_eq!(f.pasted, vec!["Hi team, polished.".to_string()]);
        assert_eq!(f.text(), Some("old clip"));
    }

    #[test]
    fn leave_on_clipboard_keeps_result() {
        let mut f = Fake::new(Some("old clip"), None);
        leave_on_clipboard(&mut f, "result");
        assert_eq!(f.text(), Some("result"));
        assert!(f.pasted.is_empty());
    }
}
