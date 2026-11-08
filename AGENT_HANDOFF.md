# Agent Handoff — Bits Download Manager

> **Read this first.** This document is the source of truth for what has been built,
> how the system works, what is broken, and what comes next. It is written for an
> AI coding agent picking up this repository with no prior conversation context.

---

## 1. Project at a Glance

**Bits Download Manager** is a Windows desktop download manager built with:

| Layer | Technology |
|---|---|
| Frontend | React 19 + TypeScript + Vite |
| Backend | Rust 2021 + Tauri v2 |
| Package manager | `bun` |
| Primary engine | Embedded `aria2c.exe` controlled over JSON-RPC |
| Stream engine | Embedded `yt-dlp.exe` (requires Node.js at runtime) |
| Browser integration | Manifest V3 extension + local TCP bridge (`127.0.0.1:6801`) |

**Current version:** `3.1.8`
**Latest tag:** `v3.1.8`
**Branch:** `main`

---

## 2. Repository Map

```
.
├── AGENTS.md                  # Repo conventions (Windows rules, verify cmds, git workflow)
├── AGENT_HANDOFF.md           # ← this file
├── README.md                  # User-facing docs & prerequisites
├── package.json               # Frontend deps + version (keep in sync)
├── extension/                 # MV3 browser companion (Chrome/Edge/Firefox)
│   ├── manifest.json, background.js, popup.*, icons/
├── .github/workflows/build.yml
├── src/                       # React frontend
│   ├── App.tsx                # Shell, clipboard watcher, drag/drop, modals
│   ├── App.css                # All styling (single stylesheet)
│   ├── types.ts               # Shared TS types (Settings, StreamFormat, ...)
│   ├── context/Aria2Provider.tsx
│   ├── components/
│   │   ├── NewDownloadModal.tsx   # ★ URL entry + stream detection + format picker
│   │   ├── SettingsModal.tsx      # General / Scheduler / Extension(vault) tabs
│   │   ├── DownloadItem.tsx, DownloadList.tsx, Toolbar.tsx, Sidebar.tsx
│   │   ├── BandwidthGraph.tsx, ChecksumModal.tsx, ConfirmDialog.tsx, Toasts.tsx
│   └── lib/ (categories.ts, format.ts, icons.tsx)
└── src-tauri/
    ├── Cargo.toml             # version must match package.json + tauri.conf.json
    ├── tauri.conf.json        # version + bundled resources
    ├── bin/windows/
    │   ├── aria2c.exe         # committed binary
    │   └── yt-dlp.exe         # committed binary (v2026.08.19 at time of writing)
    └── src/
        ├── main.rs            # #![windows_subsystem = "windows"]
        ├── lib.rs             # ★ App state, all Tauri commands, tray, setup
        ├── aria2.rs           # aria2 process spawn + JSON-RPC client + binary resolver
        ├── model.rs           # Domain types (Download, Settings, StreamFormat, ...)
        ├── settings.rs, history.rs, vault.rs
        ├── sync.rs            # Polls aria2, updates snapshot, persists history
        └── bridge.rs          # Local TCP bridge server for the browser extension
```

---

## 3. Hard Rules (from `AGENTS.md`)

- `src-tauri/src/main.rs` **must** keep `#![windows_subsystem = "windows"]` unconditionally.
- **Every** spawned child process on Windows must use `CREATE_NO_WINDOW`:
  `cmd.creation_flags(0x08000000)` (via `std::os::windows::process::CommandExt`).
- Batch downloads: per-download `out` filename override must **not** be applied to
  multi-URL batches (only when `lines.length === 1`).
- Version lives in **three** places and must stay in sync:
  `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`.
- Conventional commits: `feat:`, `fix:`, `refactor:`, `chore:`.
- After committing, create a semver tag (`vX.Y.Z`) and push both branch and tags.

### Verification commands (run before every commit)

```powershell
bun run build
cargo check --manifest-path src-tauri/Cargo.toml
cargo test  --manifest-path src-tauri/Cargo.toml   # spins up mock server + aria2
```

> PowerShell note: cargo writes progress to stderr, so piping it through
> `Select-String`/`Select-Object` may show a spurious non-zero exit code. Check for
> the literal `error` lines and `Finished` instead of relying on `$?`.

---

## 4. Completed Roadmap (staged plan → delivered)

| Version | Stage | What shipped |
|---|---|---|
| `v1.0.1` | Silent processes | Hidden console, `CREATE_NO_WINDOW` on all children |
| `v1.1.0` | Batch clipboard | Multi-URL/magnet extraction + newline-separated batch modal |
| `v1.2.0` | Bandwidth graph | Live 30s SVG sparkline in sidebar |
| `v1.3.0` | Auto-categorize | Extension-based subfolders + badges |
| `v1.4.0` | Checksums | SHA-256 / SHA-1 / MD5 verifier modal |
| `v1.5.0` | HTTP headers & auth | Referer, User-Agent, Cookie, Basic auth per download |
| `v1.6.0` | Scheduler & power | Off-peak window + shutdown/sleep/hibernate on finish |
| `v1.7.0` | Tray & background | Tray menu, close-to-tray persistence |
| `v2.0.0–2.0.2` | Browser extension | MV3 companion + local bridge on `127.0.0.1:6801` |
| `v2.1.0` | Packaging | Extension settings tab, icons, release zip, publisher metadata |
| `v2.2.0` | Drag & drop | Floating drop-zone for URLs/text |
| `v2.3.0` | Speed profiles | `⚡ Max Speed`, `🎮 Gaming/Stream`, `🌱 Eco Saver` |
| `v2.4.0` | Item details | Expandable per-file stats panel |
| `v2.5.0` | Credentials vault | Domain → auth/cookie vault, auto-injected |
| `v3.0.0` | Stream grabber | `yt-dlp` metadata fetch + format list |
| `v3.1.0` | Proxy | Global SOCKS5/HTTP proxy for aria2 + yt-dlp |
| `v3.1.1–3.1.6` | Fixes | Modal scroll/padding, platform detection (Twitch/Vimeo/etc.), cookie auth, yt-dlp path resolution, direct-URL handoff |
| `v3.1.7` | **Stream engine rewrite** | Downloads routed through `yt-dlp` (HLS/DASH stitching + muxing); Node.js runtime requirement |
| `v3.1.8` | Dependency repair | Reverted dependabot lockfile breakage (see §7) |

---

## 5. How Streaming Works Today (critical to understand)

### 5.1 Why this is not a normal download

`aria2c` can only fetch a single direct resource. A Twitch VOD or YouTube video is
**not** a single resource:

- Twitch VODs are HLS (`.m3u8`) playlists containing thousands of `.ts` segments.
- YouTube ≥1080p is DASH: separate video and audio streams that must be muxed.

Feeding those manifest URLs to `aria2c` saves the tiny playlist text, not the video.
**Therefore stream URLs must be downloaded by `yt-dlp` itself.**

### 5.2 The two backend commands

Both live in `src-tauri/src/lib.rs`.

#### `fetch_stream_metadata(url, cookie) -> StreamMetadata`
Runs `yt-dlp --dump-json` and returns `{ title, formats: StreamFormat[] }`.
Each `StreamFormat` now includes the direct media `url` in addition to
`format_id`, `ext`, `resolution`, `note`.

#### `download_stream(url, format_id, dir, out, cookie) -> String`
Spawns `yt-dlp` in a background thread to perform the actual download:

```text
yt-dlp --js-runtimes node --no-warnings -o "<dir>/%(title)s.%(ext)s" [-f <format_id>] [cookie args] <url>
```

- `--js-runtimes node` is **required** for YouTube's player challenge solving.
- Cookie argument resolution (shared logic pattern used by both commands):
  - `chrome` / `firefox` / `edge` → `--cookies-from-browser <name>`
  - an existing file path → `--cookies <path>`
  - anything else → written to `<app_data>/bin/temp_cookies.txt`, then `--cookies <that>`
- Honors output dir, auto-categorization, custom filename.
- ⚠️ **Proxy gap:** `fetch_stream_metadata` passes `--proxy <global_proxy>`, but
  `download_stream` currently does **not**. Add the same `--proxy` argument there
  (copy from the metadata command) before relying on proxies for actual stream downloads.

> ⚠️ **Known limitation:** `download_stream` currently fires-and-forgets. It does not
> appear in the aria2 snapshot/UI download list, has no progress bar, no pause/resume,
> and no completion toast. This is the biggest remaining UX gap — see §8.

### 5.3 Frontend flow (`NewDownloadModal.tsx`)

1. `isStreamingUrl()` matches: YouTube, Twitch, Vimeo, SoundCloud, Dailymotion,
   TikTok, Twitter/X, Instagram.
2. On a single matching URL, `useEffect` calls `fetch_stream_metadata` automatically
   (re-runs when the cookie field changes, so users can retry with cookies).
3. A format `<select>` appears; the chosen `format_id` is stored in `selectedFormat`.
4. On submit:
   - **If metadata exists** → `invoke("download_stream", { url, formatId, dir, out, cookie })`.
   - **Otherwise** → normal `addDownload()` into aria2 (direct files, magnets, torrents).

### 5.4 Node.js requirement

- `yt-dlp` YouTube extraction is effectively broken without a JS runtime.
- On startup, `lib.rs` runs `node --version`. If missing, it sets `startup_error`,
  which the UI already displays as a banner. The message links to <https://nodejs.org>.
- Documented in `README.md` → Prerequisites.

### 5.5 yt-dlp binary resolution

`resolve_ytdlp_path()` mirrors `aria2.rs`'s resolver. It checks, in order:

1. `<app_data>/bin/yt-dlp.exe` (copied cache)
2. `CARGO_MANIFEST_DIR/bin/windows/yt-dlp.exe` (dev/source build)
3. Tauri `resource_dir()/bin/windows/yt-dlp.exe`, then `resource_dir()/yt-dlp.exe`
4. `current_exe()` directory

First hit is copied into `<app_data>/bin/` and returned.

---

## 6. Verified Working Recipe (manual, outside the app)

If stream behavior is ever in doubt, test the binary directly first:

```powershell
# Metadata (Twitch — no auth needed)
.\src-tauri\bin\windows\yt-dlp.exe --dump-json "https://www.twitch.tv/videos/2902855488"

# YouTube — needs a JS runtime AND cookies
.\src-tauri\bin\windows\yt-dlp.exe --js-runtimes node --cookies-from-browser firefox --dump-json "https://www.youtube.com/watch?v=4tGRW6ww434"

# Actual download (what download_stream does)
.\src-tauri\bin\windows\yt-dlp.exe --js-runtimes node --cookies-from-browser firefox -o "%(title)s.%(ext)s" "https://www.youtube.com/watch?v=4tGRW6ww434"
```

Observed during testing:
- Twitch metadata works anonymously.
- YouTube returns *"Sign in to confirm you're not a bot"* unless cookies + Node runtime are supplied.
- Exported cookie files can go stale ("cookies are no longer valid / rotated"); `--cookies-from-browser` is more reliable than a static file.
- `--cookies-from-browser chrome` failed on this machine (no Chrome profile); `edge` failed DPAPI decryption; `firefox` worked.

---

## 7. Dependency Landmine (do not repeat)

Auto-merged Dependabot PRs **broke `main`** before `v3.1.8`:

1. `rand` was bumped `0.8 → 0.10`, but `aria2.rs` uses the 0.8 API
   (`rand::thread_rng()`, `rand::distributions::Alphanumeric`) → compile error.
2. The lockfile pulled `tauri-runtime 2.12.x` / `tauri-runtime-wry 2.12.x` against
   `tauri 2.11.1` → 27 errors inside the `tauri` crate itself.

**Fix applied in `v3.1.8`:** restored the last known-good `Cargo.lock` and pinned
`rand = "0.8"`.

**Outstanding:** GitHub still reports 3 Dependabot alerts (1 moderate, 2 low). These
must be upgraded *together and compatibly* — do not blindly merge Dependabot PRs.
A safe upgrade requires bumping `tauri` + `tauri-build` + plugins as a set and
migrating `aria2.rs` to the rand 0.10 API (`rand::rng()`, `rand::distr::Alphanumeric`)
if moving rand forward.

---

## 8. Next Planned Stage — `v3.2.0`: Cross-Platform Dynamic Engine Downloader

**Not started.** Goal: stop shipping binaries in git; fetch them on first run.

Planned design:

1. **Target detection** in Rust: map `target_os` + `target_arch` to the correct
   upstream asset (`yt-dlp.exe` / `yt-dlp_macos` / `yt-dlp`, `aria2c.exe` / `aria2c`).
2. **First-run flow**: if `<app_data>/bin/*` is missing, download from GitHub Releases
   with progress events to the frontend (dedicated setup modal), then
   `chmod +x` on Unix.
3. **Update checker**: compare `yt-dlp --version` against the latest GitHub release
   tag; offer/perform updates (yt-dlp breaks often when YouTube changes).
4. **Remove** `src-tauri/bin/windows/*.exe` from the repo and from
   `tauri.conf.json` bundle resources once downloading is reliable.
5. **Node.js**: keep as a documented runtime dependency, or bundle a JS runtime
   (e.g. Deno sidecar) to remove the external requirement.

### Higher-priority UX gap (recommended before/with `v3.2.0`)

Make `download_stream` a **first-class download item**:

- Register a synthetic entry in the snapshot/UI (title, progress, speed, ETA).
- Parse `yt-dlp --newline --progress-template` output to drive progress.
- Support cancel/pause (kill the child process; `yt-dlp` supports `--continue`).
- Emit completion notification/toast and respect `shutdown_on_finish`.
- Reconcile the finished file path so "Open folder" / "Delete" work.

---

## 9. Quick Sanity Checklist for a New Agent

1. `git log --oneline -5` and `git tag -l | tail` — confirm you're at `v3.1.8`+.
2. `bun install` if `node_modules` is stale.
3. `bun run build` → must pass (`tsc && vite build`).
4. `cargo check --manifest-path src-tauri/Cargo.toml` → must pass.
5. Confirm `src-tauri/bin/windows/{aria2c.exe,yt-dlp.exe}` exist.
6. Confirm `node --version` works (stream grabber needs it).
7. Before touching deps: re-read §7. Before touching streaming: re-read §5.

---

## 10. Environment Notes

- Platform: Windows (`win32`), shell is PowerShell.
- Prefer `C:\Users\asus\AppData\Local\Temp\opencode` for scratch files.
- Repo: `github.com:MrArsenic621/bits-download-manager.git`
- `src-tauri/Cargo.lock` **is** tracked — commit it whenever it changes.
