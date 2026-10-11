//! User settings (JSON) and API-key storage (OS keychain). Keys never go in the JSON.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::Mutex;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub default_provider: String,
    pub models: BTreeMap<String, String>,
    pub default_tone: String,
    pub style: String,
    pub show_pill: bool,
    pub launch_at_login: bool,
    pub pill_pos: Option<(i32, i32)>,
    pub shortcut_quick: String,
    pub shortcut_options: String,
    pub cli_paths: BTreeMap<String, String>,
    pub first_run_done: bool,
}

impl Default for Settings {
    fn default() -> Self {
        let mac = cfg!(target_os = "macos");
        Settings {
            default_provider: "claude-cli".into(),
            models: BTreeMap::new(),
            default_tone: crate::prompt::Tone::Standard.name().into(),
            style: crate::prompt::DEFAULT_STYLE.into(),
            show_pill: true,
            launch_at_login: true,
            pill_pos: None,
            shortcut_quick: if mac { "Alt+Super+P" } else { "Ctrl+Shift+P" }.into(),
            shortcut_options: if mac { "Alt+Super+O" } else { "Ctrl+Shift+O" }.into(),
            cli_paths: BTreeMap::new(),
            first_run_done: false,
        }
    }
}

pub fn load(path: &Path) -> Settings {
    let Ok(text) = std::fs::read_to_string(path) else { return Settings::default() };
    match serde_json::from_str(&text) {
        Ok(s) => s,
        Err(_) => {
            let mut bak = path.as_os_str().to_owned();
            bak.push(".bak");
            let _ = std::fs::write(&bak, &text);
            Settings::default()
        }
    }
}

pub fn save(path: &Path, s: &Settings) -> std::io::Result<()> {
    if let Some(dir) = path.parent() { std::fs::create_dir_all(dir)?; }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(s).expect("settings serialize"))?;
    std::fs::rename(tmp, path)
}

pub trait SecretStore: Send + Sync {
    fn get(&self, key: &str) -> Option<String>;
    fn set(&self, key: &str, val: &str) -> Result<(), String>;
    fn delete(&self, key: &str) -> Result<(), String>;
}

const SERVICE: &str = "app.retone.desktop";

pub struct KeyringStore;

impl SecretStore for KeyringStore {
    fn get(&self, key: &str) -> Option<String> { keyring::Entry::new(SERVICE, key).ok()?.get_password().ok() }
    fn set(&self, key: &str, val: &str) -> Result<(), String> {
        keyring::Entry::new(SERVICE, key).and_then(|e| e.set_password(val)).map_err(|e| e.to_string())
    }
    fn delete(&self, key: &str) -> Result<(), String> {
        match keyring::Entry::new(SERVICE, key).and_then(|e| e.delete_credential()) {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[derive(Default)]
pub struct MemoryStore(Mutex<HashMap<String, String>>);

impl SecretStore for MemoryStore {
    fn get(&self, key: &str) -> Option<String> { self.0.lock().unwrap().get(key).cloned() }
    fn set(&self, key: &str, val: &str) -> Result<(), String> { self.0.lock().unwrap().insert(key.into(), val.into()); Ok(()) }
    fn delete(&self, key: &str) -> Result<(), String> { self.0.lock().unwrap().remove(key); Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_loads_defaults() {
        let d = tempfile::tempdir().unwrap();
        let s = load(&d.path().join("settings.json"));
        assert_eq!(s, Settings::default());
        assert_eq!(s.default_provider, "claude-cli");
        assert_eq!(s.default_tone, "Standard");
        assert_eq!(s.style, crate::prompt::DEFAULT_STYLE);
        assert!(s.show_pill);
    }

    #[test]
    fn roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("settings.json");
        let mut s = Settings::default();
        s.default_provider = "codex-cli".into();
        s.models.insert("codex-cli".into(), "gpt-5-mini".into());
        s.pill_pos = Some((1561, 458));
        save(&p, &s).unwrap();
        assert_eq!(load(&p), s);
    }

    #[test]
    fn corrupt_file_loads_defaults_and_backs_up() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("settings.json");
        std::fs::write(&p, "{ not json").unwrap();
        assert_eq!(load(&p), Settings::default());
        assert_eq!(std::fs::read_to_string(d.path().join("settings.json.bak")).unwrap(), "{ not json");
    }

    #[test]
    fn unknown_and_missing_fields_tolerated() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("settings.json");
        std::fs::write(&p, r#"{"default_tone":"Friendly","future_field":1}"#).unwrap();
        let s = load(&p);
        assert_eq!(s.default_tone, "Friendly");
        assert_eq!(s.default_provider, "claude-cli");
    }

    #[test]
    fn memory_store() {
        let m = MemoryStore::default();
        assert_eq!(m.get("openai"), None);
        m.set("openai", "sk-1").unwrap();
        assert_eq!(m.get("openai").as_deref(), Some("sk-1"));
        m.delete("openai").unwrap();
        assert_eq!(m.get("openai"), None);
    }
}
