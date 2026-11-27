// Sample content for the UI until threads are wired to hearth-core.
// Thread names come from the Local LLM Machine tuning run.

export type Status = "working" | "needs_you" | "review" | "done" | "failed";

export type FeedItem =
  | { kind: "user"; text: string }
  | { kind: "answer"; model: string; text: string }
  | { kind: "checklist"; steps: Array<{ text: string; state: "done" | "doing" | "todo"; helpers?: number }> }
  | { kind: "tools"; calls: Array<{ name: string; arg: string; result: string; failed?: boolean }> }
  | { kind: "approval"; tool: string; detail: string; domain: string }
  | { kind: "stall"; model: string; reason: string; retryWith: string; cost: string }
  | { kind: "summarized"; turns: number }
  | { kind: "review" }
  | { kind: "files"; files: Array<{ name: string; detail: string }> };

export type Thread = {
  id: string;
  project: string;
  title: string;
  status: Status;
  model: string;
  updated: string;
  context: [number, number]; // thousands of tokens used, window
  usage: string;
  feed: FeedItem[];
};

export type Project = { id: string; name: string; threads: number };

export const projects: Project[] = [
  { id: "llm", name: "Local LLM Machine", threads: 5 },
  { id: "hearth", name: "Hearth app", threads: 2 },
  { id: "bike", name: "Bike Buddy mount", threads: 1 },
];

export const chats = [
  { id: "c1", title: "Power cap for the 3080 Ti" },
  { id: "c2", title: "DTE time-of-day rates" },
];

export const statusLabel: Record<Status, string> = {
  working: "Working",
  needs_you: "Needs you",
  review: "Ready for review",
  done: "Done",
  failed: "Failed",
};

export const threads: Thread[] = [
  {
    id: "t1",
    project: "llm",
    title: "Grade the CAD items for Qwen3.6",
    status: "needs_you",
    model: "qwen3.6-35b-a3b",
    updated: "2 min ago",
    context: [27, 32],
    usage: "41k tokens, $0.00",
    feed: [
      { kind: "user", text: "Compile the four CAD answers in OpenSCAD and compare each with its reference part by volume, bounding box and intersection." },
      {
        kind: "checklist",
        steps: [
          { text: "Compile C1 to C4", state: "done" },
          { text: "Compare with the reference parts", state: "doing", helpers: 2 },
          { text: "Fix the hole offset on C4", state: "todo" },
          { text: "Write the scores to results.json", state: "todo" },
        ],
      },
      {
        kind: "tools",
        calls: [
          { name: "openscad.render", arg: "C1.scad", result: "ok, 2.4 s" },
          { name: "openscad.render", arg: "C4.scad", result: "ok, 3.1 s" },
          { name: "mesh.compare", arg: "C4.stl, ref_c4.stl", result: "{\"volume\": 0.97, \"bbox\": 1.0, \"iou\": 0.81" , failed: true },
          { name: "mesh.compare", arg: "C4.stl, ref_c4.stl", result: "{\"volume\": 0.97, \"bbox\": 1.0, \"iou\": 0.81", failed: true },
        ],
      },
      { kind: "answer", model: "qwen3.6-35b-a3b", text: "C1 to C3 match their references. C4 is within 3% on volume, but the mounting hole sits 1.5 mm off center, so intersection is 0.81 and the item scores below the 0.8 pass line on two of three checks." },
      { kind: "stall", model: "Qwen3.6 35B-A3B", reason: "made 2 malformed tool calls in a row", retryWith: "Claude Haiku 5.5", cost: "about $0.02" },
      { kind: "files", files: [{ name: "C4.stl", detail: "version 3" }, { name: "results.json", detail: "draft" }] },
    ],
  },
  {
    id: "t2",
    project: "llm",
    title: "Rerun fable-coder-35B",
    status: "needs_you",
    model: "nemotron-3.5-bolt-74b",
    updated: "8 min ago",
    context: [12, 128],
    usage: "18k tokens, $0.00",
    feed: [
      { kind: "user", text: "fable-coder stopped after 22 items on a file lock. Rerun the rest and retry on file errors this time." },
      {
        kind: "checklist",
        steps: [
          { text: "Add retry on file errors to the harness", state: "done" },
          { text: "Rerun items 23 to 52", state: "doing" },
          { text: "Add the scores to the summary", state: "todo" },
        ],
      },
      { kind: "approval", tool: "web.fetch", detail: "https://github.com/ggml-org/llama.cpp/releases", domain: "github.com" },
    ],
  },
  {
    id: "t3",
    project: "llm",
    title: "Summarize the 21-model results",
    status: "review",
    model: "nemotron-3.5-bolt-74b",
    updated: "1 h ago",
    context: [96, 128],
    usage: "212k tokens, $0.00",
    feed: [
      { kind: "user", text: "Write a one-page summary of the run for the README." },
      { kind: "summarized", turns: 14 },
      { kind: "answer", model: "nemotron-3.5-bolt-74b", text: "Keep three local models at a 300 W cap: Gemma 4 12B for everyday text, Qwen3.6 35B-A3B for code and CAD, and Nemotron 3.5 Bolt 74B for agents and long documents. Where a failure costs time, Claude Haiku 5.5 is cheaper per completed task." },
      { kind: "files", files: [{ name: "SUMMARY.md", detail: "version 2" }] },
      { kind: "review" },
    ],
  },
  {
    id: "t4",
    project: "llm",
    title: "Abort on looping output",
    status: "working",
    model: "qwen3.6-35b-a3b",
    updated: "now",
    context: [9, 32],
    usage: "11k tokens, $0.00",
    feed: [
      { kind: "user", text: "Make the harness stop a model that repeats the same 200 characters three times." },
      { kind: "checklist", steps: [{ text: "Find where output is streamed", state: "done" }, { text: "Add the repeat check", state: "doing" }, { text: "Test on the Inkling transcripts", state: "todo" }] },
    ],
  },
  {
    id: "t5",
    project: "llm",
    title: "Fix the curly apostrophe grader bug",
    status: "done",
    model: "gemma-4-12b",
    updated: "Yesterday",
    context: [6, 32],
    usage: "7k tokens, $0.00",
    feed: [{ kind: "user", text: "Seven correct refusals with curly apostrophes were marked as failures. Normalize quotes before matching." }, { kind: "answer", model: "gemma-4-12b", text: "Fixed. The grader now folds ’ and ‘ to ' before matching, and the seven refusals pass." }],
  },
  { id: "t6", project: "hearth", title: "Apply the design system", status: "working", model: "qwen3.6-35b-a3b", updated: "now", context: [4, 32], usage: "3k tokens, $0.00", feed: [] },
  { id: "t7", project: "hearth", title: "Keychain for API keys", status: "failed", model: "gemma-4-12b", updated: "3 h ago", context: [30, 32], usage: "29k tokens, $0.00", feed: [] },
  { id: "t8", project: "bike", title: "Grade C4 bracket STL", status: "review", model: "qwen3.6-35b-a3b", updated: "2 d ago", context: [14, 32], usage: "15k tokens, $0.00", feed: [] },
];
