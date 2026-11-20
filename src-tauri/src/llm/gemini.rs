//! Google Gemini API nativa (`generateContent`).

use async_trait::async_trait;
use serde_json::json;

use super::{http, non_empty, send_json, LlmProvider, LlmRequest, LlmResponse, Result};

pub struct Gemini {
    pub id: String,
    pub base_url: String,
    pub api_key: String,
}

impl Gemini {
    fn url(&self, p: &str) -> String {
        format!("{}/v1beta/{p}", self.base_url.trim_end_matches('/'))
    }
}

#[async_trait]
impl LlmProvider for Gemini {
    fn id(&self) -> &str {
        &self.id
    }

    async fn complete(&self, req: &LlmRequest) -> Result<LlmResponse> {
        let mut config = json!({ "maxOutputTokens": req.max_tokens });
        if let Some(t) = req.temperature {
            config["temperature"] = json!(t);
        }
        let body = json!({
            "systemInstruction": { "parts": [{ "text": req.system }] },
            "contents": [{ "role": "user", "parts": [{ "text": req.user }] }],
            "generationConfig": config,
        });
        let url = self.url(&format!("models/{}:generateContent", req.model));
        let rb = http()
            .post(url)
            .header("x-goog-api-key", &self.api_key)
            .timeout(req.timeout)
            .json(&body);
        let v = send_json(rb).await?;
        let text = v["candidates"][0]["content"]["parts"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|p| p["text"].as_str())
            .collect::<String>();
        non_empty(text)
    }

    async fn list_models(&self) -> Result<Vec<String>> {
        let rb = http()
            .get(self.url("models"))
            .header("x-goog-api-key", &self.api_key);
        let v = send_json(rb).await?;
        Ok(v["models"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|m| m["name"].as_str())
            .map(|n| n.trim_start_matches("models/").to_string())
            .collect())
    }

    async fn health_check(&self) -> Result<()> {
        self.list_models().await.map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use wiremock::matchers::{body_partial_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn completes_and_lists_models() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1beta/models/gemini-flash-latest:generateContent"))
            .and(header("x-goog-api-key", "g"))
            .and(body_partial_json(json!({
                "systemInstruction": {"parts": [{"text": "sys"}]},
                "contents": [{"role": "user", "parts": [{"text": "ciao"}]}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "candidates": [{"content": {"parts": [{"text": "Cia"}, {"text": "o."}]}}]
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1beta/models"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"models": [{"name": "models/gemini-flash-latest"}]})),
            )
            .mount(&server)
            .await;

        let p = Gemini {
            id: "gemini".into(),
            base_url: server.uri(),
            api_key: "g".into(),
        };
        let req = LlmRequest {
            system: "sys".into(),
            user: "ciao".into(),
            model: "gemini-flash-latest".into(),
            temperature: None,
            max_tokens: 50,
            timeout: Duration::from_secs(2),
        };
        assert_eq!(p.complete(&req).await.unwrap().text, "Ciao.");
        assert_eq!(p.list_models().await.unwrap(), vec!["gemini-flash-latest"]);
    }
}
