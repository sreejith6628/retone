//! Google Gemini CLI (Google account). Never --yolo; read-only approval mode and tools
//! excluded via per-job project settings (honoured thanks to --skip-trust). See docs/cli-flags.md.
use super::*;
use crate::procutil::{resolve_executable, run};

pub struct GeminiCli { pub exe_override: Option<String> }

impl GeminiCli {
    pub const SETTINGS_JSON: &'static str = r#"{"tools":{"exclude":["run_shell_command","write_file","replace","read_file","read_many_files","glob","search_file_content","list_directory","web_fetch","google_web_search","save_memory"]}}"#;

    pub fn args(model: Option<&str>) -> Vec<String> {
        let mut a: Vec<String> = ["-p", "Follow the instructions above exactly.", "--approval-mode", "plan", "--skip-trust"]
            .map(String::from).to_vec();
        if let Some(m) = model.filter(|m| !m.is_empty()) { a.push("-m".into()); a.push(m.into()); }
        a.extend(["--output-format", "json"].map(String::from));
        a
    }
    fn exe(&self) -> Option<std::path::PathBuf> { resolve_executable("gemini", self.exe_override.as_deref()) }
}

const NOISE: [&str; 2] = ["Loaded cached credentials.", "Data collection is disabled."];

pub fn parse_output(stdout: &str) -> String {
    let body: String = stdout.lines().filter(|l| !NOISE.contains(&l.trim())).collect::<Vec<_>>().join("
");
    let trimmed = body.trim();
    if let Some(start) = trimmed.find('{') {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&trimmed[start..]) {
            if let Some(r) = v["response"].as_str() { return clean_output(r); }
        }
    }
    clean_output(trimmed)
}

#[async_trait]
impl Provider for GeminiCli {
    async fn complete(&self, req: &CompletionRequest) -> Result<String, ProviderError> {
        let exe = self.exe().ok_or(ProviderError::NotInstalled)?;
        let job = tempfile::tempdir().map_err(|e| ProviderError::Failed(e.to_string()))?;
        let dot = job.path().join(".gemini");
        std::fs::create_dir_all(&dot).and_then(|_| std::fs::write(dot.join("settings.json"), Self::SETTINGS_JSON))
            .map_err(|e| ProviderError::Failed(e.to_string()))?;
        let stdin = format!("{}

---

{}", req.system, req.user);
        let mut out = run(&exe, &Self::args(req.model.as_deref()), &stdin, job.path(), TIMEOUT).await.map_err(map_run_err)?;
        out.stdout = parse_output(&out.stdout);
        finish_cli(out)
    }
    async fn status(&self) -> ProviderStatus {
        if self.exe().is_none() { return ProviderStatus::NotInstalled; }
        let req = CompletionRequest { system: "Reply with exactly: OK".into(), user: "ping".into(), model: None };
        match self.complete(&req).await {
            Ok(_) => ProviderStatus::Ready,
            Err(ProviderError::SignedOut) => ProviderStatus::SignedOut,
            Err(e) => ProviderStatus::Error(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn args_never_yolo_and_read_only() {
        let a = GeminiCli::args(Some("gemini-2.5-flash"));
        assert!(!a.iter().any(|x| x == "-y" || x == "--yolo" || x == "yolo"));
        assert_eq!(a, ["-p","Follow the instructions above exactly.","--approval-mode","plan","--skip-trust",
                       "-m","gemini-2.5-flash","--output-format","json"]);
    }
    #[test]
    fn parse_json_and_noise() {
        assert_eq!(parse_output("Loaded cached credentials.
{\"response\":\"Hi team\"}"), "Hi team");
        assert_eq!(parse_output("Loaded cached credentials.
Hi team
"), "Hi team");
    }
    #[test]
    fn settings_disable_tools() {
        let v: serde_json::Value = serde_json::from_str(GeminiCli::SETTINGS_JSON).unwrap();
        assert!(v["tools"]["exclude"].as_array().unwrap().iter().any(|t| t == "run_shell_command"));
    }
}
