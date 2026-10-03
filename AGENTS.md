# Agent Guidelines for Bits Download Manager

This repository is **Bits Download Manager**, a Windows download manager desktop application built with Tauri v2, React 19, TypeScript, Vite, Bun, and an embedded aria2 daemon.

---

## 1. Core Architecture & Windows Process Rules

- **Tech Stack**:
  - Frontend: React 19 + TypeScript + Vite (`src/`)
  - Backend: Rust 2021 + Tauri v2 (`src-tauri/`)
  - Package Manager: `bun`
  - Engine: `aria2c.exe` controlled via JSON-RPC (`src-tauri/src/aria2.rs`)

- **Windows Console & Window Rules**:
  - `src-tauri/src/main.rs` must have `#![windows_subsystem = "windows"]` unconditionally to prevent console window popups.
  - Any child process spawned on Windows (such as `aria2c.exe`) must use `CREATE_NO_WINDOW` creation flag (`0x08000000` via `std::os::windows::process::CommandExt::creation_flags`).

- **Clipboard & Batch Download Rules**:
  - Multi-link detection in clipboard is supported: extract all valid URLs/magnets (`extractUrls`) and trim trailing punctuation.
  - New downloads modal accepts batch URLs separated by newlines (`\n`).
  - When batch downloading multiple URLs, per-download custom filename override (`out`) must not overwrite all batch items with the same name (disable or auto-detect per URL).

---

## 2. Development & Verification Commands

Run verification before committing any changes:

- **Frontend Build & Typecheck**:
  ```powershell
  bun run build
  ```
- **Backend Cargo Check**:
  ```powershell
  cargo check --manifest-path src-tauri/Cargo.toml
  ```
- **Backend Tests** (starts local mock server and aria2 daemon):
  ```powershell
  cargo test --manifest-path src-tauri/Cargo.toml
  ```

---

## 3. Git, Versioning, and Release Workflow

- **One-by-One Execution**:
  - Always implement user requests incrementally, step by step.
  - Verify each step with `bun run build` and `cargo check`.

- **Conventional Commits**:
  - Use conventional commit messages: `feat: ...`, `fix: ...`, `refactor: ...`, `chore: ...`.

- **Semantic Versioning & Git Tags**:
  - After committing, create an annotated or standard git tag matching the semantic version (e.g. `v1.0.1`, `v1.1.0`).
  - Check current tags before bumping: `git tag -l` and `git log --oneline -5`.
