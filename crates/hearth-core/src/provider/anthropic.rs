//! Anthropic Messages API over raw HTTP (there is no official Rust SDK).
//! Non-streaming for now; the text arrives in one delta when the turn finishes.

use super::{DeltaSink, Provider, ProviderError, check_status, http_client};
use crate::types::{ChatRequest, Completion, Message, NativeContent, ToolCall, Usage};
use async_trait::async_trait;
use serde_json::{Value, json};

const API_VERSION: &str = "2023-06-01";
pub const KIND: &str = "anthropic";

pub struct AnthropicProvider {
    id: String,
    base_url: String,
    api_key: Option<String>,
    http: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new(id: &str, base_url: &str, api_key: Option<String>) -> Self {
        Self { id: id.into(), base_url: base_url.trim_end_matches('/').into(), api_key, http: http_client() }
    }

    fn req(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        self.http
            .request(method, format!("{}{}", self.base_url, path))
            .header("x-api-key", self.api_key.clone().unwrap_or_default())
            .header("anthropic-version", API_VERSION)
    }

    fn unreachable(&self, e: reqwest::Error) -> ProviderError {
        ProviderError::Unreachable(self.base_url.clone(), e.to_string())
    }
}

/// Split out the system prompt and convert the transcript to Messages API shape.
/// Consecutive tool results become one user turn, as the API expects.
pub(crate) fn to_wire(messages: &[Message]) -> (String, Vec<Value>) {
    let mut system = Vec::new();
    let mut out: Vec<Value> = Vec::new();
    let mut pending_results: Vec<Value> = Vec::new();

    let flush = |out: &mut Vec<Value>, pending: &mut Vec<Value>| {
        if !pending.is_empty() {
            out.push(json!({"role": "user", "content": std::mem::take(pending)}));
        }
    };

    for m in messages {
        match m {
            Message::System { content } => system.push(content.clone()),
            Message::Tool { tool_call_id, content, is_error, .. } => {
                pending_results.push(json!({"type": "tool_result", "tool_use_id": tool_call_id, "content": content, "is_error": is_error}));
            }
            Message::User { content } => {
                flush(&mut out, &mut pending_results);
                out.push(json!({"role": "user", "content": content}));
            }
            Message::Assistant { content, tool_calls, native, .. } => {
                flush(&mut out, &mut pending_results);
                let blocks = match native {
                    Some(n) if n.provider_kind == KIND => n.blocks.clone(),
                    _ => {
                        let mut b = Vec::new();
                        if !content.is_empty() {
                            b.push(json!({"type": "text", "text": content}));
                        }
                        for c in tool_calls {
                            let input = c.parsed_args().ok().filter(Value::is_object).unwrap_or_else(|| json!({}));
                            b.push(json!({"type": "tool_use", "id": c.id, "name": c.name, "input": input}));
                        }
                        Value::Array(b)
                    }
                };
                out.push(json!({"role": "assistant", "content": blocks}));
            }
        }
    }
    flush(&mut out, &mut pending_results);
    (system.join("\n\n"), out)
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn id(&self) -> &str {
        &self.id
    }

    async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        let resp = self.req(reqwest::Method::GET, "/v1/models?limit=100").send().await.map_err(|e| self.unreachable(e))?;
        let v: Value = check_status(&self.id, resp).await?.json().await.map_err(|e| ProviderError::BadResponse(self.id.clone(), e.to_string()))?;
        Ok(v["data"].as_array().map(|a| a.iter().filter_map(|m| m["id"].as_str().map(String::from)).collect()).unwrap_or_default())
    }

    async fn complete(&self, req: &ChatRequest, on_delta: DeltaSink<'_>) -> Result<Completion, ProviderError> {
        // `req.settings` is for local models only; Anthropic rejects `temperature`, so it is never sent.
        let (system, messages) = to_wire(&req.messages);
        let mut body = json!({"model": req.model, "max_tokens": req.max_tokens, "messages": messages});
        if !system.is_empty() {
            body["system"] = json!(system);
        }
        if !req.tools.is_empty() {
            body["tools"] = req.tools.iter().map(|t| json!({"name": t.name, "description": t.description, "input_schema": t.parameters})).collect();
        }

        let resp = self.req(reqwest::Method::POST, "/v1/messages").json(&body).send().await.map_err(|e| self.unreachable(e))?;
        let v: Value = check_status(&self.id, resp).await?.json().await.map_err(|e| ProviderError::BadResponse(self.id.clone(), e.to_string()))?;

        let blocks = v["content"].clone();
        let mut content = String::new();
        let mut tool_calls = Vec::new();
        for b in blocks.as_array().into_iter().flatten() {
            match b["type"].as_str() {
                Some("text") => content.push_str(b["text"].as_str().unwrap_or_default()),
                Some("tool_use") => tool_calls.push(ToolCall {
                    id: b["id"].as_str().unwrap_or_default().into(),
                    name: b["name"].as_str().unwrap_or_default().into(),
                    arguments: b["input"].to_string(),
                }),
                _ => {}
            }
        }
        if !content.is_empty() {
            on_delta(&content);
        }
        let stop_reason = v["stop_reason"].as_str().unwrap_or("end_turn").to_string();
        Ok(Completion {
            content,
            tool_calls,
            native: Some(NativeContent { provider_kind: KIND.into(), blocks }),
            usage: Usage {
                input_tokens: v["usage"]["input_tokens"].as_u64().unwrap_or(0),
                output_tokens: v["usage"]["output_tokens"].as_u64().unwrap_or(0),
            },
            stop_reason,
        })
    }
}
