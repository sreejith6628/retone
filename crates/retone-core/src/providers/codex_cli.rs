//! OpenAI Codex CLI (ChatGPT subscription). Read-only sandbox; see docs/cli-flags.md.
use super::*;
use crate::procutil::{resolve_executable, run};

pub struct CodexCli { pub exe_override: Option<String> }

impl CodexCli {
    pub fn args(model: Option<&str>, out_file: &str) -> Vec<String> {
        let mut a: Vec<String> = ["exec", "--skip-git-repo-check", "--ephemeral", "--ignore-user-config",
                                  "--sandbox", "read-only", "--color", "never"].map(String::from).to_vec();
        if let Some(m) = model.filter(|m| !m.is_empty()) { a.push("-m".into()); a.push(m.into()); }
        a.extend(["--output-last-message", out_file, "-"].map(String::from));
        a
    }
    fn exe(&self) -> Option<std::path::PathBuf> { resolve_executable("codex", self.exe_override.as_deref()) }
}

/// Map a codex run + its `--output-last-message` file to a result.
pub fn codex_result(mut out: crate::procutil::RunOutput, last_message: Option<String>) -> Result<String, ProviderError> {
    if out.status == Some(0) {
        match last_message {
            Some(last) => out.stdout = last,
            None => return Err(ProviderError::Failed("Codex returned no message".into())),
        }
    }
    finish_cli(out)
}

/// `codex login status` prints its result on stderr (codex 0.162), so read both streams.
pub fn codex_status_from(o: &crate::procutil::RunOutput) -> ProviderStatus {
    if o.status == Some(0) && codex_logged_in(&format!("{}
{}", o.stdout, o.stderr)) { ProviderStatus::Ready }
    else { ProviderStatus::SignedOut }
}

pub fn codex_logged_in(status_stdout: &str) -> bool {
    let t = status_stdout.to_lowercase();
    t.contains("logged in") && !t.contains("not logged in")
}

#[async_trait]
impl Provider for CodexCli {
    async fn complete(&self, req: &CompletionRequest) -> Result<String, ProviderError> {
        let exe = self.exe().ok_or(ProviderError::NotInstalled)?;
        let job = tempfile::tempdir().map_err(|e| ProviderError::Failed(e.to_string()))?;
        let stdin = format!("{}

---

{}", req.system, req.user);
        let out = run(&exe, &Self::args(req.model.as_deref(), "out.txt"), &stdin, job.path(), TIMEOUT)
            .await.map_err(map_run_err)?;
        codex_result(out, std::fs::read_to_string(job.path().join("out.txt")).ok())
    }
    async fn status(&self) -> ProviderStatus {
        let Some(exe) = self.exe() else { return ProviderStatus::NotInstalled };
        let Ok(job) = tempfile::tempdir() else { return ProviderStatus::Error("temp dir".into()) };
        match run(&exe, &["login".into(), "status".into()], "", job.path(), std::time::Duration::from_secs(15)).await {
            Ok(o) => codex_status_from(&o),
            Err(e) => ProviderStatus::Error(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn args_are_read_only_ephemeral_and_skip_git() {
        assert_eq!(CodexCli::args(Some("gpt-5-mini"), "out.txt"),
            ["exec","--skip-git-repo-check","--ephemeral","--ignore-user-config","--sandbox","read-only","--color","never",
             "-m","gpt-5-mini","--output-last-message","out.txt","-"]);
        assert!(!CodexCli::args(None, "out.txt").contains(&"-m".to_string()));
    }
    #[test]
    fn missing_last_message_is_failure_not_banner() {
        let ok = |o: &str| crate::procutil::RunOutput { status: Some(0), stdout: o.into(), stderr: String::new() };
        assert_eq!(codex_result(ok("OpenAI Codex v0.162
model: gpt-5
sandbox: read-only"), None),
                   Err(ProviderError::Failed("Codex returned no message".into())));
        assert_eq!(codex_result(ok("banner"), Some(" Hi team 
".into())), Ok("Hi team".into()));
        assert_eq!(codex_result(ok("banner"), Some("  ".into())), Err(ProviderError::EmptyOutput));
    }
    #[test]
    fn login_status_read_from_stderr_too() {
        let o = crate::procutil::RunOutput { status: Some(0), stdout: String::new(), stderr: "Logged in using ChatGPT
".into() };
        assert_eq!(codex_status_from(&o), ProviderStatus::Ready);
        let o = crate::procutil::RunOutput { status: Some(1), stdout: String::new(), stderr: "Not logged in".into() };
        assert_eq!(codex_status_from(&o), ProviderStatus::SignedOut);
    }
    #[test]
    fn login_status_requires_positive_phrase() {
        assert!(codex_logged_in("Logged in using ChatGPT"));
        assert!(codex_logged_in("Logged in using an API key - sk-***"));
        assert!(!codex_logged_in("Not logged in"));
    }
}
