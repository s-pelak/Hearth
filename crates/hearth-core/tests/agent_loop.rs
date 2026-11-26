//! Agent loop against a scripted provider: tool use, stall detection, opt-in escalation.

use async_trait::async_trait;
use hearth_core::provider::DeltaSink;
use hearth_core::*;
use std::sync::{Arc, Mutex};

/// Replays canned completions in order.
struct Scripted {
    id: String,
    turns: Mutex<Vec<Completion>>,
}

impl Scripted {
    fn new(id: &str, mut turns: Vec<Completion>) -> Arc<Self> {
        turns.reverse();
        Arc::new(Self { id: id.into(), turns: Mutex::new(turns) })
    }
}

#[async_trait]
impl Provider for Scripted {
    fn id(&self) -> &str {
        &self.id
    }
    async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        Ok(vec!["m".into()])
    }
    async fn complete(&self, _req: &ChatRequest, on_delta: DeltaSink<'_>) -> Result<Completion, ProviderError> {
        let c = self.turns.lock().unwrap().pop().expect("script ran out");
        on_delta(&c.content);
        Ok(c)
    }
}

fn text(s: &str) -> Completion {
    Completion { content: s.into(), tool_calls: vec![], native: None, usage: Usage::default(), stop_reason: "stop".into() }
}

fn call(name: &str, args: &str) -> Completion {
    Completion {
        content: String::new(),
        tool_calls: vec![ToolCall { id: "c1".into(), name: name.into(), arguments: args.into() }],
        native: None,
        usage: Usage::default(),
        stop_reason: "tool_calls".into(),
    }
}

#[derive(Default)]
struct Hooks {
    events: Mutex<Vec<AgentEvent>>,
}

#[async_trait]
impl AgentHooks for Hooks {
    fn emit(&self, e: AgentEvent) {
        self.events.lock().unwrap().push(e);
    }
}

#[tokio::test]
async fn runs_tools_and_finishes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "hello").unwrap();
    let p = Scripted::new("box", vec![call("read_file", r#"{"path":"a.txt"}"#), text("It says hello.")]);
    let mut cfg = AgentRun::new(p, ModelRef::new("box", "m"), ToolRegistry::with_project_files(dir.path()));
    let mut history = vec![Message::user("read a.txt")];
    let hooks = Hooks::default();
    let out = run(&mut cfg, &mut history, &hooks).await.unwrap();
    assert_eq!(out, Outcome::Finished);
    assert!(matches!(&history[2], Message::Tool { content, is_error: false, .. } if content == "hello"));
    assert_eq!(history.len(), 4);
}

#[tokio::test]
async fn stalls_when_escalation_is_off() {
    let dir = tempfile::tempdir().unwrap();
    let p = Scripted::new("box", vec![call("read_file", "{bad json"), call("read_file", "{bad json")]);
    let mut cfg = AgentRun::new(p, ModelRef::new("box", "m"), ToolRegistry::with_project_files(dir.path()));
    let mut history = vec![Message::user("go")];
    let hooks = Hooks::default();
    let out = run(&mut cfg, &mut history, &hooks).await.unwrap();
    assert_eq!(out, Outcome::Stalled { reason: StallReason::MalformedToolCalls { count: 2 } });
    assert!(hooks.events.lock().unwrap().iter().any(|e| matches!(e, AgentEvent::Stalled { .. })));
}

#[tokio::test]
async fn escalates_only_when_enabled() {
    let dir = tempfile::tempdir().unwrap();
    let local = Scripted::new("box", vec![call("nope", "{}"), call("nope", "{}"), call("nope", "{}")]);
    let cloud = Scripted::new("cloud", vec![text("Done on the bigger model.")]);
    let mut cfg = AgentRun::new(local, ModelRef::new("box", "m"), ToolRegistry::with_project_files(dir.path()));
    cfg.escalation.enabled = true;
    cfg.escalation.target = Some(ModelRef::new("cloud", "big"));
    cfg.escalation_provider = Some(cloud);
    let mut history = vec![Message::user("go")];
    let hooks = Hooks::default();
    let out = run(&mut cfg, &mut history, &hooks).await.unwrap();
    assert_eq!(out, Outcome::Finished);
    assert_eq!(cfg.model, ModelRef::new("cloud", "big"));
    assert!(hooks.events.lock().unwrap().iter().any(|e| matches!(e, AgentEvent::Escalated { .. })));
}
