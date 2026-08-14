# Bits Download Manager

A fast, modern download manager for Windows built with [Tauri](https://tauri.app),
React and [aria2](https://aria2.github.io/) as the download engine.

Multi-connection downloads, pause/resume, session persistence and a clean
dark/light UI — all in one lightweight desktop app.

## Features

- **Multi-connection downloads** — splits downloads into up to 64 parallel
  connections via aria2 for maximum speed.
- **Pause / resume / retry** — pause and resume individual downloads or all at
  once; retry failed downloads with one click.
- **Real-time progress** — live speed, ETA, size and progress bars pushed to the
  UI via Tauri events (1s updates).
- **Session persistence** — active and paused downloads survive app restarts
  (aria2 session file), and completed/failed history is stored in
  `history.json`.
- **Download controls** — remove from list (keep files) or delete file + control
  file from disk, with confirmation.
- **Search & filters** — filter by status (all/active/waiting/paused/completed/
  failed) and search by name or URL.
- **Custom destination** — pick a folder with the native dialog, set a filename,
  and tune the number of connections per download.
- **Dark/light theme** — adapts to your system preference.

## Architecture

```
src/                         React + TypeScript frontend (Vite)
  context/Aria2Provider.tsx  subscribes to downloads://update events, exposes actions
  components/                Sidebar, Toolbar, DownloadList, modals
  lib/                       formatting helpers + SVG icon set
src-tauri/                   Rust backend (Tauri)
  src/aria2.rs               JSON-RPC client + aria2c process lifecycle
  src/sync.rs                background poller → snapshot + event emission
  src/history.rs             durable download history (history.json)
  src/model.rs               Download / GlobalStat / Snapshot types
  src/lib.rs                 Tauri commands + app entry
  bin/windows/aria2c.exe     bundled aria2 binary (also shipped as a resource)
```

The backend talks to `aria2c` over its JSON-RPC interface (`http://127.0.0.1:<port>/jsonrpc`)
using a randomly generated secret token. A background loop polls aria2 once per
second, merges active/waiting/stopped downloads with persisted history, and
emits the resulting snapshot to the frontend.

## Development

Prerequisites: [Rust](https://www.rust-lang.org/), [Bun](https://bun.sh/),
and the [Tauri prerequisites](https://tauri.app/start/prerequisites/).

```bash
bun install          # frontend dependencies
bun run tauri dev    # start the app in dev mode (vite + cargo)
```

Run backend tests:

```bash
cd src-tauri
cargo test           # spins up a real aria2c + local HTTP server
```

Build a production installer:

```bash
bun run tauri build
```

## Release builds (GitHub Actions)

A workflow (`.github/workflows/build.yml`) builds the app on `windows-latest`
whenever a `v*.*.*` tag is pushed (also triggerable manually from the Actions
tab). It produces both installers and attaches them to the tag's GitHub Release:

- `src-tauri/target/release/bundle/nsis/*-setup.exe`
- `src-tauri/target/release/bundle/msi/*.msi`

The bundled `aria2c.exe` is shipped as a Tauri resource in the installers and
copied to the app data dir on first run.

## How downloads work

- Downloads default to your system `Downloads` folder (per-download override
  available in the "New download" dialog).
- Partial downloads are resumable thanks to aria2 control files (`.aria2`) and
  `--continue=true`.
- When the app exits, the aria2 session is saved and the engine shuts down
  gracefully; the next launch reuses the session.
- Completed/failed downloads are kept in history so your list survives restarts.

## Project layout notes

- The bundled `aria2c.exe` lives in `src-tauri/bin/windows/` and is copied to the
  app data dir at first run (also bundled as a Tauri resource for installers).
- App data (session, history, RPC config, cached binary) lives in
  `%APPDATA%\bits-download-manager`.
