# Bits Download Manager

[![Tauri](https://img.shields.io/badge/Tauri-v2-24C8D8?style=flat-square&logo=tauri&logoColor=white)](https://tauri.app)
[![React](https://img.shields.io/badge/React-19-61DAFB?style=flat-square&logo=react&logoColor=black)](https://react.dev)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.8-3178C6?style=flat-square&logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![Rust](https://img.shields.io/badge/Rust-2021-DEA584?style=flat-square&logo=rust&logoColor=black)](https://www.rust-lang.org/)
[![aria2](https://img.shields.io/badge/Engine-aria2-brightgreen?style=flat-square)](https://aria2.github.io/)

**Bits Download Manager** is a fast, lightweight Windows desktop download manager built with Tauri v2, React 19, TypeScript, and an embedded [aria2](https://aria2.github.io/) daemon.

---

## Features

- **Multi-Connection Acceleration** — Splits files into up to 64 parallel connections via aria2 for maximum throughput.
- **System Tray & Background Mode** — Minimize to Windows system tray with quick action menu (Show/Hide, Pause/Resume All, Quit) and close-to-tray background persistence.
- **Download Scheduler & Auto-Shutdown** — Automatically run queue during scheduled off-peak hours (e.g. 1 AM – 7 AM) and put PC to sleep / shutdown when complete.
- **Advanced HTTP Headers & Auth** — Custom Referer, User-Agent, Cookie headers, and HTTP Basic/Bearer authentication support for protected URLs.
- **Checksum & Hash Integrity Checker** — Built-in SHA-256, SHA-1, and MD5 calculator with one-click copy and instant hash match/mismatch verification.
- **Smart Auto-Categorization** — Organizes downloads into subfolders (`Videos/`, `Documents/`, `Audio/`, `Archives/`, `Programs/`) by file extension with color-coded badges.
- **Live Bandwidth Speed Graph** — Real-time SVG throughput sparkline chart with sliding 30-second window and peak speed detection.
- **Smart Clipboard Link Watcher** — Detects copied URLs and magnet links automatically with single or multi-link batch import.
- **Batch Downloading** — Paste multiple URLs separated by newlines to queue downloads at once.
- **BitTorrent & Magnet Links** — Native support for downloading via `.torrent` files and `magnet:?` URIs.
- **Speed & Bandwidth Controls** — Configure global download speed caps or throttle speed per individual download.
- **Bulk Queue Operations** — Multi-select items to batch resume, pause, remove, or delete files from disk.
- **Pause / Resume / Retry** — Resume interrupted downloads seamlessly; retry failed downloads with one click.
- **Session Persistence & History** — Active queue state persists across app launches via aria2 session files; full download history is stored in local storage.
- **Keyboard Shortcuts**:
  - `Ctrl + N`: Open new download dialog
  - `Ctrl + F`: Quick search / filter downloads
  - `Esc`: Close open modal / clear selection
- **Console-Free Execution** — Zero terminal flash or background console popups on Windows.
- **Dark / Light Theme** — Clean modern UI that follows system preferences.

---

## Architecture

```
bits-download-manager/
├── src/                          # React 19 frontend (Vite + TypeScript)
│   ├── components/               # UI components (DownloadList, Modals, Toolbar, Sidebar, BandwidthGraph, ChecksumModal)
│   ├── context/                  # Aria2Provider managing state and Tauri RPC events
│   ├── lib/                      # SVG icons and format helpers
│   └── App.tsx                   # Main layout, keyboard listeners & clipboard watcher
├── src-tauri/                    # Rust backend (Tauri v2)
│   ├── src/
│   │   ├── aria2.rs              # aria2 daemon process management & JSON-RPC client
│   │   ├── sync.rs               # Background poller pushing snapshots to frontend
│   │   ├── history.rs            # History persistence (history.json)
│   │   ├── settings.rs           # User settings load/save
│   │   ├── model.rs              # Rust data structs & types
│   │   ├── lib.rs                # Tauri command handlers & plugin setup
│   │   └── main.rs               # App entrypoint (#![windows_subsystem = "windows"])
│   └── bin/windows/aria2c.exe    # Bundled Windows aria2 binary
├── AGENTS.md                     # AI Agent development & workflow guidelines
└── package.json                  # Frontend dependencies and scripts
```

---

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/) (latest stable)
- [Bun](https://bun.sh/) (or Node.js / npm)
- [Tauri v2 Prerequisites for Windows](https://tauri.app/start/prerequisites/)

### Development

1. Install frontend dependencies:
   ```powershell
   bun install
   ```

2. Start the app in development mode:
   ```powershell
   bun run tauri dev
   ```

### Running Backend Tests

Backend tests spin up an embedded mock HTTP server and test aria2 RPC lifecycle methods:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
```

### Building Installers

Create production NSIS (`.exe`) and MSI (`.msi`) installers:

```powershell
bun run tauri build
```

The output installers will be generated under `src-tauri/target/release/bundle/`.

---

## Release Builds

Releases are automatically built with GitHub Actions on `windows-latest` when pushing a version tag (e.g. `v1.1.1`).
The workflow attaches both NSIS setup and MSI installers to the GitHub Release.

---

## License

MIT License.
