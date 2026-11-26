import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type Provider = { id: string; name: string; kind: string; base_url: string; is_local: boolean };

// Phase 1 placeholder: proves the shell talks to hearth-core.
// The real UI follows docs/UI.md and docs/mockup/index.html.
export default function App() {
  const [providers, setProviders] = useState<Provider[]>([]);
  const [models, setModels] = useState<Record<string, string>>({});

  useEffect(() => {
    invoke<Provider[]>("list_providers").then(async (ps) => {
      setProviders(ps);
      for (const p of ps.filter((p) => p.is_local)) {
        try {
          const ms = await invoke<string[]>("list_models", { providerId: p.id });
          setModels((m) => ({ ...m, [p.id]: ms.join(", ") || "no models loaded" }));
        } catch (e) {
          setModels((m) => ({ ...m, [p.id]: `offline: ${String(e)}` }));
        }
      }
    });
  }, []);

  return (
    <main className="shell">
      <aside className="sidebar" data-tauri-drag-region>
        <h1>Hearth</h1>
      </aside>
      <section className="content">
        <h2>Models</h2>
        <ul>
          {providers.map((p) => (
            <li key={p.id}>
              <span>{p.name}</span>
              <small>{p.is_local ? models[p.id] ?? "checking…" : p.base_url}</small>
            </li>
          ))}
        </ul>
      </section>
    </main>
  );
}
