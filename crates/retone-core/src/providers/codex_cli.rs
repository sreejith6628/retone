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

#[async_trait]
impl Provider for CodexCli {
    async fn complete(&self, req: &CompletionRequest) -> Result<String, ProviderError> {
        let exe = self.exe().ok_or(ProviderError::NotInstalled)?;
        let job = tempfile::tempdir().map_err(|e| ProviderError::Failed(e.to_string()))?;
        let stdin = format!("{}

---

{}", req.system, req.user);
        let mut out = run(&exe, &Self::args(req.model.as_deref(), "out.txt"), &stdin, job.path(), TIMEOUT)
            .await.map_err(map_run_err)?;
        if out.status == Some(0) {
            if let Ok(last) = std::fs::read_to_string(job.path().join("out.txt")) { out.stdout = last; }
        }
        finish_cli(out)
    }
    async fn status(&self) -> ProviderStatus {
        let Some(exe) = self.exe() else { return ProviderStatus::NotInstalled };
        let Ok(job) = tempfile::tempdir() else { return ProviderStatus::Error("temp dir".into()) };
        match run(&exe, &["login".into(), "status".into()], "", job.path(), std::time::Duration::from_secs(15)).await {
            Ok(o) if o.status == Some(0) && !looks_signed_out(&o.stdout) => ProviderStatus::Ready,
            Ok(_) => ProviderStatus::SignedOut,
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
}
