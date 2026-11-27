import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ChevronRight, Cpu, FolderOpen, Gauge, Plug, Settings2 } from "lucide-react";
import { projects, threads, type Status } from "../data/sample";
import { recommendedModels, type RecommendedModel } from "../data/models";
import { StatusTag, Switch } from "./bits";
import { platform } from "../native/platform";

const inTauri = "__TAURI_INTERNALS__" in window;

/* ---------------- Overview ---------------- */

const sections: Array<{ status: Status; title: string }> = [
  { status: "needs_you", title: "Needs you" },
  { status: "failed", title: "Failed" },
  { status: "review", title: "Ready for review" },
  { status: "working", title: "Working" },
  { status: "done", title: "Done today" },
];

export function Overview({ open }: { open: (threadId: string) => void }) {
  return (
    <section className="page" aria-label="Overview">
      <header className="page-head" data-tauri-drag-region>
        <h1 className="title-m">Overview</h1>
      </header>
      <div className="page-body">
        <div className="column">
          {sections.map(({ status, title }) => {
            const list = threads.filter((t) => t.status === status);
            if (list.length === 0) return null;
            return (
              <details key={status} className="overview-group" open>
                <summary>
                  <ChevronRight size={13} className="chevron" aria-hidden />
                  {title}
                  <span className="caption">{list.length}</span>
                </summary>
                {list.map((t) => (
                  <div key={t.id} className="card overview-card">
                    <div>
                      <div>{t.title}</div>
                      <div className="caption">
                        {projects.find((p) => p.id === t.project)?.name}, {t.updated}
                      </div>
                    </div>
                    <StatusTag status={t.status} />
                    <div className="actions">
                      {status === "failed" && <button className="btn quiet small">Retry</button>}
                      {status === "review" && <button className="btn quiet small">Mark done</button>}
                      <button className="btn secondary small" onClick={() => open(t.id)}>Open</button>
                    </div>
                  </div>
                ))}
              </details>
            );
          })}
        </div>
      </div>
    </section>
  );
}

/* ---------------- Not built yet ---------------- */

export function Placeholder({ title }: { title: string }) {
  return (
    <section className="page">
      <header className="page-head" data-tauri-drag-region>
        <h1 className="title-m">{title}</h1>
      </header>
      <div className="page-body">
        <p className="muted column">This view isn't built yet. It follows the threads view in the plan.</p>
      </div>
    </section>
  );
}

/* ---------------- Settings ---------------- */

const panes = [
  { id: "general", label: "General", icon: Settings2 },
  { id: "models", label: "Models", icon: Cpu },
  { id: "escalation", label: "Escalation", icon: Gauge },
  { id: "files", label: "File locations", icon: FolderOpen },
  { id: "connectors", label: "Connectors and skills", icon: Plug },
] as const;
type Pane = (typeof panes)[number]["id"];

/**
 * macOS: its own window with a toolbar of icon tabs, as Mac Settings windows are.
 * Windows: a page in the main window with a list of categories on the left.
 */
export function Settings({ layout }: { layout: "toolbar" | "page" }) {
  const [pane, setPane] = useState<Pane>("general");
  const nav = (
    <div className={layout === "toolbar" ? "settings-toolbar" : "settings-nav"} role="tablist" aria-label="Settings">
      {panes.map(({ id, label, icon: Icon }) => (
        <button key={id} role="tab" aria-selected={pane === id} onClick={() => setPane(id)}>
          <Icon size={layout === "toolbar" ? 20 : 16} aria-hidden />
          <span>{label}</span>
        </button>
      ))}
    </div>
  );
  const body = (
    <div className="settings-pane">
      {pane === "general" && <General />}
      {pane === "models" && <Models />}
      {pane === "escalation" && <Escalation />}
      {pane === "files" && <Files />}
      {pane === "connectors" && <p className="muted">No connectors or skills yet. MCP servers and skill folders you add show up here.</p>}
    </div>
  );

  if (layout === "toolbar")
    return (
      <div className="settings-window">
        {nav}
        {body}
      </div>
    );
  return (
    <section className="page">
      <header className="page-head" data-tauri-drag-region>
        <h1 className="title-m">Settings</h1>
      </header>
      <div className="settings-split">
        {nav}
        {body}
      </div>
    </section>
  );
}

type Appearance = "system" | "light" | "dark";

export function applyAppearance(a: Appearance) {
  const root = document.documentElement;
  if (a === "system") delete root.dataset.theme;
  else root.dataset.theme = a;
}

export function savedAppearance(): Appearance {
  try {
    return (localStorage.getItem("hearth.appearance") as Appearance) || "system";
  } catch {
    return "system";
  }
}

function General() {
  const [appearance, setAppearance] = useState<Appearance>(savedAppearance);
  const [login, setLogin] = useState(true);
  const set = (a: Appearance) => {
    setAppearance(a);
    applyAppearance(a);
    try {
      localStorage.setItem("hearth.appearance", a);
    } catch {
      /* preview without storage */
    }
  };
  return (
    <>
      <div className="setting">
        <span className="setting-text">Appearance</span>
        <div className="segmented" role="radiogroup" aria-label="Appearance">
          {(["system", "light", "dark"] as const).map((a) => (
            <button key={a} role="radio" aria-checked={appearance === a} onClick={() => set(a)}>
              {{ system: "Match system", light: "Light", dark: "Dark" }[a]}
            </button>
          ))}
        </div>
      </div>
      <div className="setting">
        <span className="setting-text">
          <span>Quick ask</span>
          <span className="caption">Opens Hearth from the {platform === "macos" ? "menu bar" : "system tray"} in any app.</span>
        </span>
        <kbd>{platform === "macos" ? "⌥Space" : "Alt+Space"}</kbd>
      </div>
      <Switch label="Open at login" checked={login} onChange={setLogin} />
    </>
  );
}

function Models() {
  const [models, setModels] = useState<RecommendedModel[]>(recommendedModels);
  useEffect(() => {
    if (inTauri) invoke<RecommendedModel[]>("recommended_models").then(setModels).catch(() => {});
  }, []);
  const local = models.filter((m) => m.is_local);
  const cloud = models.filter((m) => !m.is_local);
  return (
    <>
      <p className="muted">Recommended by the 21-model tuning run. Local models run on hearth-box at a 300 W cap.</p>
      <h3 className="settings-group">On hearth-box</h3>
      {local.map((m) => (
        <div key={m.model.model} className="setting">
          <span className="setting-text">
            <span>{m.name}</span>
            <span className="caption">{m.use_for}</span>
          </span>
          <span className="caption">{m.tokens_per_second} tokens/s</span>
        </div>
      ))}
      <h3 className="settings-group">Cloud</h3>
      {cloud.map((m) => (
        <div key={m.model.model} className="setting">
          <span className="setting-text">
            <span>{m.name}</span>
            <span className="caption">{m.use_for}</span>
          </span>
          <button className="btn secondary small">Add key</button>
        </div>
      ))}
    </>
  );
}

function Escalation() {
  const [auto, setAuto] = useState(false);
  return (
    <>
      <Switch
        label="Escalate stalled threads automatically"
        detail="Off: a stalled thread pauses and offers Retry with a stronger model, with the cost shown first."
        checked={auto}
        onChange={setAuto}
      />
      <div className="setting">
        <span className="setting-text">
          <span>Stronger model</span>
          <span className="caption">Cheaper than local per completed task when a failure costs you time.</span>
        </span>
        <button className="chip">Claude Haiku 5.5</button>
      </div>
      <div className="setting">
        <span className="setting-text">
          <span>Counts as stalled</span>
          <span className="caption">2 malformed tool calls, the same call 3 times, or 3 tool errors in a row.</span>
        </span>
      </div>
      <div className="setting">
        <span className="setting-text">
          <span>Monthly cap</span>
          <span className="caption">Hearth asks before cloud spend goes past this.</span>
        </span>
        <span>$10</span>
      </div>
    </>
  );
}

function Files() {
  const reveal = platform === "macos" ? "Show in Finder" : "Show in File Explorer";
  const rows = [
    ["Projects", platform === "windows" ? "%USERPROFILE%\\Hearth\\Projects" : "~/Hearth/Projects"],
    ["App data", platform === "windows" ? "%APPDATA%\\Hearth" : "~/Library/Application Support/Hearth"],
  ];
  return (
    <>
      {rows.map(([label, path]) => (
        <div key={label} className="setting">
          <span className="setting-text">
            <span>{label}</span>
            <code className="caption selectable">{path}</code>
          </span>
          <button className="btn quiet small">{reveal}</button>
        </div>
      ))}
    </>
  );
}
