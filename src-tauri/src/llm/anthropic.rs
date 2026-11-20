//! Anthropic Messages API nativa.

use async_trait::async_trait;
use serde_json::json;

use super::{http, non_empty, send_json, LlmProvider, LlmRequest, LlmResponse, Result};

const VERSION: &str = "2023-06-01";

pub struct Anthropic {
    pub id: String,
    pub base_url: String,
    pub api_key: String,
}

impl Anthropic {
    fn req(&self, rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        rb.header("x-api-key", &self.api_key)
            .header("anthropic-version", VERSION)
    }

    fn url(&self, p: &str) -> String {
        format!("{}/v1/{p}", self.base_url.trim_end_matches('/'))
    }
}

#[async_trait]
impl LlmProvider for Anthropic {
    fn id(&self) -> &str {
        &self.id
    }

    async fn complete(&self, req: &LlmRequest) -> Result<LlmResponse> {
        let mut body = json!({
            "model": req.model,
            "max_tokens": req.max_tokens,
            "system": req.system,
            "messages": [{ "role": "user", "content": req.user }],
        });
        if let Some(t) = req.temperature {
            body["temperature"] = json!(t);
        }
        let v = send_json(
            self.req(http().post(self.url("messages")))
                .timeout(req.timeout)
                .json(&body),
        )
        .await?;
        let text = v["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|b| b["type"] == "text")
            .filter_map(|b| b["text"].as_str())
            .collect::<String>();
        non_empty(text)
    }

    async fn list_models(&self) -> Result<Vec<String>> {
        let v = send_json(self.req(http().get(self.url("models")))).await?;
        Ok(v["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|m| m["id"].as_str().map(str::to_string))
            .collect())
    }

    async fn health_check(&self) -> Result<()> {
        self.list_models().await.map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::LlmError;
    use std::time::Duration;
    use wiremock::matchers::{body_partial_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn provider(server: &MockServer) -> Anthropic {
        Anthropic {
            id: "anthropic".into(),
            base_url: server.uri(),
            api_key: "k".into(),
        }
    }

    fn req() -> LlmRequest {
        LlmRequest {
            system: "sys".into(),
            user: "ciao".into(),
            model: "claude-haiku-5-5".into(),
            temperature: Some(0.2),
            max_tokens: 50,
            timeout: Duration::from_secs(2),
        }
    }

    #[tokio::test]
    async fn completes_with_native_format() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(header("x-api-key", "k"))
            .and(header("anthropic-version", VERSION))
            .and(body_partial_json(json!({"system": "sys", "max_tokens": 50,
                "messages": [{"role": "user", "content": "ciao"}]})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "content": [{"type": "thinking", "thinking": "…"}, {"type": "text", "text": "Ciao."}]
            })))
            .expect(1)
            .mount(&server)
            .await;
        assert_eq!(
            provider(&server).complete(&req()).await.unwrap().text,
            "Ciao."
        );
    }

    #[tokio::test]
    async fn surfaces_http_errors() {
        let server = MockServer::start().await;
        Mock::given(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(529).set_body_string("overloaded"))
            .mount(&server)
            .await;
        let err = provider(&server).complete(&req()).await.unwrap_err();
        assert!(matches!(err, LlmError::Http { status: 529, .. }), "{err:?}");
    }
}
