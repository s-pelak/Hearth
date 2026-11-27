// Detects the platform and window material, and exposes them to CSS as
// `data-platform` and `data-material` on <html>. Styles in platform.css key
// off these so macOS follows Apple's HIG and Windows follows Fluent.
//
// Outside Tauri (plain `npm run dev` in a browser) the platform comes from the
// user agent, and `?platform=macos|windows` forces one for previews.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type Platform = "macos" | "windows" | "linux";
export type Material = "sidebar" | "mica" | "none";

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

function guessPlatform(): Platform {
  const forced = new URLSearchParams(location.search).get("platform");
  if (forced === "macos" || forced === "windows" || forced === "linux") return forced;
  const ua = navigator.userAgent;
  if (/Mac/i.test(ua)) return "macos";
  if (/Win/i.test(ua)) return "windows";
  return "linux";
}

export let platform: Platform = guessPlatform();
export let material: Material = "none";

/** Run once before first render. */
export async function initPlatform(): Promise<void> {
  if (inTauri) {
    try {
      const chrome = await invoke<{ platform: Platform; material: Material }>("window_chrome");
      platform = chrome.platform;
      material = chrome.material;
    } catch {
      // Keep the user-agent guess and paint opaque.
    }
  }
  const root = document.documentElement;
  root.dataset.platform = platform;
  root.dataset.material = material;

  // macOS dims the sidebar selection and controls when the window is inactive.
  const sync = () => (root.dataset.active = String(document.hasFocus()));
  window.addEventListener("focus", sync);
  window.addEventListener("blur", sync);
  sync();
}

/** Shortcut text the way each platform writes it: ⇧⌘N on macOS, Ctrl+Shift+N on Windows. */
export function shortcut(keys: { mod?: boolean; shift?: boolean; alt?: boolean; ctrl?: boolean; key: string }): string {
  if (platform === "macos") {
    return `${keys.ctrl ? "⌃" : ""}${keys.alt ? "⌥" : ""}${keys.shift ? "⇧" : ""}${keys.mod ? "⌘" : ""}${keys.key.toUpperCase()}`;
  }
  const parts = [keys.mod || keys.ctrl ? "Ctrl" : "", keys.alt ? "Alt" : "", keys.shift ? "Shift" : "", keys.key.toUpperCase()];
  return parts.filter(Boolean).join("+");
}

/**
 * Commands the UI handles. On macOS they arrive from the native menu bar as
 * `menu` events; on Windows and Linux there is no menu bar, so the same ids
 * come from keyboard shortcuts bound here.
 */
export type Command =
  | "settings" | "new_thread" | "new_chat" | "new_project" | "attach" | "find"
  | "toggle_sidebar" | "toggle_thread_list" | "toggle_artifacts" | "overview" | "search"
  | "send" | "stop" | "mark_done" | "retry_stronger" | "switch_model" | "help";

const windowsKeys: Record<string, Command> = {
  "ctrl+,": "settings",
  "ctrl+n": "new_thread",
  "ctrl+shift+n": "new_chat",
  "ctrl+alt+n": "new_project",
  "ctrl+shift+a": "attach",
  "ctrl+f": "find",
  "ctrl+b": "toggle_sidebar",
  "ctrl+alt+l": "toggle_thread_list",
  "ctrl+alt+a": "toggle_artifacts",
  "ctrl+1": "overview",
  "ctrl+k": "search",
  "ctrl+enter": "send",
  "ctrl+shift+enter": "mark_done",
  "ctrl+shift+r": "retry_stronger",
  "ctrl+m": "switch_model",
  f1: "help",
};

export function onCommand(handler: (c: Command) => void): () => void {
  const cleanups: Array<() => void> = [];
  if (inTauri) {
    listen<string>("menu", (e) => handler(e.payload as Command)).then((un) => cleanups.push(un));
  }
  // The macOS menu bar owns its ⌘ shortcuts; everywhere else the UI binds them.
  if (!(inTauri && platform === "macos")) {
    const onKey = (e: KeyboardEvent) => {
      const combo = [e.ctrlKey && "ctrl", e.altKey && "alt", e.shiftKey && "shift", e.key.toLowerCase()].filter(Boolean).join("+");
      const cmd = windowsKeys[combo];
      if (cmd) {
        e.preventDefault();
        handler(cmd);
      }
    };
    window.addEventListener("keydown", onKey);
    cleanups.push(() => window.removeEventListener("keydown", onKey));
  }
  return () => cleanups.forEach((c) => c());
}

/** Settings: its own window on macOS (⌘,), a page in the main window elsewhere. */
export function openSettings(showPage: () => void): void {
  if (inTauri && platform === "macos") invoke("open_settings");
  else showPage();
}
