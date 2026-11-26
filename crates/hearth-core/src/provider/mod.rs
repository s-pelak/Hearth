//! Model providers. Every adapter turns a [`ChatRequest`] into one assistant turn.

mod anthropic;
mod openai;

pub use anthropic::AnthropicProvider;
pub use openai::OpenAiProvider;

use crate::types::{ChatRequest, Completion};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Called with each text fragment as it streams in.
pub type DeltaSink<'a> = &'a (dyn Fn(&str) + Send + Sync);

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("could not reach {0}: {1}")]
    Unreachable(String, String),
    #[error("{provider} returned HTTP {status}: {body}")]
    Http { provider: String, status: u16, body: String },
    #[error("unexpected response from {0}: {1}")]
    BadResponse(String, String),
}

#[async_trait]
pub trait Provider: Send + Sync {
    fn id(&self) -> &str;
    async fn list_models(&self) -> Result<Vec<String>, ProviderError>;
    async fn complete(&self, req: &ChatRequest, on_delta: DeltaSink<'_>) -> Result<Completion, ProviderError>;
}

/// The API family a provider speaks.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    /// llama.cpp, LM Studio, Ollama, OpenAI, Gemini (OpenAI-compatible endpoint), OpenRouter...
    OpenAiCompatible,
    Anthropic,
}

/// What the user configures in Settings. API keys live in the OS keychain, not here.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub kind: ProviderKind,
    pub base_url: String,
    /// Local providers cost nothing; cloud providers count toward spend caps.
    #[serde(default)]
    pub is_local: bool,
}

impl ProviderConfig {
    pub fn build(&self, api_key: Option<String>) -> Arc<dyn Provider> {
        match self.kind {
            ProviderKind::OpenAiCompatible => Arc::new(OpenAiProvider::new(&self.id, &self.base_url, api_key)),
            ProviderKind::Anthropic => Arc::new(AnthropicProvider::new(&self.id, &self.base_url, api_key)),
        }
    }

    /// Sensible starting set: the LLM box plus the main clouds.
    pub fn defaults() -> Vec<ProviderConfig> {
        vec![
            ProviderConfig {
                id: "hearth-box".into(),
                name: "LLM box".into(),
                kind: ProviderKind::OpenAiCompatible,
                base_url: "http://hearth-box:8080/v1".into(),
                is_local: true,
            },
            ProviderConfig {
                id: "anthropic".into(),
                name: "Anthropic".into(),
                kind: ProviderKind::Anthropic,
                base_url: "https://api.anthropic.com".into(),
                is_local: false,
            },
            ProviderConfig {
                id: "openai".into(),
                name: "OpenAI".into(),
                kind: ProviderKind::OpenAiCompatible,
                base_url: "https://api.openai.com/v1".into(),
                is_local: false,
            },
            ProviderConfig {
                id: "gemini".into(),
                name: "Gemini".into(),
                kind: ProviderKind::OpenAiCompatible,
                base_url: "https://generativelanguage.googleapis.com/v1beta/openai".into(),
                is_local: false,
            },
        ]
    }
}

pub(crate) fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("http client")
}

pub(crate) async fn check_status(provider: &str, resp: reqwest::Response) -> Result<reqwest::Response, ProviderError> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    let body = resp.text().await.unwrap_or_default();
    Err(ProviderError::Http { provider: provider.to_string(), status: status.as_u16(), body: body.chars().take(2000).collect() })
}
