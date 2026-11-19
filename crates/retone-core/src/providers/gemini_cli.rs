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

/// The final path component, case-folded on Windows. Retone's job folder name
/// (`retone-<random>`) is unique, so matching on it alone is enough, and it survives
/// symlinked temp paths (macOS `/var` vs `/private/var`) and 8.3 short names.
fn last_component(p: &str) -> String {
    let s = p.trim_end_matches(['/', '\\']);
    let name = s.rsplit(['/', '\\']).next().unwrap_or(s);
    if cfg!(windows) { name.to_lowercase() } else { name.to_string() }
}

/// A slug Gemini could have produced for one of Retone's job folders: `retone-` followed by
/// lowercase letters, digits and dashes only. Rejects `.`, `..`, `...`, trailing dots/spaces,
/// drive letters and separators, so the result can only ever name a folder of our own.
fn is_retone_slug(slug: &str) -> bool {
    slug.len() > "retone-".len() && slug.starts_with("retone-")
        && slug.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// Gemini's home folder: `$GEMINI_CLI_HOME/.gemini` when set (Gemini honours it), else `~/.gemini`.
pub fn gemini_home() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("GEMINI_CLI_HOME").filter(|v| !v.is_empty()).map(std::path::PathBuf::from)
        .or_else(dirs::home_dir)?;
    Some(base.join(".gemini"))
}

/// The per-project folders where Gemini keeps chat recordings and history for `cwd`, looked up in
/// `<gemini_home>/projects.json` (`{"projects": {"<cwd>": "<slug>"}}`). Returns
/// `<gemini_home>/tmp/<slug>` and `<gemini_home>/history/<slug>`, only for an entry whose folder
/// name equals our job folder's and whose slug passes `is_retone_slug`.
pub fn session_dirs_for(gemini_home: &std::path::Path, cwd: &std::path::Path) -> Vec<std::path::PathBuf> {
    let Ok(text) = std::fs::read_to_string(gemini_home.join("projects.json")) else { return vec![] };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { return vec![] };
    let want = last_component(&cwd.to_string_lossy());
    if !want.starts_with("retone-") { return vec![]; }
    let Some(projects) = v["projects"].as_object() else { return vec![] };
    projects.iter()
        .filter(|(k, _)| last_component(k) == want)
        .filter_map(|(_, slug)| slug.as_str())
        .filter(|slug| is_retone_slug(slug))
        .flat_map(|slug| [gemini_home.join("tmp").join(slug), gemini_home.join("history").join(slug)])
        .collect()
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
        // Best effort: if Gemini's registry can't be read, the recording stays (see README).
        if let Some(home) = gemini_home() {
            for dir in session_dirs_for(&home, &work) { let _ = std::fs::remove_dir_all(dir); }
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
        let h = home.path();
        // Gemini records the physical path (macOS /private/var); Retone knows the /var one.
        let key = if cfg!(windows) { r"c:\users\x\appdata\local\temp\.tmpabc\retone-tmpabc12" }
                  else { "/private/var/folders/xy/T/.tmpAbC/retone-tmpAbC12" };
        let job = if cfg!(windows) { r"C:\Users\X\AppData\Local\Temp\.tmpAbC\retone-tmpAbC12" }
                  else { "/var/folders/xy/T/.tmpAbC/retone-tmpAbC12" };
        let json = serde_json::json!({"projects": {
            key: "retone-tmpabc12", "/work/proj": "proj", "/evil/retone-x": "../escape",
            "/a/retone-dots": "...", "/a/retone-sp": " ", "/a/retone-trail": "retone-a.", "/a/retone-drive": "C:",
            "/a/retone-colon": "a:b", "/a/retone-up": "Retone-UP"}});
        std::fs::write(h.join("projects.json"), json.to_string()).unwrap();
        let p = std::path::Path::new;
        assert_eq!(session_dirs_for(h, p(job)),
                   vec![h.join("tmp").join("retone-tmpabc12"), h.join("history").join("retone-tmpabc12")]);
        assert!(session_dirs_for(h, p("/nope")).is_empty());
        assert!(session_dirs_for(h, p("/work/proj")).is_empty());
        for bad in ["/x/retone-x", "/x/retone-dots", "/x/retone-sp", "/x/retone-trail", "/x/retone-drive",
                    "/x/retone-colon", "/x/retone-up"] {
            assert!(session_dirs_for(h, p(bad)).is_empty(), "{bad}");
        }
        for slug in ["...", " ", ". .", "retone-", "retone-a.", "retone-a ", "C:", "a:b", "retone-../x", "proj"] {
            assert!(!is_retone_slug(slug), "{slug}");
        }
        assert!(is_retone_slug("retone-tmpabc12-1"));
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
