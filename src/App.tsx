import { useCallback, useEffect, useState } from "react";
import { onCommand, openSettings, platform } from "./native/platform";
import { threads } from "./data/sample";
import { Sidebar } from "./ui/Sidebar";
import { ArtifactsPane, ThreadList, ThreadView } from "./ui/Threads";
import { Overview, Placeholder, Settings, applyAppearance, savedAppearance } from "./ui/Pages";
import "./App.css";

export type View =
  | { kind: "overview" }
  | { kind: "search" }
  | { kind: "settings" }
  | { kind: "chat"; id: string }
  | { kind: "project"; project: string; page: "coordinator" | "threads" | "ideas" | "goals" | "library" | "settings" };

type Layout = {
  sidebar: "full" | "rail";
  list: boolean;
  artifacts: boolean;
  collapsed: Record<string, boolean>;
};

const defaultLayout: Layout = { sidebar: "full", list: true, artifacts: true, collapsed: { "project:hearth": true, "project:bike": true } };

// Collapse state is remembered per window (docs/UI.md).
function useLayout(): [Layout, (f: (l: Layout) => Layout) => void] {
  const [layout, setLayout] = useState<Layout>(() => {
    try {
      return { ...defaultLayout, ...JSON.parse(localStorage.getItem("hearth.layout") ?? "{}") };
    } catch {
      return defaultLayout;
    }
  });
  const update = useCallback((f: (l: Layout) => Layout) => {
    setLayout((l) => {
      const next = f(l);
      try {
        localStorage.setItem("hearth.layout", JSON.stringify(next));
      } catch {
        /* preview without storage */
      }
      return next;
    });
  }, []);
  return [layout, update];
}

export default function App() {
  useEffect(() => applyAppearance(savedAppearance()), []);
  // The macOS Settings window loads this same bundle at #/settings.
  if (location.hash === "#/settings") return <Settings layout="toolbar" />;
  return <MainWindow />;
}

function MainWindow() {
  const [view, setView] = useState<View>({ kind: "project", project: "llm", page: "threads" });
  const [threadId, setThreadId] = useState("t1");
  const [layout, setLayout] = useLayout();

  const showSettings = useCallback(() => openSettings(() => setView({ kind: "settings" })), []);

  useEffect(
    () =>
      onCommand((c) => {
        if (c === "settings") showSettings();
        if (c === "toggle_sidebar") setLayout((l) => ({ ...l, sidebar: l.sidebar === "full" ? "rail" : "full" }));
        if (c === "toggle_thread_list") setLayout((l) => ({ ...l, list: !l.list }));
        if (c === "toggle_artifacts") setLayout((l) => ({ ...l, artifacts: !l.artifacts }));
        if (c === "overview") setView({ kind: "overview" });
      }),
    [showSettings, setLayout],
  );

  const needsYou = threads.filter((t) => t.status === "needs_you").length;
  const inThreads = view.kind === "project" && view.page === "threads";
  const projectThreads = inThreads ? threads.filter((t) => t.project === view.project) : [];
  const thread = projectThreads.find((t) => t.id === threadId) ?? projectThreads[0];
  const files = thread?.feed.flatMap((f) => (f.kind === "files" ? f.files : [])) ?? [];

  const openThread = (id: string) => {
    const t = threads.find((x) => x.id === id);
    if (t) {
      setView({ kind: "project", project: t.project, page: "threads" });
      setThreadId(id);
    }
  };

  let main: React.ReactNode;
  if (view.kind === "overview") main = <Overview open={openThread} />;
  else if (view.kind === "settings") main = <Settings layout={platform === "macos" ? "toolbar" : "page"} />;
  else if (inThreads && thread) main = <ThreadView thread={thread} artifactsOpen={layout.artifacts} toggleArtifacts={() => setLayout((l) => ({ ...l, artifacts: !l.artifacts }))} />;
  else if (view.kind === "search") main = <Placeholder title="Search" />;
  else if (view.kind === "chat") main = <Placeholder title="Chat" />;
  else main = <Placeholder title={{ coordinator: "Coordinator", threads: "Threads", ideas: "Ideas", goals: "Goals", library: "Library", settings: "Project settings" }[view.page]} />;

  return (
    <div
      className="window"
      data-sidebar={layout.sidebar}
      data-list={inThreads && layout.list ? "open" : "closed"}
      data-artifacts={inThreads && layout.artifacts ? "open" : "closed"}
    >
      <Sidebar
        view={view}
        go={setView}
        mode={layout.sidebar}
        toggleMode={() => setLayout((l) => ({ ...l, sidebar: l.sidebar === "full" ? "rail" : "full" }))}
        needsYou={needsYou}
        collapsed={layout.collapsed}
        toggle={(k) => setLayout((l) => ({ ...l, collapsed: { ...l.collapsed, [k]: !l.collapsed[k] } }))}
        openSettings={showSettings}
      />
      <div className="content">
        {inThreads && layout.list && <ThreadList threads={projectThreads} selected={thread?.id} select={setThreadId} />}
        <main className="main">{main}</main>
        {inThreads && layout.artifacts && thread && <ArtifactsPane key={thread.id} files={files} />}
      </div>
    </div>
  );
}
