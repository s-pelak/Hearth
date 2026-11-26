//! Provider-neutral conversation types shared by every adapter.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One message in a thread's transcript.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum Message {
    System {
        content: String,
    },
    User {
        content: String,
    },
    Assistant {
        #[serde(default)]
        content: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tool_calls: Vec<ToolCall>,
        /// Which model wrote this turn, e.g. `hearth-box/qwen3.5-9b`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model: Option<String>,
        /// Provider-native content (e.g. Anthropic content blocks with thinking),
        /// replayed verbatim when the same provider sees this turn again.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        native: Option<NativeContent>,
    },
    Tool {
        tool_call_id: String,
        name: String,
        content: String,
        #[serde(default)]
        is_error: bool,
    },
}

impl Message {
    pub fn user(s: impl Into<String>) -> Self {
        Message::User { content: s.into() }
    }
    pub fn system(s: impl Into<String>) -> Self {
        Message::System { content: s.into() }
    }
    pub fn assistant(s: impl Into<String>) -> Self {
        Message::Assistant { content: s.into(), tool_calls: vec![], model: None, native: None }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NativeContent {
    /// Provider kind that produced `blocks` (`anthropic`).
    pub provider_kind: String,
    pub blocks: Value,
}

/// A tool call as the model emitted it. `arguments` is kept as the raw string
/// so malformed JSON can be detected (and counted toward escalation).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

impl ToolCall {
    pub fn parsed_args(&self) -> Result<Value, serde_json::Error> {
        if self.arguments.trim().is_empty() {
            return Ok(Value::Object(Default::default()));
        }
        serde_json::from_str(&self.arguments)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    /// JSON Schema for the arguments object.
    pub parameters: Value,
}

/// A model on a configured provider, e.g. `{provider: "hearth-box", model: "qwen3.5-9b"}`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ModelRef {
    pub provider: String,
    pub model: String,
}

impl ModelRef {
    pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self { provider: provider.into(), model: model.into() }
    }
    pub fn label(&self) -> String {
        format!("{}/{}", self.provider, self.model)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Clone, Debug)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolSpec>,
    pub max_tokens: u32,
}

/// The finished assistant turn from one provider call.
#[derive(Clone, Debug)]
pub struct Completion {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub native: Option<NativeContent>,
    pub usage: Usage,
    pub stop_reason: String,
}
