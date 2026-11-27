// Mirrors hearth_core::models::recommended(), so the browser preview has the
// same roster. In the app the list comes from the `recommended_models` command.

export type RecommendedModel = {
  model: { provider: string; model: string };
  name: string;
  use_for: string;
  role: "everyday_text" | "code_and_cad" | "agents_and_long_work" | "when_failure_costs_time" | "hardest_work";
  is_local: boolean;
  tokens_per_second: number | null;
};

export const recommendedModels: RecommendedModel[] = [
  { model: { provider: "hearth-box", model: "gemma-4-12b" }, name: "Gemma 4 12B", use_for: "Everyday text", role: "everyday_text", is_local: true, tokens_per_second: 81 },
  { model: { provider: "hearth-box", model: "qwen3.6-35b-a3b" }, name: "Qwen3.6 35B-A3B", use_for: "Code and CAD", role: "code_and_cad", is_local: true, tokens_per_second: 73 },
  { model: { provider: "hearth-box", model: "nemotron-3.5-bolt-74b" }, name: "Nemotron 3.5 Bolt 74B", use_for: "Agents and long documents", role: "agents_and_long_work", is_local: true, tokens_per_second: 48 },
  { model: { provider: "anthropic", model: "claude-haiku-5-5" }, name: "Claude Haiku 5.5", use_for: "When a failure would cost you time", role: "when_failure_costs_time", is_local: false, tokens_per_second: null },
  { model: { provider: "anthropic", model: "claude-opus-5-5" }, name: "Claude Opus 5.5", use_for: "CAD from images and large builds", role: "hardest_work", is_local: false, tokens_per_second: null },
];

export const modelName = (id: string) => recommendedModels.find((m) => m.model.model === id)?.name ?? id;
