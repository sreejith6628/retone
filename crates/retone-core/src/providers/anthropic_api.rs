//! Anthropic Messages API (API key).
use super::*;

pub struct AnthropicApi { pub base_url: String, pub api_key: Option<String> }

#[async_trait]
impl Provider for AnthropicApi {
    async fn complete(&self, req: &CompletionRequest) -> Result<String, ProviderError> {
        let key = self.api_key.clone().filter(|k| !k.is_empty()).ok_or(ProviderError::NoKey)?;
        let model = req.model.clone().filter(|m| !m.is_empty()).ok_or(ProviderError::Failed("no model selected".into()))?;
        let body = serde_json::json!({"model": model, "max_tokens": 4096, "system": req.system,
                                      "messages": [{"role": "user", "content": req.user}]});
        let v = post_json(&format!("{}/v1/messages", self.base_url.trim_end_matches('/')),
                          &[("x-api-key", key), ("anthropic-version", "2023-06-01".into())], body).await?;
        let text: String = v["content"].as_array().map(|blocks| blocks.iter()
            .filter_map(|b| b["text"].as_str()).collect::<Vec<_>>().join("")).unwrap_or_default();
        non_empty(Some(&text))
    }
    async fn status(&self) -> ProviderStatus {
        if self.api_key.as_deref().unwrap_or("").is_empty() { ProviderStatus::NoKey } else { ProviderStatus::Ready }
    }
}
