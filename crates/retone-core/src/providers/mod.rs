//! Provider abstraction: every backend takes (system, user, model) and returns text.
use async_trait::async_trait;
use std::time::Duration;

pub mod claude_cli;
pub mod codex_cli;
pub mod gemini_cli;
pub mod registry;

pub const TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
pub struct CompletionRequest { pub system: String, pub user: String, pub model: Option<String> }

#[derive(Debug, Clone, PartialEq)]
pub enum ProviderStatus { Ready, NotInstalled, SignedOut, NoKey, Error(String) }

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
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
    ["failed to authenticate", "not logged in", "run /login", "please log in", "please login",
     "codex login", "session expired", "auth method"]
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

pub mod anthropic_api;
pub mod openai_compat;

pub(crate) async fn post_json(url: &str, headers: &[(&str, String)], body: serde_json::Value) -> Result<serde_json::Value, ProviderError> {
    // No redirects: reqwest only strips `Authorization` on cross-host redirects, not `x-api-key`.
    let client = reqwest::Client::builder().timeout(TIMEOUT).redirect(reqwest::redirect::Policy::none())
        .build().map_err(|e| ProviderError::Failed(e.to_string()))?;
    let mut rb = client.post(url).json(&body);
    for (k, v) in headers { rb = rb.header(*k, v); }
    let resp = rb.send().await.map_err(|e| if e.is_timeout() { ProviderError::Timeout } else { ProviderError::Failed(e.to_string()) })?;
    let status = resp.status();
    let v: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);
    if status.is_success() { return Ok(v); }
    let msg = v["error"]["message"].as_str().map(String::from).unwrap_or_else(|| format!("HTTP {status}"));
    if status.as_u16() == 401 || status.as_u16() == 403 { Err(ProviderError::Auth(msg)) } else { Err(ProviderError::Failed(msg)) }
}

pub(crate) const TRUNCATED: &str = "response was cut off (too long)";

pub(crate) fn non_empty(s: Option<&str>) -> Result<String, ProviderError> {
    let t = clean_output(s.unwrap_or(""));
    if t.is_empty() { Err(ProviderError::EmptyOutput) } else { Ok(t) }
}
