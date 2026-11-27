//! The recommended model roster, from Stefan's 21-model tuning run on the
//! LLM box (RTX 3080 Ti, 12 GB, November 2026). Settings and the model picker
//! start from this; the user can change any of it.
//!
//! Findings behind it: three local models are worth keeping, each for its own
//! kind of work, all at a 300 W cap. Where a failure costs time, Claude Haiku
//! 5.5 is cheaper than local per completed task. Opus or Fable are the only
//! models that pass CAD from images and large app builds.

use crate::types::ModelRef;
use serde::Serialize;

/// What a model is for, as the picker shows it.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    EverydayText,
    CodeAndCad,
    AgentsAndLongWork,
    WhenFailureCostsTime,
    HardestWork,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct RecommendedModel {
    pub model: ModelRef,
    pub name: &'static str,
    /// One line for the model picker.
    pub use_for: &'static str,
    pub role: Role,
    pub is_local: bool,
    /// Decode speed measured on the LLM box, for local models.
    pub tokens_per_second: Option<u32>,
}

const BOX: &str = "hearth-box";
const ANTHROPIC: &str = "anthropic";

pub fn recommended() -> Vec<RecommendedModel> {
    let m = |provider: &str, id: &str| ModelRef::new(provider, id);
    vec![
        RecommendedModel {
            model: m(BOX, "gemma-4-12b"),
            name: "Gemma 4 12B",
            use_for: "Everyday text",
            role: Role::EverydayText,
            is_local: true,
            tokens_per_second: Some(81),
        },
        RecommendedModel {
            model: m(BOX, "qwen3.6-35b-a3b"),
            name: "Qwen3.6 35B-A3B",
            use_for: "Code and CAD",
            role: Role::CodeAndCad,
            is_local: true,
            tokens_per_second: Some(73),
        },
        RecommendedModel {
            model: m(BOX, "nemotron-3.5-bolt-74b"),
            name: "Nemotron 3.5 Bolt 74B",
            use_for: "Agents and long documents",
            role: Role::AgentsAndLongWork,
            is_local: true,
            tokens_per_second: Some(48),
        },
        RecommendedModel {
            model: m(ANTHROPIC, "claude-haiku-5-5"),
            name: "Claude Haiku 5.5",
            use_for: "When a failure would cost you time",
            role: Role::WhenFailureCostsTime,
            is_local: false,
            tokens_per_second: None,
        },
        RecommendedModel {
            model: m(ANTHROPIC, "claude-opus-5-5"),
            name: "Claude Opus 5.5",
            use_for: "CAD from images and large builds",
            role: Role::HardestWork,
            is_local: false,
            tokens_per_second: None,
        },
    ]
}

fn pick(role: Role) -> ModelRef {
    recommended().into_iter().find(|r| r.role == role).map(|r| r.model).expect("every role has a model")
}

/// Plain chats outside projects.
pub fn default_chat_model() -> ModelRef {
    pick(Role::EverydayText)
}

/// Threads run the agent loop, and grow their context with every step;
/// Nemotron keeps its full speed at 32k context.
pub fn default_thread_model() -> ModelRef {
    pick(Role::AgentsAndLongWork)
}

/// What "Retry with stronger model" offers first. Escalation itself stays off
/// unless the user turns it on.
pub fn suggested_escalation_model() -> ModelRef {
    pick(Role::WhenFailureCostsTime)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_follow_the_tuning_results() {
        assert_eq!(default_chat_model().label(), "hearth-box/gemma-4-12b");
        assert_eq!(default_thread_model().label(), "hearth-box/nemotron-3.5-bolt-74b");
        assert_eq!(suggested_escalation_model().label(), "anthropic/claude-haiku-5-5");
        assert_eq!(recommended().iter().filter(|r| r.is_local).count(), 3);
    }
}
