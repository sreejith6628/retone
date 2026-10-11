//! Claude Code CLI (subscription). Flags verified against claude 2.1.x in the AHK tool.
use super::*;
use crate::procutil::{resolve_executable, run};

pub struct ClaudeCli { pub exe_override: Option<String> }

impl ClaudeCli {
    pub fn args(model: Option<&str>, system_file: &str) -> Vec<String> {
        let mut a = vec!["-p".to_string()];
        if let Some(m) = model.filter(|m| !m.is_empty()) { a.push("--model".into()); a.push(m.into()); }
        a.extend(["--tools", "", "--setting-sources", "", "--strict-mcp-config", "--no-session-persistence",
                  "--system-prompt-file", system_file].map(String::from));
        a
    }
    fn exe(&self) -> Option<std::path::PathBuf> { resolve_executable("claude", self.exe_override.as_deref()) }
}

#[async_trait]
impl Provider for ClaudeCli {
    async fn complete(&self, req: &CompletionRequest) -> Result<String, ProviderError> {
        let exe = self.exe().ok_or(ProviderError::NotInstalled)?;
        let job = tempfile::tempdir().map_err(|e| ProviderError::Failed(e.to_string()))?;
        std::fs::write(job.path().join("sys.txt"), &req.system).map_err(|e| ProviderError::Failed(e.to_string()))?;
        let out = run(&exe, &Self::args(req.model.as_deref(), "sys.txt"), &req.user, job.path(), TIMEOUT)
            .await.map_err(map_run_err)?;
        finish_cli(out)
    }
    async fn status(&self) -> ProviderStatus {
        let Some(exe) = self.exe() else { return ProviderStatus::NotInstalled };
        let Ok(job) = tempfile::tempdir() else { return ProviderStatus::Error("temp dir".into()) };
        match run(&exe, &["auth".into(), "status".into()], "", job.path(), std::time::Duration::from_secs(15)).await {
            Ok(o) => match serde_json::from_str::<serde_json::Value>(o.stdout.trim()) {
                Ok(v) if v["loggedIn"] == true => ProviderStatus::Ready,
                Ok(_) => ProviderStatus::SignedOut,
                Err(_) => ProviderStatus::Error(clean_output(&o.stderr)),
            },
            Err(e) => ProviderStatus::Error(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn args_disable_tools_and_settings() {
        let a = ClaudeCli::args(Some("haiku"), "sys.txt");
        assert_eq!(a, ["-p","--model","haiku","--tools","","--setting-sources","","--strict-mcp-config",
                       "--no-session-persistence","--system-prompt-file","sys.txt"]);
        let b = ClaudeCli::args(None, "sys.txt");
        assert!(!b.contains(&"--model".to_string()));
    }
    #[test]
    fn signed_out_detection() {
        assert!(crate::providers::looks_signed_out("Failed to authenticate: OAuth session expired"));
        assert!(crate::providers::looks_signed_out("Please run /login"));
        assert!(!crate::providers::looks_signed_out("Hi Rahul, please share the SOW."));
    }
    #[test]
    fn clean_output_trims_and_normalises() {
        assert_eq!(crate::providers::clean_output("\r\n  Hi\r\nthere \r\n"), "Hi\nthere");
    }
}
