//! Spotting a stalled thread, and (only if the user turned it on) handing it to a stronger model.

use crate::types::ModelRef;
use serde::{Deserialize, Serialize};

/// Off by default: nothing calls a paid model unless the user opts in per project,
/// or presses "Retry with stronger model" on a stalled thread.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct EscalationPolicy {
    pub enabled: bool,
    pub target: Option<ModelRef>,
    pub triggers: Triggers,
    /// Hearth asks before cloud spend passes this in a calendar month.
    pub monthly_cap_usd: Option<f64>,
}

impl Default for EscalationPolicy {
    fn default() -> Self {
        Self { enabled: false, target: None, triggers: Triggers::default(), monthly_cap_usd: Some(10.0) }
    }
}

/// How many of each failure in a row count as stalled. 0 disables a trigger.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Triggers {
    pub malformed_tool_calls: u32,
    pub repeated_identical_calls: u32,
    pub consecutive_tool_errors: u32,
}

impl Default for Triggers {
    fn default() -> Self {
        Self { malformed_tool_calls: 2, repeated_identical_calls: 3, consecutive_tool_errors: 3 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StallReason {
    MalformedToolCalls { count: u32 },
    RepeatedCall { tool: String, times: u32 },
    ToolErrors { count: u32 },
}

impl StallReason {
    pub fn describe(&self) -> String {
        match self {
            StallReason::MalformedToolCalls { count } => format!("{count} malformed tool calls in a row"),
            StallReason::RepeatedCall { tool, times } => format!("the same {tool} call {times} times in a row"),
            StallReason::ToolErrors { count } => format!("{count} tool errors in a row"),
        }
    }
}

#[derive(Debug, Default)]
pub struct StallDetector {
    triggers: Triggers,
    malformed: u32,
    errors: u32,
    last_call: Option<(String, String)>,
    repeats: u32,
}

impl StallDetector {
    pub fn new(triggers: Triggers) -> Self {
        Self { triggers, ..Default::default() }
    }

    pub fn reset(&mut self) {
        *self = Self::new(self.triggers.clone());
    }

    pub fn on_malformed(&mut self) {
        self.malformed += 1;
        self.errors += 1;
    }

    /// Record a call that parsed. `args` should be canonical JSON so key order doesn't matter.
    pub fn on_call(&mut self, tool: &str, args: &str, is_error: bool) {
        self.malformed = 0;
        let sig = (tool.to_string(), args.to_string());
        if self.last_call.as_ref() == Some(&sig) {
            self.repeats += 1;
        } else {
            self.repeats = 1;
            self.last_call = Some(sig);
        }
        if is_error {
            self.errors += 1;
        } else {
            self.errors = 0;
        }
    }

    pub fn check(&self) -> Option<StallReason> {
        let t = &self.triggers;
        if t.malformed_tool_calls > 0 && self.malformed >= t.malformed_tool_calls {
            return Some(StallReason::MalformedToolCalls { count: self.malformed });
        }
        if t.repeated_identical_calls > 0 && self.repeats >= t.repeated_identical_calls {
            let tool = self.last_call.as_ref().map(|c| c.0.clone()).unwrap_or_default();
            return Some(StallReason::RepeatedCall { tool, times: self.repeats });
        }
        if t.consecutive_tool_errors > 0 && self.errors >= t.consecutive_tool_errors {
            return Some(StallReason::ToolErrors { count: self.errors });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_is_off() {
        assert!(!EscalationPolicy::default().enabled);
    }

    #[test]
    fn detects_repeats_and_errors() {
        let mut d = StallDetector::new(Triggers::default());
        d.on_call("read_file", r#"{"path":"a"}"#, false);
        d.on_call("read_file", r#"{"path":"a"}"#, false);
        assert_eq!(d.check(), None);
        d.on_call("read_file", r#"{"path":"a"}"#, false);
        assert!(matches!(d.check(), Some(StallReason::RepeatedCall { times: 3, .. })));

        let mut d = StallDetector::new(Triggers::default());
        d.on_call("a", "1", true);
        d.on_call("b", "2", true);
        assert_eq!(d.check(), None);
        d.on_call("c", "3", true);
        assert_eq!(d.check(), Some(StallReason::ToolErrors { count: 3 }));
        d.on_call("d", "4", false);
        assert_eq!(d.check(), None);
    }

    #[test]
    fn detects_malformed() {
        let mut d = StallDetector::new(Triggers::default());
        d.on_malformed();
        assert_eq!(d.check(), None);
        d.on_malformed();
        assert_eq!(d.check(), Some(StallReason::MalformedToolCalls { count: 2 }));
    }
}
