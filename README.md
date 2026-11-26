# Hearth

A local-first agent harness that lives in your Mac's menu bar or your Windows tray. Projects, parallel threads, a coordinator, ideas, goals, artifacts and plugins, running on your own models (llama.cpp / LM Studio over Tailscale) and on cloud models (Anthropic, OpenAI, Gemini) when you choose.

- Plan: [docs/PLAN.md](docs/PLAN.md)
- UI and design requirements: [docs/UI.md](docs/UI.md)
- Clickable mockup: [docs/mockup/index.html](docs/mockup/index.html) (open it in a browser)

## Status

Early scaffold. The Rust core (providers, agent loop, tools, escalation, storage) builds and is tested; the app shell runs in the menu bar with a placeholder window.

## Develop

Requirements: Rust (stable), Node 20+, and on Linux the Tauri system libraries (`libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev`).

```sh
npm install
npm run tauri dev           # run the app
cargo test -p hearth-core   # core tests, no GUI needed
```

## Install a build

Tagged releases (`v*`) build a universal macOS `.dmg` and a Windows installer in GitHub Actions. The Mac build is ad-hoc signed, so the first time you open it, right-click Hearth and choose Open.

## License

Apache-2.0
