import {
  ChevronRight, FolderClosed, Inbox, Lightbulb, Library, MessageSquare, MessagesSquare,
  PanelLeft, Plus, Search, Settings, SlidersHorizontal, Target, Waypoints,
} from "lucide-react";
import { chats, projects } from "../data/sample";
import { platform, shortcut } from "../native/platform";
import type { View } from "../App";

type Props = {
  view: View;
  go: (v: View) => void;
  mode: "full" | "rail";
  toggleMode: () => void;
  needsYou: number;
  collapsed: Record<string, boolean>;
  toggle: (key: string) => void;
  openSettings: () => void;
};

const projectPages = [
  { page: "coordinator", label: "Coordinator", icon: Waypoints },
  { page: "threads", label: "Threads", icon: MessagesSquare },
  { page: "ideas", label: "Ideas", icon: Lightbulb },
  { page: "goals", label: "Goals", icon: Target },
  { page: "library", label: "Library", icon: Library },
  { page: "settings", label: "Project settings", icon: SlidersHorizontal },
] as const;

export function Sidebar({ view, go, mode, toggleMode, needsYou, collapsed, toggle, openSettings }: Props) {
  const rail = mode === "rail";
  const current = (v: View) => (JSON.stringify(v) === JSON.stringify(view) ? "page" : undefined);

  return (
    <nav className="sidebar" aria-label="Sidebar">
      <div className="sidebar-top" data-tauri-drag-region>
        <button className="icon-btn" aria-label={rail ? "Show sidebar" : "Collapse sidebar"} title={`Toggle sidebar (${shortcut(platform === "macos" ? { ctrl: true, mod: true, key: "s" } : { ctrl: true, key: "b" })})`} onClick={toggleMode}>
          <PanelLeft size={16} />
        </button>
      </div>

      <div className="sidebar-scroll">
        <button className="row" title="Search" onClick={() => go({ kind: "search" })} aria-current={current({ kind: "search" })}>
          <Search size={16} aria-hidden />
          <span className="label">Search</span>
          <kbd className="label">{shortcut({ mod: true, key: "k" })}</kbd>
        </button>
        <button className="row" title="Overview" onClick={() => go({ kind: "overview" })} aria-current={current({ kind: "overview" })}>
          <Inbox size={16} aria-hidden />
          <span className="label">Overview</span>
          {needsYou > 0 && <span className="badge" aria-label={`${needsYou} need you`}>{needsYou}</span>}
        </button>

        <Group id="chats" title="Chats" collapsed={collapsed} toggle={toggle} rail={rail}>
          {chats.map((c) => (
            <button key={c.id} className="row" title={c.title} onClick={() => go({ kind: "chat", id: c.id })} aria-current={current({ kind: "chat", id: c.id })}>
              <MessageSquare size={16} aria-hidden />
              <span className="label">{c.title}</span>
            </button>
          ))}
          <button className="row quiet-row" title="New chat" onClick={() => go({ kind: "chat", id: "new" })}>
            <Plus size={16} aria-hidden />
            <span className="label">New chat</span>
          </button>
        </Group>

        <Group id="projects" title="Projects" collapsed={collapsed} toggle={toggle} rail={rail}>
          {projects.map((p) => {
            const key = `project:${p.id}`;
            const open = !collapsed[key];
            return (
              <div key={p.id}>
                <button className="row" title={p.name} aria-expanded={open} onClick={() => toggle(key)}>
                  {rail ? <FolderClosed size={16} aria-hidden /> : <ChevronRight size={14} className="chevron" aria-hidden />}
                  <span className="label">{p.name}</span>
                </button>
                {open && !rail && (
                  <div className="subrows">
                    {projectPages.map(({ page, label, icon: Icon }) => (
                      <button key={page} className="row" onClick={() => go({ kind: "project", project: p.id, page })} aria-current={current({ kind: "project", project: p.id, page })}>
                        <Icon size={15} aria-hidden />
                        <span className="label">{label}</span>
                        {page === "threads" && <span className="count">{p.threads}</span>}
                      </button>
                    ))}
                  </div>
                )}
              </div>
            );
          })}
        </Group>
      </div>

      <footer className="sidebar-foot">
        <div className="box-status label" title="The LLM box, reached over Tailscale">
          <span className="dot" aria-hidden /> hearth-box online
        </div>
        <div className="caption label">Cloud spend this month: $0.00</div>
        <button className="row" title="Settings" onClick={openSettings} aria-current={view.kind === "settings" ? "page" : undefined}>
          <Settings size={16} aria-hidden />
          <span className="label">Settings</span>
          <kbd className="label">{shortcut({ mod: true, key: "," })}</kbd>
        </button>
      </footer>
    </nav>
  );
}

function Group({ id, title, collapsed, toggle, rail, children }: { id: string; title: string; collapsed: Record<string, boolean>; toggle: (k: string) => void; rail: boolean; children: React.ReactNode }) {
  const open = !collapsed[id];
  return (
    <section className="group">
      {!rail && (
        <button className="group-head" aria-expanded={open} onClick={() => toggle(id)}>
          {title}
          <ChevronRight size={12} className="chevron" aria-hidden />
        </button>
      )}
      {(open || rail) && children}
    </section>
  );
}
