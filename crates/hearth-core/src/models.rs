//! The local models Hearth is set up for, and which one does which job.
//!
//! Source: Stefan's tuning run of 21 models on the LLM box (November 2026). It recommends
//! keeping three: Gemma-4-12B for everyday text, Qwen3.6-35B-A3B for code and CAD, and
//! Nemotron-3.5-Bolt-74B for agents and long documents, with thinking set per model.
//! Other models the box serves still show up in its model list, but Hearth's defaults
//! only ever point at these three.

use crate::types::{ModelRef, ModelSettings, Sampling};
use serde::{Deserialize, Serialize};

/// Provider id of the LLM box.
pub const LOCAL_PROVIDER: &str = "hearth-box";

/// What a model is kept for.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Strength {
    /// Q&A, emails, summaries, reading images.
    EverydayText,
    /// Code helpers and CAD from text.
    CodeAndCad,
    /// Multi-step tool use and long documents.
    AgentsAndLongWork,
}

/// One of the kept local models.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LocalModel {
    /// Canonical id. Served ids are matched loosely (see [`find`]).
    pub id: &'static str,
    pub name: &'static str,
    pub strength: Strength,
    /// Request settings from the tuning run.
    pub settings: ModelSettings,
    /// Tokens. The tuning run passed long-document items at 46–54k on all three.
    pub context_window: u32,
}

impl LocalModel {
    pub fn model_ref(&self) -> ModelRef {
        ModelRef::new(LOCAL_PROVIDER, self.id)
    }
}

pub const GEMMA: &str = "gemma-4-12b";
pub const QWEN: &str = "qwen3.6-35b-a3b";
pub const NEMOTRON: &str = "nemotron-3.5-bolt-74b";

/// The three kept models, sorted by id so the list is stable.
pub fn kept() -> Vec<LocalModel> {
    vec![
        LocalModel {
            id: GEMMA,
            name: "Gemma 4 12B",
            strength: Strength::EverydayText,
            // Thinking off scored 43% (it still reasons and runs out of tokens); on scored 87%.
            // The sampling search lifted it from 0.870 to 0.957 with these values.
            settings: ModelSettings {
                thinking: Some(true),
                sampling: Some(Sampling { temperature: Some(1.3), top_p: Some(1.0), repeat_penalty: Some(1.05) }),
            },
            context_window: 65_536,
        },
        LocalModel {
            id: NEMOTRON,
            name: "Nemotron 3.5 Bolt 74B",
            strength: Strength::AgentsAndLongWork,
            // Only runs with thinking on; 100% on the tune items. Model card sampling.
            settings: ModelSettings { thinking: Some(true), sampling: None },
            context_window: 65_536,
        },
        LocalModel {
            id: QWEN,
            name: "Qwen3.6 35B A3B",
            strength: Strength::CodeAndCad,
            // Thinking off scored 83% in 7.9 s an item; on scored 76% in 57 s. Model card sampling.
            settings: ModelSettings { thinking: Some(false), sampling: None },
            context_window: 65_536,
        },
    ]
}

/// The kinds of work Hearth hands to a model.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Project threads: tool use over many steps.
    Threads,
    /// The project coordinator: routing and JSON actions over thread summaries.
    Coordinator,
    /// Chats outside projects, no tools.
    Chat,
    /// Threads the user marks as code or CAD work.
    Code,
    /// Summarizing older turns near the context limit.
    Summaries,
}

/// Default model for each role.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RoleDefaults {
    pub threads: ModelRef,
    pub coordinator: ModelRef,
    pub chat: ModelRef,
    pub code: ModelRef,
    pub summaries: ModelRef,
}

impl Default for RoleDefaults {
    fn default() -> Self {
        let m = |id: &str| ModelRef::new(LOCAL_PROVIDER, id);
        Self {
            // Agents: Nemotron 11 of 13 tasks, Gemma 10, Qwen3.6 9; no slowdown at 32k.
            threads: m(NEMOTRON),
            coordinator: m(NEMOTRON),
            // Everyday Q&A: Gemma 31 of 32, Nemotron 30, Qwen3.6 27.
            chat: m(GEMMA),
            // CAD from text 4 of 4, best local code (0.97).
            code: m(QWEN),
            // Long documents: all three pass; Nemotron keeps full speed at long context.
            summaries: m(NEMOTRON),
        }
    }
}

impl RoleDefaults {
    pub fn get(&self, role: Role) -> &ModelRef {
        match role {
            Role::Threads => &self.threads,
            Role::Coordinator => &self.coordinator,
            Role::Chat => &self.chat,
            Role::Code => &self.code,
            Role::Summaries => &self.summaries,
        }
    }
}

/// Reduce a served id to its model name: drop the publisher, a GGUF suffix and the
/// quantization tag, then keep only lowercase letters and digits.
/// `lmstudio-community/Gemma-4-12B-it-GGUF@Q4_K_M` and `gemma-4-12b` both become `gemma412b`.
fn normalize(served: &str) -> String {
    let mut s = served.rsplit('/').next().unwrap_or(served).to_lowercase();
    for sep in ['@', ':'] {
        if let Some(i) = s.find(sep) {
            s.truncate(i);
        }
    }
    for suffix in [".gguf", "-gguf", "_gguf"] {
        if let Some(stripped) = s.strip_suffix(suffix) {
            s = stripped.to_string();
        }
    }
    // Trailing quant tag such as -Q4_K_M, .IQ4_XS, -UD-Q4_K_XL, -F16.
    let parts: Vec<&str> = s.split(['-', '.', '_']).collect();
    let is_quant = |p: &str| {
        let p = p.strip_prefix('i').unwrap_or(p);
        (p.starts_with('q') && p[1..].chars().next().is_some_and(|c| c.is_ascii_digit())) || p == "f16" || p == "bf16" || p == "f32"
    };
    let end = parts.iter().position(|p| is_quant(p)).unwrap_or(parts.len());
    let mut keep = parts[..end].to_vec();
    // Build labels that do not change the model: Unsloth dynamic, instruction-tuned.
    while matches!(keep.last(), Some(&("ud" | "it" | "instruct"))) {
        keep.pop();
    }
    keep.join("-")
        .chars().filter(|c| c.is_ascii_alphanumeric()).collect()
}

/// The kept model a served id refers to, if any. Variants such as Gemma-4-12B heretic
/// or Qwen3.6-35B REAP50 do not match.
pub fn find(served: &str) -> Option<LocalModel> {
    let n = normalize(served);
    kept().into_iter().find(|m| normalize(m.id) == n)
}

/// The served id for each kept model, given what the box reports from `/models`.
pub fn resolve(served: &[String]) -> Vec<(LocalModel, Option<String>)> {
    kept()
        .into_iter()
        .map(|m| {
            let hit = served.iter().find(|s| normalize(s) == normalize(m.id)).cloned();
            (m, hit)
        })
        .collect()
}

/// Request settings for a model. Cloud and unknown models get none, so nothing extra is sent.
pub fn settings_for(model: &ModelRef) -> ModelSettings {
    if model.provider != LOCAL_PROVIDER {
        return ModelSettings::default();
    }
    find(&model.model).map(|m| m.settings).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exactly_three_sorted() {
        let ids: Vec<_> = kept().iter().map(|m| m.id).collect();
        assert_eq!(ids, vec![GEMMA, NEMOTRON, QWEN]);
    }

    #[test]
    fn matches_served_ids() {
        for s in ["gemma-4-12b", "lmstudio-community/Gemma-4-12B-GGUF@Q4_K_M", "unsloth/gemma-4-12b-it-UD-Q4_K_XL.gguf"] {
            assert_eq!(find(s).map(|m| m.id), Some(GEMMA), "{s}");
        }
        assert_eq!(find("Qwen3.6-35B-A3B-Q4_K_M").map(|m| m.id), Some(QWEN));
        assert_eq!(find("nvidia/Nemotron-3.5-Bolt-74B-GGUF").map(|m| m.id), Some(NEMOTRON));
        for s in ["gemma-4-12b-heretic", "Qwen3.6-35B-REAP50", "qwen3.5-9b", "gemma-4-26b-a4b"] {
            assert!(find(s).is_none(), "{s}");
        }
    }

    #[test]
    fn resolve_reports_missing() {
        let served = vec!["Gemma-4-12B-Q4_K_M".to_string(), "qwen3.5-9b".to_string()];
        let r = resolve(&served);
        assert_eq!(r[0].1.as_deref(), Some("Gemma-4-12B-Q4_K_M"));
        assert!(r[1].1.is_none() && r[2].1.is_none());
    }

    #[test]
    fn defaults_use_only_kept_models() {
        let d = RoleDefaults::default();
        for role in [Role::Threads, Role::Coordinator, Role::Chat, Role::Code, Role::Summaries] {
            let m = d.get(role);
            assert_eq!(m.provider, LOCAL_PROVIDER);
            assert!(find(&m.model).is_some());
        }
        assert_eq!(d.threads.model, NEMOTRON);
        assert_eq!(d.code.model, QWEN);
        assert_eq!(d.chat.model, GEMMA);
    }

    #[test]
    fn settings_only_for_local_models() {
        assert_eq!(settings_for(&ModelRef::new(LOCAL_PROVIDER, QWEN)).thinking, Some(false));
        assert!(settings_for(&ModelRef::new(LOCAL_PROVIDER, GEMMA)).sampling.is_some());
        assert_eq!(settings_for(&ModelRef::new("anthropic", "claude-haiku-5-5")), ModelSettings::default());
    }
}
