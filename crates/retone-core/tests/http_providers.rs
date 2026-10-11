use retone_core::providers::{anthropic_api::AnthropicApi, openai_compat::OpenAiCompat, CompletionRequest, Provider, ProviderError};
use wiremock::{matchers::{header, method, path, body_partial_json}, Mock, MockServer, ResponseTemplate};
use serde_json::json;

fn req() -> CompletionRequest { CompletionRequest { system: "SYS".into(), user: "<draft>\nhi\n</draft>".into(), model: Some("m1".into()) } }

#[tokio::test]
async fn anthropic_happy_path() {
    let s = MockServer::start().await;
    Mock::given(method("POST")).and(path("/v1/messages")).and(header("x-api-key", "k"))
        .and(header("anthropic-version", "2023-06-01"))
        .and(body_partial_json(json!({"model":"m1","system":"SYS","messages":[{"role":"user","content":"<draft>\nhi\n</draft>"}]})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"content":[{"type":"text","text":" Hi team "}]})))
        .mount(&s).await;
    let p = AnthropicApi { base_url: s.uri(), api_key: Some("k".into()) };
    assert_eq!(p.complete(&req()).await.unwrap(), "Hi team");
}

#[tokio::test]
async fn anthropic_401_is_auth_error() {
    let s = MockServer::start().await;
    Mock::given(method("POST")).respond_with(ResponseTemplate::new(401).set_body_json(json!({"error":{"message":"invalid x-api-key"}})))
        .mount(&s).await;
    let p = AnthropicApi { base_url: s.uri(), api_key: Some("bad".into()) };
    assert!(matches!(p.complete(&req()).await, Err(ProviderError::Auth(m)) if m.contains("invalid")));
}

#[tokio::test]
async fn openai_compat_happy_path() {
    let s = MockServer::start().await;
    Mock::given(method("POST")).and(path("/chat/completions")).and(header("authorization", "Bearer k"))
        .and(body_partial_json(json!({"model":"m1","messages":[{"role":"system","content":"SYS"},{"role":"user","content":"<draft>\nhi\n</draft>"}]})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"choices":[{"message":{"content":"Hi team\n"}}]})))
        .mount(&s).await;
    let p = OpenAiCompat { base_url: s.uri(), api_key: Some("k".into()) };
    assert_eq!(p.complete(&req()).await.unwrap(), "Hi team");
}

#[tokio::test]
async fn missing_key_and_model() {
    let p = OpenAiCompat { base_url: "http://unused".into(), api_key: None };
    assert_eq!(p.complete(&req()).await, Err(ProviderError::NoKey));
    let p = OpenAiCompat { base_url: "http://unused".into(), api_key: Some("k".into()) };
    let mut r = req(); r.model = None;
    assert_eq!(p.complete(&r).await, Err(ProviderError::Failed("no model selected".into())));
}

#[tokio::test]
async fn empty_choice_is_empty_output() {
    let s = MockServer::start().await;
    Mock::given(method("POST")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"choices":[{"message":{"content":"  "}}]})))
        .mount(&s).await;
    let p = OpenAiCompat { base_url: s.uri(), api_key: Some("k".into()) };
    assert_eq!(p.complete(&req()).await, Err(ProviderError::EmptyOutput));
}

#[tokio::test]
async fn anthropic_truncated_is_error() {
    let s = MockServer::start().await;
    Mock::given(method("POST")).respond_with(ResponseTemplate::new(200).set_body_json(
        json!({"content":[{"type":"text","text":"Hi te"}],"stop_reason":"max_tokens"}))).mount(&s).await;
    let p = AnthropicApi { base_url: s.uri(), api_key: Some("k".into()) };
    assert_eq!(p.complete(&req()).await, Err(ProviderError::Failed("response was cut off (too long)".into())));
}

#[tokio::test]
async fn openai_truncated_is_error() {
    let s = MockServer::start().await;
    Mock::given(method("POST")).respond_with(ResponseTemplate::new(200).set_body_json(
        json!({"choices":[{"message":{"content":"Hi te"},"finish_reason":"length"}]}))).mount(&s).await;
    let p = OpenAiCompat { base_url: s.uri(), api_key: Some("k".into()) };
    assert_eq!(p.complete(&req()).await, Err(ProviderError::Failed("response was cut off (too long)".into())));
}
