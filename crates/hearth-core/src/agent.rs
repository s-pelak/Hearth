//! The agent loop for one thread: call the model, run its tool calls, repeat.

use crate::escalation::{EscalationPolicy, StallDetector, StallReason};
use crate::provider::{Provider, ProviderError};
use crate::tools::{Permission, ToolRegistry};
use crate::types::{ChatRequest, Message, ModelRef, ToolCall, Usage};
use async_trait::async_trait;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// Streaming text from the current model turn.
    Delta { text: String },
    /// A message was added to the transcript (assistant turn or tool result).
    Message { message: Message },
    Usage { model: ModelRef, usage: Usage },
    Escalated { from: ModelRef, to: ModelRef, reason: StallReason },
    Stalled { reason: StallReason },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum Outcome {
    Finished,
    Stalled { reason: StallReason },
    StepLimit,
    Cancelled,
}

#[async_trait]
pub trait AgentHooks: Send + Sync {
    fn emit(&self, event: AgentEvent);
    /// Asked before running a tool whose permission is `Ask`.
    async fn approve(&self, _call: &ToolCall) -> bool {
        false
    }
}

/// Everything one run of the loop needs.
pub struct AgentRun {
    pub provider: Arc<dyn Provider>,
    pub model: ModelRef,
    pub tools: ToolRegistry,
    pub permissions: HashMap<String, Permission>,
    pub max_steps: u32,
    pub max_tokens: u32,
    pub escalation: EscalationPolicy,
    /// Provider for `escalation.target`, if escalation is on and the provider is configured.
    pub escalation_provider: Option<Arc<dyn Provider>>,
    pub cancel: Arc<AtomicBool>,
}

impl AgentRun {
    pub fn new(provider: Arc<dyn Provider>, model: ModelRef, tools: ToolRegistry) -> Self {
        Self {
            provider,
            model,
            tools,
            permissions: HashMap::new(),
            max_steps: 25,
            max_tokens: 8192,
            escalation: EscalationPolicy::default(),
            escalation_provider: None,
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }
}

fn canonical(v: &serde_json::Value) -> String {
    // serde_json's default Map is a BTreeMap, so this sorts keys.
    serde_json::to_string(v).unwrap_or_default()
}

pub async fn run(cfg: &mut AgentRun, history: &mut Vec<Message>, hooks: &dyn AgentHooks) -> Result<Outcome, ProviderError> {
    let mut detector = StallDetector::new(cfg.escalation.triggers.clone());

    for _ in 0..cfg.max_steps {
        if cfg.cancel.load(Ordering::Relaxed) {
            return Ok(Outcome::Cancelled);
        }
        let req = ChatRequest { model: cfg.model.model.clone(), messages: history.clone(), tools: cfg.tools.specs(), max_tokens: cfg.max_tokens };
        let sink = |t: &str| hooks.emit(AgentEvent::Delta { text: t.to_string() });
        let done = cfg.provider.complete(&req, &sink).await?;
        hooks.emit(AgentEvent::Usage { model: cfg.model.clone(), usage: done.usage.clone() });

        let assistant = Message::Assistant {
            content: done.content.clone(),
            tool_calls: done.tool_calls.clone(),
            model: Some(cfg.model.label()),
            native: done.native.clone(),
        };
        history.push(assistant.clone());
        hooks.emit(AgentEvent::Message { message: assistant });

        if done.tool_calls.is_empty() {
            return Ok(Outcome::Finished);
        }

        for call in &done.tool_calls {
            if cfg.cancel.load(Ordering::Relaxed) {
                return Ok(Outcome::Cancelled);
            }
            let (content, is_error) = match call.parsed_args() {
                Err(e) => {
                    detector.on_malformed();
                    (format!("Invalid JSON in the arguments for {}: {e}. Send the call again with a valid JSON object.", call.name), true)
                }
                Ok(args) => {
                    let result = match cfg.tools.get(&call.name) {
                        None => Err(format!("Unknown tool: {}. Available tools: {}", call.name, cfg.tools.specs().iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(", "))),
                        Some(tool) => {
                            let perm = cfg.permissions.get(&call.name).copied().unwrap_or_else(|| tool.default_permission());
                            match perm {
                                Permission::Deny => Err(format!("The user has not allowed the {} tool in this project.", call.name)),
                                Permission::Ask if !hooks.approve(call).await => Err("The user declined this tool call.".to_string()),
                                _ => tool.call(args.clone()).await,
                            }
                        }
                    };
                    let is_error = result.is_err();
                    detector.on_call(&call.name, &canonical(&args), is_error);
                    (result.unwrap_or_else(|e| e), is_error)
                }
            };
            let msg = Message::Tool { tool_call_id: call.id.clone(), name: call.name.clone(), content, is_error };
            history.push(msg.clone());
            hooks.emit(AgentEvent::Message { message: msg });
        }

        if let Some(reason) = detector.check() {
            let target = cfg.escalation.target.clone();
            match (cfg.escalation.enabled, target, cfg.escalation_provider.take()) {
                (true, Some(to), Some(p)) if to != cfg.model => {
                    hooks.emit(AgentEvent::Escalated { from: cfg.model.clone(), to: to.clone(), reason });
                    cfg.provider = p;
                    cfg.model = to;
                    detector.reset();
                }
                _ => {
                    hooks.emit(AgentEvent::Stalled { reason: reason.clone() });
                    return Ok(Outcome::Stalled { reason });
                }
            }
        }
    }
    Ok(Outcome::StepLimit)
}
