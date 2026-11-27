import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type Provider = { id: string; name: string; kind: string; base_url: string; is_local: boolean };
type KeptModel = { id: string; name: string; strength: string; roles: string[]; served_as: string | null };

const STRENGTH: Record<string, string> = {
  everyday_text: "Everyday text",
  code_and_cad: "Code and CAD",
  agents_and_long_work: "Agents and long work",
};

// Phase 1 placeholder: proves the shell talks to hearth-core.
// The real UI follows docs/UI.md and docs/mockup/index.html.
export default function App() {
  const [providers, setProviders] = useState<Provider[]>([]);
  const [kept, setKept] = useState<KeptModel[] | string>("checking…");

  useEffect(() => {
    invoke<Provider[]>("list_providers").then(setProviders);
    invoke<KeptModel[]>("kept_models")
      .then(setKept)
      .catch((e) => setKept(`LLM box offline: ${String(e)}`));
  }, []);

  return (
    <main className="shell">
      <aside className="sidebar" data-tauri-drag-region>
        <h1>Hearth</h1>
      </aside>
      <section className="content">
        <h2>Local models</h2>
        {typeof kept === "string" ? (
          <small>{kept}</small>
        ) : (
          <ul>
            {kept.map((m) => (
              <li key={m.id}>
                <span>
                  {m.name}
                  <br />
                  <small>
                    {STRENGTH[m.strength]}
                    {m.roles.length > 0 && ` · default for ${m.roles.join(", ")}`}
                  </small>
                </span>
                <small>{m.served_as ? "On the box" : "Not on the box"}</small>
              </li>
            ))}
          </ul>
        )}
        <h2 className="gap">Providers</h2>
        <ul>
          {providers.map((p) => (
            <li key={p.id}>
              <span>{p.name}</span>
              <small>{p.base_url}</small>
            </li>
          ))}
        </ul>
      </section>
    </main>
  );
}
