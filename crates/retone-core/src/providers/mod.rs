//! Provider abstraction: every backend takes (system, user, model) and returns text.
use async_trait::async_trait;
use std::time::Duration;

pub mod claude_cli;

pub const TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
pub struct CompletionRequest { pub system: String, pub user: String, pub model: Option<String> }

#[derive(Debug, Clone, PartialEq)]
pub enum ProviderStatus { Ready, NotInstalled, SignedOut, NoKey, Error(String) }

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ProviderError {
    #[error("not installed")] NotInstalled,
    #[error("signed out — sign in again")] SignedOut,
    #[error("no API key set")] NoKey,
    #[error("authentication failed: {0}")] Auth(String),
    #[error("timed out")] Timeout,
    #[error("{0}")] Failed(String),
    #[error("empty response")] EmptyOutput,
}

#[async_trait]
pub trait Provider: Send + Sync {
    async fn complete(&self, req: &CompletionRequest) -> Result<String, ProviderError>;
    async fn status(&self) -> ProviderStatus;
}

pub fn clean_output(s: &str) -> String { s.replace("\r\n", "\n").trim().to_string() }

pub fn looks_signed_out(text: &str) -> bool {
    let t = text.to_lowercase();
    ["failed to authenticate", "not logged in", "/login", "please log in", "please login", "oauth", "unauthorized", "sign in"]
        .iter().any(|k| t.contains(k))
}

/// Shared tail for CLI providers: map a RunOutput to a result.
pub(crate) fn finish_cli(out: crate::procutil::RunOutput) -> Result<String, ProviderError> {
    let text = clean_output(&out.stdout);
    if out.status != Some(0) {
        let msg = if out.stderr.trim().is_empty() { text.clone() } else { clean_output(&out.stderr) };
        if looks_signed_out(&msg) { return Err(ProviderError::SignedOut); }
        return Err(ProviderError::Failed(msg.lines().next().unwrap_or("failed").chars().take(200).collect()));
    }
    if text.is_empty() { return Err(ProviderError::EmptyOutput); }
    Ok(text)
}

pub(crate) fn map_run_err(e: crate::procutil::RunError) -> ProviderError {
    match e { crate::procutil::RunError::Timeout => ProviderError::Timeout, crate::procutil::RunError::Spawn(m) => ProviderError::Failed(m) }
}
