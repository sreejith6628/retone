//! Maps a provider id + settings + secrets to a ready-to-use Provider.
use super::*;
use crate::settings::{SecretStore, Settings};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderId { ClaudeCli, CodexCli, GeminiCli, Anthropic, OpenAi, GeminiApi, OpenRouter }

impl ProviderId {
    pub const ALL: [ProviderId; 7] = [ProviderId::ClaudeCli, ProviderId::CodexCli, ProviderId::GeminiCli,
        ProviderId::Anthropic, ProviderId::OpenAi, ProviderId::GeminiApi, ProviderId::OpenRouter];

    pub fn key(self) -> &'static str {
        match self {
            ProviderId::ClaudeCli => "claude-cli", ProviderId::CodexCli => "codex-cli", ProviderId::GeminiCli => "gemini-cli",
            ProviderId::Anthropic => "anthropic", ProviderId::OpenAi => "openai", ProviderId::GeminiApi => "gemini-api",
            ProviderId::OpenRouter => "openrouter",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            ProviderId::ClaudeCli => "Claude (subscription)", ProviderId::CodexCli => "ChatGPT (subscription)",
            ProviderId::GeminiCli => "Gemini (Google account)", ProviderId::Anthropic => "Anthropic API key",
            ProviderId::OpenAi => "OpenAI API key", ProviderId::GeminiApi => "Gemini API key (free tier)",
            ProviderId::OpenRouter => "OpenRouter API key",
        }
    }
    pub fn from_key(k: &str) -> Option<ProviderId> { ProviderId::ALL.into_iter().find(|p| p.key() == k) }
    pub fn is_cli(self) -> bool { matches!(self, ProviderId::ClaudeCli | ProviderId::CodexCli | ProviderId::GeminiCli) }
    pub fn key_url(self) -> Option<&'static str> {
        match self {
            ProviderId::Anthropic => Some("https://console.anthropic.com/settings/keys"),
            ProviderId::OpenAi => Some("https://platform.openai.com/api-keys"),
            ProviderId::GeminiApi => Some("https://aistudio.google.com/apikey"),
            ProviderId::OpenRouter => Some("https://openrouter.ai/keys"),
            _ => None,
        }
    }
}

pub fn build_provider(id: ProviderId, s: &Settings, secrets: &dyn SecretStore) -> Box<dyn Provider> {
    let exe = s.cli_paths.get(id.key()).cloned();
    let key = || secrets.get(id.key());
    match id {
        ProviderId::ClaudeCli => Box::new(claude_cli::ClaudeCli { exe_override: exe }),
        ProviderId::CodexCli => Box::new(codex_cli::CodexCli { exe_override: exe }),
        ProviderId::GeminiCli => Box::new(gemini_cli::GeminiCli { exe_override: exe }),
        ProviderId::Anthropic => Box::new(anthropic_api::AnthropicApi { base_url: "https://api.anthropic.com".into(), api_key: key() }),
        ProviderId::OpenAi => Box::new(openai_compat::OpenAiCompat { base_url: "https://api.openai.com/v1".into(), api_key: key() }),
        ProviderId::GeminiApi => Box::new(openai_compat::OpenAiCompat {
            base_url: "https://generativelanguage.googleapis.com/v1beta/openai".into(), api_key: key() }),
        ProviderId::OpenRouter => Box::new(openai_compat::OpenAiCompat { base_url: "https://openrouter.ai/api/v1".into(), api_key: key() }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{MemoryStore, SecretStore, Settings};

    #[test]
    fn keys_roundtrip_and_every_provider_has_models() {
        for id in ProviderId::ALL {
            assert_eq!(ProviderId::from_key(id.key()), Some(id));
            assert!(!crate::models::curated(id.key()).is_empty(), "{}", id.key());
            assert_eq!(id.is_cli(), id.key_url().is_none());
        }
    }

    #[tokio::test]
    async fn api_provider_reads_key_from_secret_store() {
        let s = Settings::default();
        let secrets = MemoryStore::default();
        assert_eq!(build_provider(ProviderId::OpenAi, &s, &secrets).status().await, ProviderStatus::NoKey);
        secrets.set("openai", "sk-test").unwrap();
        assert_eq!(build_provider(ProviderId::OpenAi, &s, &secrets).status().await, ProviderStatus::Ready);
    }

    #[test]
    fn default_models() {
        assert_eq!(crate::models::default_model("claude-cli").as_deref(), Some("sonnet"));
        assert_eq!(crate::models::default_model("codex-cli"), None);
    }
}
