//! One polish job, start to finish: prompt → provider → paste, with status shown in the pill.
use crate::prompt::{self, Length, Tone};
use crate::providers::{CompletionRequest, Provider, ProviderError};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, PartialEq)]
pub enum PillStatus { Idle, Working, Done, Hint(String), Error(String), Copied }

pub trait StatusSink: Send + Sync { fn status(&self, s: PillStatus); }

/// The desktop side of a job: paste into the app the text came from.
pub trait Desktop: Send {
    /// Paste into the original target. `false` = target gone; the result was left on the clipboard.
    fn paste_into_target(&mut self, text: &str) -> bool;
}

#[derive(Debug, Clone)]
pub struct JobRequest {
    pub draft: String,
    pub tone: Tone,
    pub length: Length,
    pub context: String,
    pub style: String,
    pub model: Option<String>,
}

/// Only one job at a time.
#[derive(Default)]
pub struct Busy(AtomicBool);
pub struct BusyGuard<'a>(&'a Busy);
impl Busy {
    pub fn try_start(&self) -> Option<BusyGuard<'_>> {
        (!self.0.swap(true, Ordering::SeqCst)).then_some(BusyGuard(self))
    }
    pub fn is_busy(&self) -> bool { self.0.load(Ordering::SeqCst) }
}
impl Drop for BusyGuard<'_> { fn drop(&mut self) { self.0 .0.store(false, Ordering::SeqCst); } }

pub const HINT_NOTHING_SELECTED: &str = "Select text first — or ▼ to write";

pub fn short_error(e: &ProviderError) -> String {
    let s = match e {
        ProviderError::NotInstalled => "AI tool not installed — click to open Settings".to_string(),
        ProviderError::SignedOut => "Signed out — click to open Settings".to_string(),
        ProviderError::NoKey => "No API key — click to open Settings".to_string(),
        ProviderError::Auth(_) => "Key rejected — click to open Settings".to_string(),
        ProviderError::Timeout => "Timed out — try again".to_string(),
        ProviderError::EmptyOutput => "Empty response — try again".to_string(),
        ProviderError::Failed(m) => m.clone(),
    };
    if s.chars().count() > 60 { format!("{}…", s.chars().take(59).collect::<String>()) } else { s }
}

pub async fn run(req: JobRequest, provider: &dyn Provider, desktop: &mut dyn Desktop, sink: &dyn StatusSink)
    -> Result<(), ProviderError> {
    if req.draft.trim().is_empty() && req.context.trim().is_empty() {
        sink.status(PillStatus::Hint(HINT_NOTHING_SELECTED.into()));
        return Ok(());
    }
    sink.status(PillStatus::Working);
    let built = prompt::build(&prompt::PromptInput {
        style: &req.style, tone: req.tone, length: req.length, context: &req.context, draft: &req.draft });
    let creq = CompletionRequest { system: built.system, user: built.user, model: req.model.clone() };
    match provider.complete(&creq).await {
        Err(e) => { sink.status(PillStatus::Error(short_error(&e))); Err(e) }
        Ok(text) => {
            sink.status(if desktop.paste_into_target(&text) { PillStatus::Done } else { PillStatus::Copied });
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{CompletionRequest, Provider, ProviderError, ProviderStatus};
    use std::sync::Mutex;

    struct FakeProvider(Result<String, ProviderError>, Mutex<Option<CompletionRequest>>);
    #[async_trait::async_trait]
    impl Provider for FakeProvider {
        async fn complete(&self, req: &CompletionRequest) -> Result<String, ProviderError> {
            *self.1.lock().unwrap() = Some(req.clone()); self.0.clone()
        }
        async fn status(&self) -> ProviderStatus { ProviderStatus::Ready }
    }
    #[derive(Default)] struct Sink(Mutex<Vec<PillStatus>>);
    impl StatusSink for Sink { fn status(&self, s: PillStatus) { self.0.lock().unwrap().push(s); } }
    struct Desk { pasted: Vec<String>, target_alive: bool }
    impl Desktop for Desk {
        fn paste_into_target(&mut self, text: &str) -> bool { if self.target_alive { self.pasted.push(text.into()); } self.target_alive }
    }
    fn req(draft: &str, context: &str) -> JobRequest {
        JobRequest { draft: draft.into(), tone: Tone::Standard, length: Length::Same, context: context.into(),
                     style: crate::prompt::DEFAULT_STYLE.into(), model: Some("haiku".into()) }
    }

    #[tokio::test]
    async fn happy_path_pastes_and_ends_done() {
        let p = FakeProvider(Ok("Hi team".into()), Mutex::new(None));
        let (sink, mut d) = (Sink::default(), Desk { pasted: vec![], target_alive: true });
        run(req("hi tem", ""), &p, &mut d, &sink).await.unwrap();
        assert_eq!(d.pasted, vec!["Hi team"]);
        assert_eq!(*sink.0.lock().unwrap(), vec![PillStatus::Working, PillStatus::Done]);
        let sent = p.1.lock().unwrap().clone().unwrap();
        assert_eq!(sent.user, "<draft>\nhi tem\n</draft>");
        assert_eq!(sent.model.as_deref(), Some("haiku"));
    }

    #[tokio::test]
    async fn provider_error_pastes_nothing() {
        let p = FakeProvider(Err(ProviderError::SignedOut), Mutex::new(None));
        let (sink, mut d) = (Sink::default(), Desk { pasted: vec![], target_alive: true });
        assert_eq!(run(req("hi", ""), &p, &mut d, &sink).await, Err(ProviderError::SignedOut));
        assert!(d.pasted.is_empty());
        assert_eq!(sink.0.lock().unwrap().last(), Some(&PillStatus::Error("Signed out — click to open Settings".into())));
    }

    #[tokio::test]
    async fn paste_target_gone_leaves_result_on_clipboard() {
        let p = FakeProvider(Ok("Hi".into()), Mutex::new(None));
        let (sink, mut d) = (Sink::default(), Desk { pasted: vec![], target_alive: false });
        run(req("hi", ""), &p, &mut d, &sink).await.unwrap();
        assert_eq!(sink.0.lock().unwrap().last(), Some(&PillStatus::Copied));
    }

    #[tokio::test]
    async fn nothing_to_do_is_a_hint() {
        let p = FakeProvider(Ok("x".into()), Mutex::new(None));
        let (sink, mut d) = (Sink::default(), Desk { pasted: vec![], target_alive: true });
        run(req("  ", " "), &p, &mut d, &sink).await.unwrap();
        assert_eq!(*sink.0.lock().unwrap(), vec![PillStatus::Hint("Select text first — or ▼ to write".into())]);
        assert!(p.1.lock().unwrap().is_none());
    }

    #[test]
    fn busy_rejects_second_job() {
        let b = Busy::default();
        let g = b.try_start();
        assert!(g.is_some());
        assert!(b.try_start().is_none());
        drop(g);
        assert!(b.try_start().is_some());
    }

    #[test]
    fn short_errors_are_short() {
        assert_eq!(short_error(&ProviderError::NoKey), "No API key — click to open Settings");
        let long = ProviderError::Failed("x".repeat(300));
        assert!(short_error(&long).chars().count() <= 60);
    }
}
