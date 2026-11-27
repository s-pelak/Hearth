import { useState } from "react";
import {
  ArrowUp, Check, ChevronRight, Copy, FileText, MoreHorizontal, PanelRight, Paperclip, Plus, RotateCcw, Wrench,
} from "lucide-react";
import type { FeedItem, Thread } from "../data/sample";
import { modelName } from "../data/models";
import { ContextMeter, StatusTag } from "./bits";
import { platform, shortcut } from "../native/platform";

export function ThreadList({ threads, selected, select }: { threads: Thread[]; selected?: string; select: (id: string) => void }) {
  return (
    <section className="thread-list" aria-label="Threads">
      <header className="pane-head" data-tauri-drag-region>
        <h2 className="pane-title">Threads</h2>
        <button className="icon-btn" aria-label="New thread" title={`New thread (${shortcut({ mod: true, key: "n" })})`}>
          <Plus size={16} />
        </button>
      </header>
      <ul role="listbox" aria-label="Threads in this project">
        {threads.map((t) => (
          <li key={t.id} role="option" aria-selected={t.id === selected} tabIndex={0} onClick={() => select(t.id)} onKeyDown={(e) => e.key === "Enter" && select(t.id)}>
            <span className="thread-title">{t.title}</span>
            <span className="thread-meta">
              <StatusTag status={t.status} />
              <span className="caption">{t.updated}</span>
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}

export function ThreadView({ thread, artifactsOpen, toggleArtifacts }: { thread: Thread; artifactsOpen: boolean; toggleArtifacts: () => void }) {
  const [menu, setMenu] = useState(false);
  // One flame per screen: when the thread waits on an approval or a stall,
  // that decision is the Copper action and Send steps back.
  const pendingDecision = thread.feed.some((f) => f.kind === "approval" || f.kind === "stall");

  return (
    <section className="thread" aria-label={thread.title}>
      <header className="thread-head" data-tauri-drag-region>
        <div className="thread-head-text">
          <h1 className="title-m">{thread.title}</h1>
          <div className="thread-facts">
            <StatusTag status={thread.status} />
            <span className="caption">{modelName(thread.model)}</span>
            <ContextMeter used={thread.context[0]} window={thread.context[1]} />
          </div>
        </div>
        <div className="toolbar">
          <div className="menu-anchor">
            <button className="icon-btn" aria-label="Thread actions" aria-expanded={menu} onClick={() => setMenu(!menu)}>
              <MoreHorizontal size={16} />
            </button>
            {menu && <ThreadMenu close={() => setMenu(false)} />}
          </div>
          <button className="icon-btn" aria-label="Artifacts" aria-pressed={artifactsOpen} onClick={toggleArtifacts} title={`Artifacts (${shortcut(platform === "macos" ? { alt: true, mod: true, key: "a" } : { ctrl: true, alt: true, key: "a" })})`}>
            <PanelRight size={16} />
          </button>
        </div>
      </header>

      <div className="feed selectable">
        <div className="column">
          {thread.feed.length === 0 ? (
            <p className="muted empty-feed">Nothing in this thread yet.</p>
          ) : (
            thread.feed.map((item, i) => <Feed key={i} item={item} />)
          )}
        </div>
      </div>

      <Composer model={thread.model} primary={!pendingDecision} usage={thread.usage} />
    </section>
  );
}

function ThreadMenu({ close }: { close: () => void }) {
  const items: Array<[string, string?] | null> = [
    ["Mark done", shortcut({ shift: true, mod: true, key: "↩" })],
    ["Remind me in 1 day"],
    ["Remind me in 1 week"],
    null,
    ["Rename"],
    ["Move to project…"],
    ["Switch model…", shortcut({ mod: true, key: "m" })],
    ["Export transcript"],
    null,
    ["Archive"],
  ];
  return (
    <div className="menu" role="menu" onMouseLeave={close}>
      {items.map((it, i) =>
        it ? (
          <button key={i} role="menuitem" onClick={close}>
            <span>{it[0]}</span>
            {it[1] && <kbd>{it[1]}</kbd>}
          </button>
        ) : (
          <hr key={i} />
        ),
      )}
    </div>
  );
}

function Feed({ item }: { item: FeedItem }) {
  switch (item.kind) {
    case "user":
      return <div className="msg-user">{item.text}</div>;
    case "answer":
      return (
        <article className="answer">
          <div className="caption">{modelName(item.model)}</div>
          <p>{item.text}</p>
          <div className="answer-actions">
            <button className="btn quiet small"><Copy size={13} aria-hidden />Copy</button>
            <button className="btn quiet small"><RotateCcw size={13} aria-hidden />Retry</button>
          </div>
        </article>
      );
    case "checklist": {
      const done = item.steps.filter((s) => s.state === "done").length;
      return (
        <section className="card checklist" aria-label="Progress">
          <div className="card-head caption">
            <span>Progress</span>
            <span>{done} of {item.steps.length} done</span>
          </div>
          <ol>
            {item.steps.map((s, i) => (
              <li key={i} className={s.state}>
                <span className="mark" aria-label={{ done: "Done", doing: "In progress", todo: "Not started" }[s.state]}>
                  {{ done: "✓", doing: "✱", todo: "○" }[s.state]}
                </span>
                <span>{s.text}</span>
                {s.helpers && <span className="caption helpers">{s.helpers} helpers</span>}
              </li>
            ))}
          </ol>
        </section>
      );
    }
    case "tools": {
      const failed = item.calls.filter((c) => c.failed).length;
      return (
        <details className="card tools" open={failed > 0}>
          <summary>
            <ChevronRight size={13} className="chevron" aria-hidden />
            <Wrench size={13} aria-hidden />
            <span>{item.calls.length} tool calls</span>
            {failed > 0 && <span className="risk-text">{failed} failed</span>}
          </summary>
          {item.calls.map((c, i) => (
            <div key={i} className={`tool${c.failed ? " failed" : ""}`}>
              <code>{c.name}({c.arg})</code>
              <span className="caption">{c.failed ? "Couldn't parse the arguments" : c.result}</span>
            </div>
          ))}
        </details>
      );
    }
    case "approval":
      return (
        <section className="card decision" aria-label="Approval needed">
          <h3 className="decision-title">Allow {item.tool}?</h3>
          <code className="caption">{item.detail}</code>
          <div className="actions">
            <button className="btn primary">Allow once</button>
            <button className="btn secondary">Always allow {item.domain}</button>
            <button className="btn quiet">Deny</button>
          </div>
        </section>
      );
    case "stall":
      return (
        <section className="caution" aria-label="Caution">
          <p><strong>Caution:</strong> {item.model} {item.reason}, so the thread paused.</p>
          <div className="actions">
            <button className="btn primary">Retry with {item.retryWith}</button>
            <span className="caption">{item.cost}</span>
            <button className="btn quiet">Let it try again</button>
            <button className="btn quiet">Edit and continue</button>
          </div>
        </section>
      );
    case "summarized":
      return (
        <div className="summarized caption">
          <span>{item.turns} earlier turns were summarized to fit the context window.</span>
          <button className="btn quiet small">Show</button>
        </div>
      );
    case "review":
      return (
        <section className="card decision" aria-label="Ready for review">
          <h3 className="decision-title">Ready for review</h3>
          <div className="actions">
            <button className="btn secondary"><Check size={14} aria-hidden />Mark done</button>
            <button className="btn quiet">Ask for changes</button>
          </div>
        </section>
      );
    case "files":
      return (
        <div className="file-chips">
          {item.files.map((f, i) => (
            <button key={f.name} className="source-chip" title={`Open ${f.name}`}>
              <span className="n">{i + 1}</span>
              <FileText size={13} aria-hidden />
              {f.name}
              <span className="caption">{f.detail}</span>
            </button>
          ))}
        </div>
      );
  }
}

function Composer({ model, primary, usage }: { model: string; primary: boolean; usage: string }) {
  const [text, setText] = useState("");
  return (
    <form className="composer" onSubmit={(e) => e.preventDefault()}>
      <div className="composer-field">
        <button type="button" className="icon-btn" aria-label="Attach files">
          <Paperclip size={16} />
        </button>
        <textarea
          rows={1}
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder="Reply, or type @ to mention a file"
          aria-label="Message"
        />
        <button type="submit" className={`send ${primary ? "primary" : "secondary"}`} aria-label="Send" title={`Send (${platform === "macos" ? "⌘↩" : "Ctrl+Enter"})`}>
          <ArrowUp size={16} />
        </button>
      </div>
      <div className="composer-meta caption">
        <button type="button" className="chip">{modelName(model)}</button>
        <span>4 tools</span>
        <span>{usage}</span>
      </div>
    </form>
  );
}

export function ArtifactsPane({ files }: { files: Array<{ name: string; detail: string }> }) {
  const [tab, setTab] = useState(0);
  if (files.length === 0)
    return (
      <aside className="artifacts" aria-label="Artifacts">
        <header className="pane-head"><h2 className="pane-title">Artifacts</h2></header>
        <p className="muted pad">Files this thread makes show up here.</p>
      </aside>
    );
  const f = files[Math.min(tab, files.length - 1)];
  return (
    <aside className="artifacts" aria-label="Artifacts">
      <header className="pane-head"><h2 className="pane-title">Artifacts</h2></header>
      <div className="tabs" role="tablist">
        {files.map((x, i) => (
          <button key={x.name} role="tab" aria-selected={i === tab} onClick={() => setTab(i)}>{x.name}</button>
        ))}
      </div>
      <div className="artifact-info caption">
        <span>{f.detail}</span>
        <span>From this thread</span>
      </div>
      <div className="artifact-preview selectable">
        <pre>{previewFor(f.name)}</pre>
      </div>
    </aside>
  );
}

function previewFor(name: string): string {
  if (name.endsWith(".json")) return `{\n  "C1": 1.00,\n  "C2": 0.98,\n  "C3": 0.97,\n  "C4": 0.81\n}`;
  if (name.endsWith(".md")) return "# 21-model run\n\nKeep three local models at a 300 W cap.\n\n| Model | Use |\n|---|---|\n| Gemma 4 12B | Everyday text |\n| Qwen3.6 35B-A3B | Code and CAD |\n| Nemotron 3.5 Bolt 74B | Agents |";
  return `${name}\nBinary file. Open it in ${platform === "macos" ? "Finder" : "File Explorer"} to view.`;
}
