//! Google Gemini CLI (Google account). Never --yolo; read-only approval mode, and every tool
//! denied by a per-job policy file (`--policy`). The project-settings `tools.exclude` list is
//! kept as a second layer, but Gemini 0.63 in plan mode does not honour it on its own: read_file,
//! list_directory, google_web_search and invoke_agent stayed available. See docs/cli-flags.md.
use super::*;
use crate::procutil::{resolve_executable, run};

pub struct GeminiCli { pub exe_override: Option<String> }

impl GeminiCli {
    pub const SETTINGS_JSON: &'static str = r#"{"tools":{"exclude":["run_shell_command","write_file","replace","read_file","read_many_files","glob","search_file_content","grep_search","list_directory","web_fetch","google_web_search","save_memory","invoke_agent","activate_skill","ask_user","write_todos"]}}"#;

    /// Policy-engine rules that deny every built-in and MCP tool. A user-tier `--policy` file
    /// outranks the default plan-mode rules, so denied tools are not even offered to the model.
    pub const DENY_ALL_POLICY: &'static str = r#"[[rule]]
toolName = "*"
decision = "deny"
priority = 999
denyMessage = "Tools are disabled in Retone."

[[rule]]
toolName = "*"
mcpName = "*"
decision = "deny"
priority = 999
denyMessage = "Tools are disabled in Retone."
"#;

    pub fn args(model: Option<&str>, policy_file: &str) -> Vec<String> {
        let mut a: Vec<String> = ["-p", "Follow the instructions above exactly.", "--approval-mode", "plan", "--skip-trust",
                                  "--policy", policy_file].map(String::from).to_vec();
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

fn path_key(p: &str) -> String {
    let s = p.trim_end_matches(['/', '\\']);
    if cfg!(windows) { s.replace('/', "\\").to_lowercase() } else { s.to_string() }
}

/// The per-project folder where Gemini keeps chat recordings for `cwd`, looked up in
/// `<gemini_home>/projects.json` (`{"projects": {"<cwd>": "<slug>"}}`). Only ever returns a
/// direct child of `<gemini_home>/tmp`, and only for an exact match of our own job folder.
pub fn session_dir_for(gemini_home: &std::path::Path, cwd: &std::path::Path) -> Option<std::path::PathBuf> {
    let text = std::fs::read_to_string(gemini_home.join("projects.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let want = path_key(&cwd.to_string_lossy());
    let slug = v["projects"].as_object()?.iter().find(|(k, _)| path_key(k) == want)?.1.as_str()?;
    let mut parts = std::path::Path::new(slug).components();
    match (parts.next(), parts.next()) {
        (Some(std::path::Component::Normal(_)), None) => Some(gemini_home.join("tmp").join(slug)),
        _ => None,
    }
}

/// Gemini reports failures as a JSON object (`{"error":{"message":..}}`) on stderr.
pub fn parse_error(stderr: &str) -> ProviderError {
    let t = stderr.trim();
    let msg = t.find('{').and_then(|i| serde_json::from_str::<serde_json::Value>(&t[i..]).ok())
        .and_then(|v| v["error"]["message"].as_str().map(String::from))
        .unwrap_or_else(|| t.lines().next().unwrap_or("failed").to_string());
    let lower = msg.to_lowercase();
    if lower.contains("auth method") || looks_signed_out(&msg) { return ProviderError::SignedOut; }
    ProviderError::Failed(msg.chars().take(200).collect())
}

#[async_trait]
impl Provider for GeminiCli {
    async fn complete(&self, req: &CompletionRequest) -> Result<String, ProviderError> {
        let exe = self.exe().ok_or(ProviderError::NotInstalled)?;
        let job = tempfile::tempdir().map_err(|e| ProviderError::Failed(e.to_string()))?;
        // Gemini shows the model a listing of its working folder, so keep the policy file outside it.
        // A unique name, since Gemini keys its per-project data folder on the folder name.
        let unique = job.path().file_name().map(|n| n.to_string_lossy().trim_start_matches('.').to_string()).unwrap_or_default();
        let work = job.path().join(format!("retone-{unique}"));
        let dot = work.join(".gemini");
        let policy = job.path().join("policy.toml");
        std::fs::create_dir_all(&dot).and_then(|_| std::fs::write(dot.join("settings.json"), Self::SETTINGS_JSON))
            .and_then(|_| std::fs::write(&policy, Self::DENY_ALL_POLICY))
            .map_err(|e| ProviderError::Failed(e.to_string()))?;
        let stdin = format!("{}

---

{}", req.system, req.user);
        let result = run(&exe, &Self::args(req.model.as_deref(), &policy.to_string_lossy()), &stdin, &work, TIMEOUT).await;
        // Gemini records every session (including the full draft) under ~/.gemini/tmp/<project>/.
        if let Some(gemini_home) = dirs::home_dir().map(|h| h.join(".gemini")) {
            if let Some(dir) = session_dir_for(&gemini_home, &work) { let _ = std::fs::remove_dir_all(dir); }
        }
        let mut out = result.map_err(map_run_err)?;
        if out.status != Some(0) { return Err(parse_error(&out.stderr)); }
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
        let a = GeminiCli::args(Some("gemini-2.5-flash"), "p.toml");
        assert!(!a.iter().any(|x| x == "-y" || x == "--yolo" || x == "yolo"));
        assert_eq!(a, ["-p","Follow the instructions above exactly.","--approval-mode","plan","--skip-trust",
                       "--policy","p.toml","-m","gemini-2.5-flash","--output-format","json"]);
    }
    #[test]
    fn policy_denies_every_tool_including_mcp() {
        let p = GeminiCli::DENY_ALL_POLICY;
        assert_eq!(p.matches("toolName = \"*\"").count(), 2);
        assert!(p.contains("mcpName = \"*\""));
        assert!(!p.contains("allow"));
    }
    #[test]
    fn session_dir_lookup_matches_only_our_job() {
        let home = tempfile::tempdir().unwrap();
        let job = if cfg!(windows) { r"C:\Users\X\AppData\Local\Temp\.tmpAbC12" } else { "/tmp/.tmpAbC12" };
        let key = if cfg!(windows) { job.to_lowercase() } else { job.to_string() };
        let json = serde_json::json!({"projects": {key: "tmpabc12", "/work/proj": "proj", "/evil": "../escape"}});
        std::fs::write(home.path().join("projects.json"), json.to_string()).unwrap();
        let p = std::path::Path::new;
        assert_eq!(session_dir_for(home.path(), p(job)), Some(home.path().join("tmp").join("tmpabc12")));
        assert_eq!(session_dir_for(home.path(), p("/nope")), None);
        assert_eq!(session_dir_for(home.path(), p("/evil")), None);
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
    fn json_errors_are_mapped() {
        let auth = "{
  \"session_id\": \"x\",
  \"error\": {
    \"type\": \"Error\",
    \"message\": \"Please set an Auth method in your settings.json\",
    \"code\": 41
  }
}";
        assert_eq!(parse_error(auth), ProviderError::SignedOut);
        let quota = "{\"error\":{\"message\":\"Quota exceeded for model\",\"code\":429}}";
        assert_eq!(parse_error(quota), ProviderError::Failed("Quota exceeded for model".into()));
        assert_eq!(parse_error("plain failure text
more"), ProviderError::Failed("plain failure text".into()));
    }
    #[test]
    fn settings_disable_tools() {
        let v: serde_json::Value = serde_json::from_str(GeminiCli::SETTINGS_JSON).unwrap();
        assert!(v["tools"]["exclude"].as_array().unwrap().iter().any(|t| t == "run_shell_command"));
    }
}
