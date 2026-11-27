# Hearth UI and design requirements

The goal: an interface very close to Claude's own projects experience, in Stefan's colors, that feels native on macOS. The clickable reference is [mockup/index.html](mockup/index.html). When the mockup and this file disagree, this file wins.

## Principles

1. **Native first on macOS** (Apple Human Interface Guidelines): system font (SF Pro via `-apple-system`), traffic lights in a unified title bar (`titleBarStyle: Overlay`, hidden title), translucent sidebar (vibrancy, `NSVisualEffectView` sidebar material via the `window-vibrancy` crate), a real app menu bar with standard ⌘ shortcuts, native context menus, native notifications, a Settings window opened with ⌘,, and a template (monochrome) menu bar icon. Respect the user's accent color for focus rings where possible, reduced motion, and increased contrast.
2. **Windows gets the Fluent equivalents:** Mica backdrop, Segoe UI Variable, tray icon, standard Ctrl shortcuts.
3. **Follow the system appearance** (light or dark), with an override in Settings > General.
4. **Everything collapses:** the sidebar (down to an icon rail), the Chats and Projects groups, each project, the thread list, groups of tool calls, and the artifacts pane. Remember collapse state per window.
5. **Quiet by default:** settings and rarely used controls live in the lower-left Settings menu, not in the thread view.

## Native shell (implemented)

Color-independent platform work lives in `src-tauri/src/native.rs` and `src/native/`:

- **Materials:** the main window is transparent; macOS gets `NSVisualEffectView` sidebar vibrancy, Windows 11 gets Mica. The UI reads which one applied (`window_chrome`) and sets `data-material` on `<html>`, so Windows 10 and Linux stay opaque.
- **macOS menu bar:** Hearth, File, Edit, View, Thread, Window, Help with standard ⌘ shortcuts (⌘, Settings, ⌘N New Thread, ⌘K Search, ⌃⌘S Toggle Sidebar, ⌘1 Overview). Custom items reach the UI as `menu` events.
- **Dock presence:** Hearth is a regular app (Dock icon and menu bar) while a window is open, and drops back to menu bar only when every window is closed.
- **Settings:** its own window on macOS (⌘,); a page inside the main window on Windows, as Fluent apps do. Windows binds the same commands to Ctrl shortcuts in `platform.ts`.
- **Styles:** `platform.css` holds type ramps, control metrics, selection, focus and switch styles per platform (HIG on macOS, Fluent 2 on Windows). Colors come only from the brand tokens below.
- **Platform differences in the UI:** macOS uses a translucent sidebar, grey selection that dims when the window is in the background, a Settings window with icon tabs, and ⌘ shortcuts. Windows uses Mica under the navigation pane, a content layer with a rounded top-left corner, a short pill marking the selected item, Settings as a page with categories on the left, and Ctrl shortcuts.

## Brand: Hearth app design system v2

Source: `Hearth-app-design-system-v2.pdf` (27 Nov 2026). The tokens ship unchanged as `src/native/hearth-tokens.css` (and `docs/brand/hearth-tokens.json`); `src/native/brand.css` maps them to app roles. Use `--hearth-*` tokens, never hex values.

| Role | Light | Dark | Use in Hearth |
|---|---|---|---|
| ground | Cream `#FAF5EF` | Night `#12222B` | Window content; the sidebar is ground mixed toward Mortar |
| surface | `#FFFFFF` | Hearthstone `#1A2F3A` | Cards, the composer, menus, artifacts pane, selected thread |
| structure | Slate `#204254` | Cream | Icons, secondary and quiet buttons, toggles when on, "Working", focus |
| answer | Teal `#12727D` | Glow teal `#4FB0BA` | File and source chips, "Ready for review", hearth-box online |
| flame | Copper `#C9853B` | Copper | The one primary action per screen (Send, or Allow once / Retry when a thread waits on you), active tab underline |
| risk | Brick `#B94A3E` | Glow brick `#E07B6D` | "Needs you", "Failed", caution notes, failed tool calls, the Overview badge |

Rules taken from the design system:

- **One flame per screen.** When a thread shows an approval or a stall, that button is Copper and Send drops to the secondary style.
- **Two status colors.** Brick for anything that needs care, Teal for success; Working uses structure. Every status is a word plus an icon, never color alone. This replaces the earlier "copper = needs you" mapping, because Copper is reserved for the primary action and Copper text fails contrast.
- **Shapes:** pill buttons and chips, 12 px cards and inputs, 20 px dialogs; cards get a 1 px Mortar edge; only floating layers (menus, dialogs) get the one shadow.
- **Type:** Fraunces (bundled, SOFT axis) for screen titles at 22 px and up only. Everything else uses the platform system font (SF Pro on macOS, Segoe UI Variable on Windows) instead of the brand's Inter, so the app reads as native. Answers and messages use 15 px with a 1.6 line height.
- **Focus:** 2 px ring in the focus token, 2 px from the control, on every interactive element.
- **Answer anatomy:** the user's message is a hairline bubble on the right; answers have no bubble, show the model that wrote them, and end in quiet Copy / Retry. Cautions are a 3 px Brick bar starting with "Caution:". The confidence line is not shown yet; the design system says to switch it on only once confidence is reliable.

## Models (from the 21-model tuning run, 27 Nov 2026)

`hearth_core::models::recommended()` holds the roster the pickers and Settings start from:

| Model | Where | Use for | Default for |
|---|---|---|---|
| Gemma 4 12B | hearth-box, 81 tok/s | Everyday text | Chats |
| Qwen3.6 35B-A3B | hearth-box, 73 tok/s | Code and CAD | |
| Nemotron 3.5 Bolt 74B | hearth-box, 48 tok/s, full speed at 32k | Agents and long documents | Threads |
| Claude Haiku 5.5 | Anthropic | When a failure would cost you time | Retry with a stronger model |
| Claude Opus 5.5 | Anthropic | CAD from images and large builds | |

Local models run at a 300 W cap. Escalation stays off by default; Haiku is only what the Retry button offers first.

## Window layout

Four columns, each collapsible:

1. **Sidebar** (220 px, collapses to a 56 px icon rail)
   - Top: Search (⌘K), Overview (badge with the needs-you count)
   - **Chats** group: plain chats outside projects
   - **Projects** group: each project expands to Coordinator, Threads, Ideas, Goals, Library, Project settings
   - Bottom: LLM box status (online over Tailscale), cloud spend this month, **Settings** button
2. **Thread list** (260 px): title, status pill, last activity. Hidden in non-thread views.
3. **Main view:** thread, coordinator chat, ideas board, goals, library, project settings, overview, or chat.
4. **Artifacts pane** (290 px): tabs per artifact, version and source thread, live preview.

## Views

- **Overview:** across all projects, grouped and collapsible: Needs you, Ready for review, Working, Reminders, Done today. Each card has inline actions (Retry, Allow once / Deny, Answer, Mark done, Open).
- **Thread:** header with title, model chip, status pill, context meter (used / window, turns copper above 80%), a ⋯ menu (Mark done, Remind me in 1 day / 1 week, Rename, Move to project, Switch model, Export transcript, Archive) and the artifacts toggle. Feed items:
  - user and assistant messages (assistant labeled with the model that wrote it)
  - **progress checklist** card (✓ done, ✱ in progress, ○ not started; steps can show "2 helpers" for sub-agents)
  - **tool call groups**, collapsed by default with a count; groups containing failures open automatically and show the failure count in brick
  - **approval card** (copper): Allow once, Always allow for this domain/tool, Deny
  - **stall card** when a local model gets stuck: reason, then Retry with a stronger model (with estimated cost), Let it try again, Edit and continue
  - **summarized marker** when older turns were summarized to fit the context window, with Show
  - **ready for review** card: Mark done, Ask for changes
  - artifact chips
- **Composer:** attach button, drag and drop, `@` to mention files or threads, model chip, tool count, token and cost readout, Send.
- **Coordinator:** chat with the coordinator, event lines (thread needs you, thread finished), and **decision cards**: a question with 2 to 4 options and one marked Recommended.
- **Ideas:** board with Inbox, Planning, Promoted. Cards have Start thread and Make goal.
- **Goals:** rows with name and description, schedule or trigger plus last run, model and budget, and an on/off switch. Each run opens a thread.
- **Library:** table of every file and artifact in the project (name, kind, version, source thread, updated, size) and Open in Finder.
- **Project settings:** instructions, folder, models (threads, coordinator, escalation), coordinator on / Direct mode, enabled connectors and skills, memory file (agents can read and update it), context summarization threshold.
- **Chat:** simple conversation, model chip, no tools by default.
- **⌘K palette:** search threads, files, ideas and commands (new thread with a model, load a model on the box).

## App Settings (lower left, and ⌘, on macOS)

- **Escalation:** auto-escalate (off by default), escalation model, triggers (malformed tool calls ×2, same call ×3, tool errors ×3), monthly cap, per-project overrides. Note: with auto-escalate off, a stalled thread pauses and shows a Retry button with the estimated cost.
- **Models:** providers (LLM box, Anthropic, OpenAI, Gemini, Apple Foundation Models), connection status, add key.
- **File locations:** projects folder (`~/Hearth/Projects`), app data, plugin cache, optional sync with the LLM box (phase 3).
- **Connectors and skills:** MCP servers and skills with on/off switches, Add.
- **General:** quick-ask hotkey (⌥Space), open at login, appearance (Match system / Light / Dark).

## Menu bar popover

Opened by clicking the flame (template icon; a copper dot when something needs you) or the global hotkey. Contains a project picker, a model picker, an input ("Ask Hearth, or start a thread"), the active threads with status pills, LLM box status with tokens per second, and Open Hearth.

## Gaps to cover (compared with Claude's projects UI)

All of these are in the mockup and the plan; listed here so none get dropped during implementation:

1. Overview inbox across projects, plus a badge on the menu bar icon.
2. Progress checklists per thread.
3. Decision cards and tool approval cards.
4. Thread lifecycle: working, needs you, ready for review, done, failed; resolve, archive, reminders.
5. Library with versions and Open in Finder.
6. Project settings and a project memory file.
7. Context management (summaries) for small local models.
8. Attachments, drag and drop, `@` mentions.
9. ⌘K command palette.
10. Sub-agents shown on the checklist.
11. Usage meter: tokens and cost per thread and project.
