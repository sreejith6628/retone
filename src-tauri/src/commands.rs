//! Commands called from the Pill / Options / Settings screens.
use crate::flows::{self, Choice};
use crate::state::AppState;
use crate::{pill, platform, shortcuts, winmgr};
use retone_core::models::{self, ModelInfo};
use retone_core::prompt::{self, Length, Tone};
use retone_core::providers::registry::{build_provider, ProviderId};
use retone_core::providers::{CompletionRequest, ProviderStatus};
use retone_core::settings::{SecretStore, Settings};
use retone_core::{job, pillgeom::Region};
use serde::Serialize;
use std::collections::BTreeMap;
use tauri::{AppHandle, Manager, State};

/// Order shown in Settings and suggested on first run (personal Google users → free Gemini API key).
const ORDER: [ProviderId; 7] = [ProviderId::ClaudeCli, ProviderId::CodexCli, ProviderId::GeminiApi,
    ProviderId::Anthropic, ProviderId::OpenAi, ProviderId::OpenRouter, ProviderId::GeminiCli];

#[derive(Serialize, Clone)]
pub struct ProviderInfo {
    key: &'static str,
    label: &'static str,
    is_cli: bool,
    key_url: Option<&'static str>,
    install_cmd: Option<&'static str>,
    note: Option<&'static str>,
    models: Vec<ModelInfo>,
}

fn info(id: ProviderId) -> ProviderInfo {
    let (install_cmd, note) = match id {
        ProviderId::ClaudeCli => (Some("npm install -g @anthropic-ai/claude-code"), Some("Uses your Claude Pro/Max subscription.")),
        ProviderId::CodexCli => (Some("npm install -g @openai/codex"), Some("Uses your ChatGPT Plus/Pro subscription.")),
        ProviderId::GeminiCli => (Some("npm install -g @google/gemini-cli"),
            Some("Google no longer allows personal Google accounts here — use the free Gemini API key instead. Works with a Gemini API key, Workspace or Vertex AI.")),
        ProviderId::GeminiApi => (None, Some("Free tier available — get a key from Google AI Studio.")),
        _ => (None, None),
    };
    ProviderInfo { key: id.key(), label: id.label(), is_cli: id.is_cli(), key_url: id.key_url(), install_cmd, note,
                   models: models::curated(id.key()) }
}

#[tauri::command]
pub fn provider_list() -> Vec<ProviderInfo> { ORDER.iter().map(|id| info(*id)).collect() }

fn status_str(s: ProviderStatus) -> String {
    match s {
        ProviderStatus::Ready => "ready".into(),
        ProviderStatus::NotInstalled => "not_installed".into(),
        ProviderStatus::SignedOut => "signed_out".into(),
        ProviderStatus::NoKey => "no_key".into(),
        ProviderStatus::Error(e) => format!("error:{e}"),
    }
}

fn provider_id(key: &str) -> Result<ProviderId, String> { ProviderId::from_key(key).ok_or_else(|| format!("unknown provider {key}")) }

#[tauri::command]
pub async fn provider_status(app: AppHandle, key: String) -> Result<String, String> {
    let id = provider_id(&key)?;
    let st = app.state::<AppState>();
    let p = build_provider(id, &st.snapshot(), &st.secrets);
    Ok(status_str(p.status().await))
}

#[tauri::command]
pub async fn test_provider(app: AppHandle, key: String, model: Option<String>, draft: Option<String>) -> Result<String, String> {
    let id = provider_id(&key)?;
    let st = app.state::<AppState>();
    let s = st.snapshot();
    let p = build_provider(id, &s, &st.secrets);
    let built = prompt::build(&prompt::PromptInput { style: &s.style, tone: Tone::Standard, length: Length::Same,
        context: "", draft: draft.as_deref().filter(|d| !d.trim().is_empty())
            .unwrap_or("hi team, pls share the deck by tmrw eod, client meet is on fri") });
    let model = model.filter(|m| !m.is_empty()).or_else(|| models::default_model(&key));
    p.complete(&CompletionRequest { system: built.system, user: built.user, model }).await
        .map_err(|e| job::short_error(&e))
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings { state.snapshot() }

#[derive(Serialize)]
pub struct SaveResult { warnings: Vec<String> }

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) -> Result<SaveResult, String> {
    if Tone::from_name(&settings.default_tone).is_none() { return Err("Unknown tone".into()); }
    provider_id(&settings.default_provider)?;
    shortcuts::validate(&settings.shortcut_quick, &settings.shortcut_options)?;
    let st = app.state::<AppState>();
    let previous = st.snapshot();
    let saved = st.update(|s| {
        let pos = s.pill_pos;
        let first = s.first_run_done;
        *s = settings;
        s.pill_pos = pos;            // owned by the pill, not the form
        s.first_run_done = first || s.first_run_done;
    }).map_err(|e| e.to_string())?;
    let failed = shortcuts::register(&app, &saved);
    if !failed.is_empty() {
        // Keep the user's working shortcuts rather than leaving them with none.
        let _ = st.update(|s| { s.shortcut_quick = previous.shortcut_quick.clone(); s.shortcut_options = previous.shortcut_options.clone(); });
        shortcuts::register(&app, &previous);
        return Err(format!("{} — another app may be using it. Your previous shortcuts are still active.", failed.join("; ")));
    }
    let mut warnings = Vec::new();
    if saved.first_run_done { pill::show(&app, saved.show_pill); }
    if let Err(e) = apply_autostart(&app, saved.launch_at_login) { warnings.push(e); }
    Ok(SaveResult { warnings })
}

fn apply_autostart(app: &AppHandle, on: bool) -> Result<(), String> {
    if cfg!(debug_assertions) { return Ok(()); } // never register a dev build to start at login
    use tauri_plugin_autostart::ManagerExt;
    let al = app.autolaunch();
    let r = if on { al.enable() } else { al.disable() };
    r.map_err(|e| format!("Launch at login: {e}"))
}

#[tauri::command]
pub fn finish_first_run(app: AppHandle) -> Result<(), String> {
    let s = app.state::<AppState>().update(|s| s.first_run_done = true).map_err(|e| e.to_string())?;
    let _ = apply_autostart(&app, s.launch_at_login);
    pill::show(&app, s.show_pill);
    Ok(())
}

#[tauri::command]
pub fn has_api_key(state: State<'_, AppState>, key: String) -> bool {
    provider_id(&key).is_ok() && state.secrets.get(&key).map(|k| !k.is_empty()).unwrap_or(false)
}

#[tauri::command]
pub fn set_api_key(state: State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    provider_id(&key)?;
    let v = value.trim();
    if v.is_empty() { return Err("Key is empty".into()); }
    state.secrets.set(&key, v)
}

#[tauri::command]
pub fn delete_api_key(state: State<'_, AppState>, key: String) -> Result<(), String> {
    provider_id(&key)?;
    state.secrets.delete(&key)
}

/// Open a terminal running the tool's own sign-in. Retone never sees the password.
#[tauri::command]
pub fn sign_in(key: String) -> Result<(), String> {
    let cmd = match key.as_str() {
        "claude-cli" => "claude auth login",
        "codex-cli" => "codex login",
        "gemini-cli" => "gemini",
        _ => return Err("This provider uses an API key".into()),
    };
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("cmd.exe")
            .raw_arg(format!("/c start \"Retone sign-in\" cmd /k {cmd}"))
            .creation_flags(0x0800_0000).spawn().map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        let script = format!("tell application \"Terminal\" to do script \"{cmd}\"\ntell application \"Terminal\" to activate");
        std::process::Command::new("osascript").args(["-e", &script]).spawn().map_err(|e| e.to_string())?;
    }
    let _ = cmd;
    Ok(())
}

#[derive(Serialize)]
pub struct OptionsInit {
    has_draft: bool,
    chars: usize,
    tones: Vec<&'static str>,
    default_tone: String,
    providers: Vec<ProviderInfo>,
    provider: String,
    models: BTreeMap<String, String>,
}

#[tauri::command]
pub fn options_init(state: State<'_, AppState>) -> OptionsInit {
    let s = state.snapshot();
    let draft_chars = state.pending.lock().unwrap().as_ref().map(|p| p.draft.chars().count()).unwrap_or(0);
    OptionsInit {
        has_draft: draft_chars > 0,
        chars: draft_chars,
        tones: Tone::ALL.iter().map(|t| t.name()).collect(),
        default_tone: s.default_tone,
        providers: provider_list(),
        provider: s.default_provider,
        models: s.models,
    }
}

#[tauri::command]
pub fn options_go(app: AppHandle, tone: String, length: String, context: String, provider: String, model: Option<String>)
    -> Result<(), String> {
    let has_draft = app.state::<AppState>().pending.lock().unwrap().as_ref().map(|p| !p.draft.trim().is_empty()).unwrap_or(false);
    if !has_draft && context.trim().is_empty() {
        return Err("Nothing selected — type what the message should say".into());
    }
    let tone = Tone::from_name(&tone).unwrap_or(Tone::Standard);
    let length = match length.as_str() { "shorter" => Length::Shorter, "longer" => Length::Longer, _ => Length::Same };
    provider_id(&provider)?;
    let model = model.filter(|m| !m.is_empty()).or_else(|| models::default_model(&provider));
    flows::options_go(app, tone, length, context, Choice { provider, model });
    Ok(())
}

#[tauri::command]
pub fn options_cancel(app: AppHandle) {
    app.state::<AppState>().pending.lock().unwrap().take();
    winmgr::close(&app, winmgr::OPTIONS);
}

/// macOS: the pill webview reports clicks itself (Windows uses the native hook).
#[tauri::command]
pub fn pill_click(app: AppHandle, region: String) {
    pill::click(&app, if region == "menu" { Region::Menu } else { Region::Main });
}

#[tauri::command]
pub fn pill_right_click(app: AppHandle) { pill::handle_event(&app, platform::PillEvent::RightClick); }

#[tauri::command]
pub fn pill_moved(app: AppHandle, x: i32, y: i32) { pill::handle_event(&app, platform::PillEvent::Moved(x, y)); }

#[tauri::command]
pub fn open_settings(app: AppHandle) { winmgr::open_settings(&app); }

#[tauri::command]
pub fn accessibility_status() -> bool { platform::accessibility_trusted() }

#[tauri::command]
pub fn open_accessibility_settings() { platform::open_accessibility_settings(); }

#[tauri::command]
pub fn platform_name() -> &'static str { if cfg!(target_os = "macos") { "macos" } else { "windows" } }

#[tauri::command]
pub fn default_style() -> &'static str { prompt::DEFAULT_STYLE }
