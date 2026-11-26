//! OpenAI-compatible chat completions with SSE streaming.
//! Works with llama.cpp server, LM Studio, Ollama, OpenAI and Gemini's OpenAI endpoint.

use super::{DeltaSink, Provider, ProviderError, check_status, http_client};
use crate::types::{ChatRequest, Completion, Message, ToolCall, Usage};
use async_trait::async_trait;
use futures_util::StreamExt;
use serde_json::{Value, json};

pub struct OpenAiProvider {
    id: String,
    base_url: String,
    api_key: Option<String>,
    http: reqwest::Client,
}

impl OpenAiProvider {
    pub fn new(id: &str, base_url: &str, api_key: Option<String>) -> Self {
        Self { id: id.into(), base_url: base_url.trim_end_matches('/').into(), api_key, http: http_client() }
    }

    fn req(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let r = self.http.request(method, format!("{}{}", self.base_url, path));
        match &self.api_key {
            Some(k) if !k.is_empty() => r.bearer_auth(k),
            _ => r,
        }
    }

    fn unreachable(&self, e: reqwest::Error) -> ProviderError {
        ProviderError::Unreachable(self.base_url.clone(), e.to_string())
    }
}

pub(crate) fn to_wire_messages(messages: &[Message]) -> Vec<Value> {
    messages
        .iter()
        .map(|m| match m {
            Message::System { content } => json!({"role": "system", "content": content}),
            Message::User { content } => json!({"role": "user", "content": content}),
            Message::Assistant { content, tool_calls, .. } => {
                let mut v = json!({"role": "assistant", "content": content});
                if !tool_calls.is_empty() {
                    v["tool_calls"] = tool_calls
                        .iter()
                        .map(|c| json!({"id": c.id, "type": "function", "function": {"name": c.name, "arguments": c.arguments}}))
                        .collect();
                }
                v
            }
            Message::Tool { tool_call_id, content, .. } => {
                json!({"role": "tool", "tool_call_id": tool_call_id, "content": content})
            }
        })
        .collect()
}

#[derive(Default)]
struct PartialCall {
    id: String,
    name: String,
    arguments: String,
}

#[async_trait]
impl Provider for OpenAiProvider {
    fn id(&self) -> &str {
        &self.id
    }

    async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        let resp = self.req(reqwest::Method::GET, "/models").send().await.map_err(|e| self.unreachable(e))?;
        let v: Value = check_status(&self.id, resp).await?.json().await.map_err(|e| ProviderError::BadResponse(self.id.clone(), e.to_string()))?;
        let mut ids: Vec<String> = v["data"]
            .as_array()
            .map(|a| a.iter().filter_map(|m| m["id"].as_str().map(|s| s.trim_start_matches("models/").to_string())).collect())
            .unwrap_or_default();
        ids.sort();
        Ok(ids)
    }

    async fn complete(&self, req: &ChatRequest, on_delta: DeltaSink<'_>) -> Result<Completion, ProviderError> {
        let mut body = json!({
            "model": req.model,
            "messages": to_wire_messages(&req.messages),
            "stream": true,
            "stream_options": {"include_usage": true},
            "max_tokens": req.max_tokens,
        });
        if !req.tools.is_empty() {
            body["tools"] = req
                .tools
                .iter()
                .map(|t| json!({"type": "function", "function": {"name": t.name, "description": t.description, "parameters": t.parameters}}))
                .collect();
        }

        let resp = self.req(reqwest::Method::POST, "/chat/completions").json(&body).send().await.map_err(|e| self.unreachable(e))?;
        let resp = check_status(&self.id, resp).await?;

        let mut content = String::new();
        let mut calls: Vec<PartialCall> = Vec::new();
        let mut usage = Usage::default();
        let mut stop_reason = String::from("stop");
        let mut buf = String::new();
        let mut stream = resp.bytes_stream();

        'outer: while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| self.unreachable(e))?;
            buf.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(nl) = buf.find('\n') {
                let line: String = buf.drain(..=nl).collect();
                let line = line.trim();
                let Some(data) = line.strip_prefix("data:") else { continue };
                let data = data.trim();
                if data == "[DONE]" {
                    break 'outer;
                }
                let Ok(ev) = serde_json::from_str::<Value>(data) else { continue };
                if let Some(u) = ev.get("usage").filter(|u| !u.is_null()) {
                    usage.input_tokens = u["prompt_tokens"].as_u64().unwrap_or(0);
                    usage.output_tokens = u["completion_tokens"].as_u64().unwrap_or(0);
                }
                let Some(choice) = ev["choices"].get(0) else { continue };
                if let Some(r) = choice["finish_reason"].as_str() {
                    stop_reason = r.to_string();
                }
                let delta = &choice["delta"];
                if let Some(t) = delta["content"].as_str() {
                    if !t.is_empty() {
                        content.push_str(t);
                        on_delta(t);
                    }
                }
                if let Some(tcs) = delta["tool_calls"].as_array() {
                    for tc in tcs {
                        let idx = tc["index"].as_u64().unwrap_or(calls.len().saturating_sub(1) as u64) as usize;
                        while calls.len() <= idx {
                            calls.push(PartialCall::default());
                        }
                        let p = &mut calls[idx];
                        if let Some(id) = tc["id"].as_str() {
                            p.id = id.to_string();
                        }
                        if let Some(n) = tc["function"]["name"].as_str() {
                            p.name.push_str(n);
                        }
                        if let Some(a) = tc["function"]["arguments"].as_str() {
                            p.arguments.push_str(a);
                        }
                    }
                }
            }
        }

        let tool_calls = calls
            .into_iter()
            .enumerate()
            .filter(|(_, p)| !p.name.is_empty())
            .map(|(i, p)| ToolCall {
                id: if p.id.is_empty() { format!("call_{i}") } else { p.id },
                name: p.name,
                arguments: p.arguments,
            })
            .collect();

        Ok(Completion { content, tool_calls, native: None, usage, stop_reason })
    }
}
