# Hearth UI and design requirements

The goal: an interface very close to Claude's own projects experience, in Stefan's colors, that feels native on macOS. The clickable reference is [mockup/index.html](mockup/index.html). When the mockup and this file disagree, this file wins.

## Principles

1. **Native first on macOS** (Apple Human Interface Guidelines): system font (SF Pro via `-apple-system`), traffic lights in a unified title bar (`titleBarStyle: Overlay`, hidden title), translucent sidebar (vibrancy, `NSVisualEffectView` sidebar material via the `window-vibrancy` crate), a real app menu bar with standard ⌘ shortcuts, native context menus, native notifications, a Settings window opened with ⌘,, and a template (monochrome) menu bar icon. Respect the user's accent color for focus rings where possible, reduced motion, and increased contrast.
2. **Windows gets the Fluent equivalents:** Mica backdrop, Segoe UI Variable, tray icon, standard Ctrl shortcuts.
3. **Follow the system appearance** (light or dark), with an override in Settings > General.
4. **Everything collapses:** the sidebar (down to an icon rail), the Chats and Projects groups, each project, the thread list, groups of tool calls, and the artifacts pane. Remember collapse state per window.
5. **Quiet by default:** settings and rarely used controls live in the lower-left Settings menu, not in the thread view.

## Palette (from Stefan's whitepaper style guide)

Accents keep their whitepaper meaning so a glance at a thread list says what needs attention.

| Token | Hex | Use |
|---|---|---|
| Teal | `#12727D` | Working, primary buttons, the local model, success values |
| Copper | `#C9853B` | Needs you, approvals, decisions, artifacts, the flame |
| Brick | `#B94A3E` | Failed, errors, anything that will cost money |
| Slate | `#204254` | Labels, table heads, review state, quiet structure |
| Ink | `#121B22` | Text in light mode |
| Concrete | `#5A5F65` | Secondary text in light mode |
| Mortar | `#E3E1DD` | Hairlines in light mode, text in dark mode |

Grounds: light mode is white with a warm grey sidebar (`#F2F0ED`); dark mode is ink and slate pushed darker (`#111A21` window, `#0E161C` sidebar). In dark mode, text-colored accents use lighter tints (teal `#46B3BE`, copper `#E0A866`, brick `#E07A6D`, slate `#7FA6BB`). Never introduce a new hue. No bold body text; emphasis comes from size and weight 500.

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
