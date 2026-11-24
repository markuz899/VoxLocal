//! `/v1/chat/completions`: copre OpenAI, Groq, OpenRouter, Together, Mistral, DeepSeek,
//! Ollama, LM Studio e llama.cpp server (i preset sono dati, vedi `registry`).

use async_trait::async_trait;
use serde_json::json;

use super::{http, non_empty, send_json, LlmError, LlmProvider, LlmRequest, LlmResponse, Result};

pub struct OpenAiCompatible {
    pub id: String,
    pub base_url: String,
    pub api_key: Option<String>,
    /// I modelli di ragionamento OpenAI rifiutano `temperature` e `max_tokens`: lì va a false.
    pub send_sampling: bool,
}

impl OpenAiCompatible {
    fn req(&self, rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.api_key {
            Some(k) => rb.bearer_auth(k),
            None => rb,
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAiCompatible {
    fn id(&self) -> &str {
        &self.id
    }

    async fn complete(&self, req: &LlmRequest) -> Result<LlmResponse> {
        let mut body = json!({
            "model": req.model,
            "messages": [
                { "role": "system", "content": req.system },
                { "role": "user", "content": req.user },
            ],
        });
        if self.id == "ollama" {
            // Per ripulire un dettato il ragionamento serve solo a rallentare; e con i modelli che Ollama vede come
            // "ragionanti" (Qwen3, il modello di pulizia addestrato) la risposta finirebbe in `reasoning`, non in
            // `content`. Sui modelli che non ragionano Ollama lo ignora.
            body["reasoning_effort"] = json!("none");
        }
        if self.send_sampling {
            body["max_tokens"] = json!(req.max_tokens);
            if let Some(t) = req.temperature {
                body["temperature"] = json!(t);
            }
        }
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let v = send_json(self.req(http().post(url)).timeout(req.timeout).json(&body)).await?;
        let text = v["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| LlmError::Parse("manca choices[0].message.content".into()))?;
        non_empty(text.to_string())
    }

    async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/models", self.base_url.trim_end_matches('/'));
        let v = send_json(self.req(http().get(url))).await?;
        Ok(v["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|m| m["id"].as_str().map(str::to_string))
            .collect::<Vec<_>>())
    }

    async fn health_check(&self) -> Result<()> {
        self.list_models().await.map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::time::Duration;
    use wiremock::matchers::{body_partial_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn provider(server: &MockServer, send_sampling: bool) -> OpenAiCompatible {
        OpenAiCompatible {
            id: "test".into(),
            base_url: format!("{}/v1", server.uri()),
            api_key: Some("sk-test".into()),
            send_sampling,
        }
    }

    fn req() -> LlmRequest {
        LlmRequest {
            system: "sys".into(),
            user: "ciao".into(),
            model: "m".into(),
            temperature: Some(0.2),
            max_tokens: 100,
            timeout: Duration::from_secs(2),
        }
    }

    #[tokio::test]
    async fn completes_and_sends_auth() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("authorization", "Bearer sk-test"))
            .and(body_partial_json(
                json!({"model": "m", "temperature": 0.2, "max_tokens": 100}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                json!({"choices": [{"message": {"role": "assistant", "content": " Ciao. "}}]}),
            ))
            .expect(1)
            .mount(&server)
            .await;
        let out = provider(&server, true).complete(&req()).await.unwrap();
        assert_eq!(out.text, "Ciao.");
    }

    #[tokio::test]
    async fn omits_sampling_params_when_disabled() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(|r: &wiremock::Request| {
                let b: Value = serde_json::from_slice(&r.body).unwrap();
                let ok = b.get("temperature").is_none() && b.get("max_tokens").is_none();
                ResponseTemplate::new(if ok { 200 } else { 400 })
                    .set_body_json(json!({"choices": [{"message": {"content": "ok"}}]}))
            })
            .mount(&server)
            .await;
        assert!(provider(&server, false).complete(&req()).await.is_ok());
    }

    #[tokio::test]
    async fn ollama_turns_reasoning_off_others_do_not() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(|r: &wiremock::Request| {
                let b: Value = serde_json::from_slice(&r.body).unwrap();
                let effort = b["reasoning_effort"].as_str().unwrap_or("-").to_string();
                ResponseTemplate::new(200)
                    .set_body_json(json!({"choices": [{"message": {"content": effort}}]}))
            })
            .mount(&server)
            .await;
        let mut p = provider(&server, true);
        assert_eq!(p.complete(&req()).await.unwrap().text, "-");
        p.id = "ollama".into();
        assert_eq!(p.complete(&req()).await.unwrap().text, "none");
    }

    #[tokio::test]
    async fn maps_401_to_auth_and_slow_to_timeout() {
        let server = MockServer::start().await;
        Mock::given(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;
        let err = provider(&server, true).complete(&req()).await.unwrap_err();
        assert!(matches!(err, LlmError::Auth(401)), "{err:?}");

        let slow = MockServer::start().await;
        Mock::given(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5)))
            .mount(&slow)
            .await;
        let mut r = req();
        r.timeout = Duration::from_millis(200);
        let err = provider(&slow, true).complete(&r).await.unwrap_err();
        assert!(matches!(err, LlmError::Timeout), "{err:?}");
    }

    #[tokio::test]
    async fn lists_models() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"data": [{"id": "a"}, {"id": "b"}]})),
            )
            .mount(&server)
            .await;
        let p = provider(&server, true);
        assert_eq!(p.list_models().await.unwrap(), vec!["a", "b"]);
        assert!(p.health_check().await.is_ok());
    }
}
