//! OpenAI-compatible Chat Completions (OpenAI, Google Gemini's OpenAI endpoint, OpenRouter).
use super::*;

pub struct OpenAiCompat { pub base_url: String, pub api_key: Option<String> }

#[async_trait]
impl Provider for OpenAiCompat {
    async fn complete(&self, req: &CompletionRequest) -> Result<String, ProviderError> {
        let key = self.api_key.clone().filter(|k| !k.is_empty()).ok_or(ProviderError::NoKey)?;
        let model = req.model.clone().filter(|m| !m.is_empty()).ok_or(ProviderError::Failed("no model selected".into()))?;
        let body = serde_json::json!({"model": model, "messages": [
            {"role": "system", "content": req.system}, {"role": "user", "content": req.user}]});
        let v = post_json(&format!("{}/chat/completions", self.base_url.trim_end_matches('/')),
                          &[("authorization", format!("Bearer {key}"))], body).await?;
        non_empty(v["choices"][0]["message"]["content"].as_str())
    }
    async fn status(&self) -> ProviderStatus {
        if self.api_key.as_deref().unwrap_or("").is_empty() { ProviderStatus::NoKey } else { ProviderStatus::Ready }
    }
}
