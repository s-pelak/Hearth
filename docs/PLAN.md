# Hearth: product and architecture plan

Status as of 2026-11-26. This file is the source of truth for what Hearth is and the order it gets built in. UI details live in [UI.md](UI.md); the clickable mockup is [mockup/index.html](mockup/index.html) (open it in a browser).

## What Hearth is

A desktop agent harness that lives in the macOS menu bar and the Windows tray, modeled on Claude's projects experience:

- **Projects** hold instructions, a files folder, default models, enabled connectors and a memory file.
- **Threads** are tasks that run in parallel inside a project, each with its own agent loop.
- A per-project **coordinator** routes requests into threads and reports back.
- **Chats** are plain conversations outside any project.
- **Ideas** are a planning board; **Goals** are recurring or triggered jobs that run agents automatically.
- Threads produce **artifacts** (files), collected in each project's **Library**.
- **Plugins** add tools: MCP servers plus Claude Code style skills.

It talks to local models on Stefan's headless LLM box (llama.cpp / LM Studio, OpenAI-compatible API) and to cloud models (Anthropic, OpenAI, Gemini, others), with optional escalation from a struggling local model to a stronger one.

## Decisions

| Topic | Decision |
|---|---|
| Name | Hearth. App icon is a small flame (see `docs/brand/`). |
| Stack | Tauri 2 shell, Rust agent core (`crates/hearth-core`), React + TypeScript UI. Approved. |
| Repo | github.com/s-pelak/Hearth (this repo). Apache-2.0. |
| Escalation | **Optional and off by default.** Nothing calls a paid model unless the user turns escalation on for a project, or presses "Retry with stronger model" on a stalled thread. Monthly cap (default $10) with a prompt before going past it. |
| Settings | In a menu in the lower left of the sidebar: Escalation, Models, File locations, Connectors and skills, General. On macOS it is also the standard Settings window opened with ⌘,. |
| Look | Follows the system light/dark appearance. Mac app follows Apple's Human Interface Guidelines with the system font (SF Pro); Windows follows Fluent. Colors, shapes and title type from the Hearth app design system v2 (see UI.md). |
| Models | From the 21-model tuning run: Gemma 4 12B (everyday text, chats), Qwen3.6 35B-A3B (code and CAD), Nemotron 3.5 Bolt 74B (agents, threads), all on hearth-box at 300 W; Claude Haiku 5.5 when a failure costs time; Opus 5.5 for CAD from images and large builds. |
| Layout | Very close to Claude's own projects interface, everything collapsible. See UI.md. |
| Signing | Stefan has a free Apple developer account, which cannot sign for distribution. Builds are ad-hoc signed (`signingIdentity: "-"`); first launch needs right-click > Open. Windows SmartScreen asks once. |
| Networking | The LLM box is reached over Tailscale (set up in a separate thread). Its tailnet name becomes the default `hearth-box` endpoint, currently a placeholder `http://hearth-box:8080/v1`. |

## Architecture

```
src-tauri/        Tauri shell: tray/menu bar icon, windows, native menus, keychain, commands
crates/hearth-core/
  provider/       Provider trait + adapters (OpenAI-compatible with SSE streaming; Anthropic Messages API)
  agent.rs        Per-thread agent loop, tool execution, permissions, stall detection, escalation hand-off
  escalation.rs   EscalationPolicy (off by default), StallDetector, triggers
  tools/          Tool trait, registry, built-in file tools scoped to the project folder
  store.rs        SQLite: projects, threads (with status), messages, usage
src/              React UI
docs/             This plan, UI requirements, mockup, brand assets
```

### Providers

- **OpenAI-compatible** covers llama.cpp server, LM Studio, Ollama, OpenAI, Gemini (its OpenAI endpoint), OpenRouter. Streams text and tool-call deltas over SSE; `/models` for discovery.
- **Anthropic** uses the Messages API over raw HTTP (no official Rust SDK). Assistant turns keep their native content blocks (including thinking blocks) and replay them unchanged to Anthropic. Do not send `temperature` (newer models reject it). Model list comes from `/v1/models`; the default escalation suggestion is `claude-haiku-5-5` (cheaper than local per completed task in the tuning run), with `claude-opus-5-5` for CAD from images and large builds. See `hearth_core::models`.
- **Apple Foundation Models** (phase 2, macOS 26+, Apple silicon): a small Swift sidecar exposing the on-device model as a local OpenAI-compatible endpoint. Good for titles, summaries, routing and classification; too small for multi-step agent work.
- API keys live in the OS keychain (Keychain / Credential Manager), never in config files.

### Agent loop and escalation

One loop per thread: call model, run tool calls, append results, repeat, up to a step limit. The `StallDetector` counts three failure patterns that Stefan's benchmark showed small local models hit often:

- malformed tool-call JSON (default trigger: 2 in a row)
- the same call repeated (3 in a row)
- tool errors (3 in a row)

On a stall: if escalation is enabled for the project and a target model is set, the thread switches model and continues with the same transcript. Otherwise the thread pauses with status **needs you** and shows Retry with a stronger model (priced up front), Let it try again, or Edit and continue. Every stall and escalation is logged with its reason.

### Coordinator (per project)

Built to work on a small local model:

- **Event-driven:** wakes only for a message in the project chat, a thread finishing or getting stuck, or a goal firing. One short call per wake.
- **Reads summaries, not transcripts:** project instructions plus each thread's title, status and a two-line summary (a few thousand tokens).
- **Constrained JSON output:** actions like `start_thread`, `reply`, `ask_user` (decision card), `wait`. llama.cpp enforces the schema with a grammar / `response_format: json_schema`.
- **Own model setting:** small local model, Apple FM, or cloud. **Direct mode** turns the coordinator off.
- **Provider queue:** parallel threads are queued to match the LLM box's slot count.

### Context management for small models

Summarize older turns when a thread reaches ~80% of the model's context window (setting), keep the full transcript on disk, and show a "summarized" marker in the thread. This matters more than anything else for local models on long tasks.

### Goals

Recurring (cron-style) or triggered (e.g. "a new model appears on hearth-box") jobs with success criteria, a model and a budget. Each run opens a thread you can review. Local models by default, so goals cost nothing unless a cloud model is chosen.

### Plugins

MCP servers (stdio and HTTP) through the official Rust SDK (`rmcp`), plus skills as folders with a `SKILL.md`, matching Claude Code's formats so existing plugins drop in. Per-tool permission: allow / ask / deny. Shell commands always ask unless allowed.

## Phases

**Phase 1 (MVP)**
1. Tray / menu bar app with quick-ask popover and main window, Mac and Windows. Closing the window keeps Hearth in the menu bar.
2. Providers: OpenAI-compatible (LLM box), Anthropic, OpenAI, Gemini; keys in keychain.
3. Projects (instructions, folder, default model), Chats, Ideas board.
4. Parallel threads with streaming, statuses (working, needs you, ready for review, done, failed), cancel, retry, resolve, reminders.
5. Progress checklists, decision cards and tool approval cards.
6. Built-in tools and MCP plugins with permission prompts.
7. Artifacts pane and project Library, with versions and Open in Finder.
8. Manual model switch and Retry with stronger model. Escalation settings exist but default off.
9. Context summarization for small models.
10. Collapsible layout, system light/dark, native macOS chrome, ⌘K search, Overview inbox.
11. Installers from GitHub Actions (ad-hoc signed).

**Phase 1.5:** project coordinator (event-driven, JSON-constrained), provider queue, project memory file.

**Phase 2:** Goals and the scheduler, automatic escalation triggers wired to settings, Apple Foundation Models provider, notifications, global hotkey, usage and cost tracking, sub-agents (a thread can spawn helper agents shown on its checklist).

**Phase 3:** run hearth-core as a server on the LLM box so the Mac and the Windows desktop share projects (the apps become clients); web/phone client; an evals view that feeds benchmark results into routing defaults.

## What exists in the repo today

- `crates/hearth-core`: types, OpenAI-compatible streaming adapter, Anthropic adapter, file tools with path sandboxing, stall detector and escalation policy, agent loop, SQLite store. `cargo test -p hearth-core` runs 10 tests (agent loop with a scripted provider, escalation only when enabled, SSE parsing against a mock server, path escapes, store round trip).
- `src-tauri`: tray icon (template image), Open/Quit menu, left-click toggles the window, close hides instead of quitting, macOS Dock icon hidden, commands `list_providers` and `list_models`.
- `src`: placeholder UI that lists providers and asks the LLM box for its models.
- CI: tests on Linux; release builds for macOS (universal) and Windows on tags.

## Open items

- Tailscale address of the LLM box (comes from the Tailscale thread).
- Provider settings persistence and the keychain wiring (`keyring` crate).
- MCP client (`rmcp`) integration.
- Anthropic streaming (currently non-streaming; text arrives at the end of each turn).
