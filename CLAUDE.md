# Hearth: notes for agents working in this repo

Read docs/PLAN.md (what and why, phases) and docs/UI.md (design requirements) before changing behavior or UI. docs/mockup/index.html is the visual reference; docs/UI.md wins where they differ.

## Layout
- `crates/hearth-core`: all logic (providers, agent loop, tools, escalation, SQLite store). No Tauri or UI code here, so it stays testable headless and can later run as a server on the LLM box.
- `src-tauri`: thin shell (tray/menu bar, windows, native menus, keychain, commands that call hearth-core).
- `src`: React + TypeScript UI.

## Rules
- Escalation to paid models is off by default. Never add a code path that calls a cloud model without the user enabling it for the project or pressing Retry with stronger model.
- API keys go in the OS keychain, never in files or logs.
- File tools must stay confined to the project folder (`tools::fs::resolve`).
- Keep the tool list order stable between calls (the registry is sorted) so prompt caching works.
- macOS UI follows Apple's Human Interface Guidelines: system font, native chrome, ⌘ shortcuts, Settings on ⌘,. Follow system light/dark. Colors only from the palette in docs/UI.md.
- Anthropic adapter: replay assistant `native` content blocks unchanged; do not send `temperature`.

## Commands
- `cargo test -p hearth-core`: core tests (no GUI).
- `npm run build && cargo check -p hearth`: type-check the shell (needs `dist/`).
- `npm run tauri dev`: run the app.

## Context
- The LLM box is a headless machine serving OpenAI-compatible endpoints (llama.cpp / LM Studio), reached over Tailscale. Its default provider id is `hearth-box`; the URL is a placeholder until the tailnet name is known.
- The box keeps three models: `gemma-4-12b` (everyday text), `qwen3.6-35b-a3b` (code and CAD) and `nemotron-3.5-bolt-74b` (agents, long documents; the default for threads and the coordinator). Their roles and tuned settings live in `crates/hearth-core/src/models.rs`; don't add defaults that point at other local models.
- Stefan's benchmarks showed small local models often fail agentic tasks through malformed tool calls, repeated calls and tool errors. That is why stall detection, decision cards, context summarization and optional escalation are core features.
