import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { onCommand, openSettings, platform } from "./native/platform";
import "./App.css";

type Provider = { id: string; name: string; kind: string; base_url: string; is_local: boolean };
type Page = "models" | "settings";
type SidebarMode = "full" | "rail";

// Phase 1 placeholder inside the native window frame. The visual design waits
// for the brand style guide; this proves the shell, materials, menu commands
// and the per-platform Settings behavior.
export default function App() {
  // The macOS Settings window loads this same bundle at #/settings.
  if (location.hash === "#/settings") return <SettingsWindow />;
  return <MainWindow />;
}

function MainWindow() {
  const [page, setPage] = useState<Page>("models");
  const [sidebar, setSidebar] = useState<SidebarMode>("full");
  const showSettings = () => openSettings(() => setPage("settings"));

  useEffect(
    () =>
      onCommand((c) => {
        if (c === "settings") showSettings();
        if (c === "toggle_sidebar") setSidebar((s) => (s === "full" ? "rail" : "full"));
      }),
    [],
  );

  return (
    <div className="window" data-sidebar={sidebar}>
      <nav className="sidebar" aria-label="Sidebar" data-tauri-drag-region>
        <button className="row" aria-current={page === "models" ? "page" : undefined} onClick={() => setPage("models")}>
          <span className="label">Models</span>
        </button>
        <div className="spacer" />
        <button
          className="row"
          aria-current={page === "settings" ? "page" : undefined}
          onClick={showSettings}
          title={platform === "macos" ? "Settings (⌘,)" : "Settings (Ctrl+,)"}
        >
          <span className="label">Settings</span>
        </button>
      </nav>
      <main className="content">
        <header className="titlebar" data-tauri-drag-region>
          <h1 className="title">{page === "settings" ? "Settings" : "Models"}</h1>
        </header>
        <div className="page">{page === "settings" ? <SettingsBody /> : <Models />}</div>
      </main>
    </div>
  );
}

function Models() {
  const [providers, setProviders] = useState<Provider[]>([]);
  const [models, setModels] = useState<Record<string, string>>({});

  useEffect(() => {
    invoke<Provider[]>("list_providers")
      .then(async (ps) => {
        setProviders(ps);
        for (const p of ps.filter((p) => p.is_local)) {
          try {
            const ms = await invoke<string[]>("list_models", { providerId: p.id });
            setModels((m) => ({ ...m, [p.id]: ms.join(", ") || "No models loaded" }));
          } catch (e) {
            setModels((m) => ({ ...m, [p.id]: `Offline: ${String(e)}` }));
          }
        }
      })
      .catch(() => setProviders([]));
  }, []);

  if (providers.length === 0) return <p className="secondary">Run Hearth as the desktop app to see your providers.</p>;
  return (
    <ul className="list">
      {providers.map((p) => (
        <li key={p.id}>
          <span>{p.name}</span>
          <span className="caption selectable">{p.is_local ? models[p.id] ?? "Checking…" : p.base_url}</span>
        </li>
      ))}
    </ul>
  );
}

function SettingsWindow() {
  return (
    <div className="settings-window">
      <SettingsBody />
    </div>
  );
}

function SettingsBody() {
  return (
    <p className="secondary">
      Escalation, Models, File locations, Connectors and skills, and General will live here.
    </p>
  );
}
