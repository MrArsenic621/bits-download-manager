import { useEffect, useMemo, useRef, useState } from "react";
import { readText } from "@tauri-apps/plugin-clipboard-manager";
import { useAria2 } from "./context/Aria2Provider";
import type { Download, FilterKey, SortKey } from "./types";
import Sidebar from "./components/Sidebar";
import Toolbar from "./components/Toolbar";
import DownloadList from "./components/DownloadList";
import NewDownloadModal from "./components/NewDownloadModal";
import ConfirmDialog from "./components/ConfirmDialog";
import SettingsModal from "./components/SettingsModal";
import { PauseIcon, PlayIcon, CloseIcon, TrashIcon } from "./lib/icons";

const URL_RE =
  /(https?:\/\/[^\s<>"']+|magnet:\?[^\s<>"']+)/i;

function extractUrl(text: string): string | null {
  const m = text.match(URL_RE);
  if (!m) return null;
  return m[1].replace(/[.,;:!?]+$/, "");
}

export default function App() {
  const { del, remove, pause, resume, snapshot, settings } = useAria2();
  const [filter, setFilter] = useState<FilterKey>("all");
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<SortKey>("newest");
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [newOpen, setNewOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [clipboardUrl, setClipboardUrl] = useState<string | null>(null);
  const lastDetected = useRef<string>("");
  const [confirm, setConfirm] = useState<{
    title: string;
    message: React.ReactNode;
    confirmLabel?: string;
    onConfirm: () => void;
  } | null>(null);

  // Drop selections that no longer exist.
  const liveGids = useMemo(
    () => new Set(snapshot.downloads.map((d) => d.gid)),
    [snapshot.downloads],
  );
  useEffect(() => {
    setSelected((prev) => {
      const next = new Set([...prev].filter((g) => liveGids.has(g)));
      return next.size === prev.size ? prev : next;
    });
  }, [liveGids]);

  // Keyboard shortcuts: Ctrl+N new download, Ctrl+F search, Esc close/clear.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const mod = e.ctrlKey || e.metaKey;
      if (mod && e.key.toLowerCase() === "n") {
        e.preventDefault();
        setNewOpen(true);
      } else if (mod && e.key.toLowerCase() === "f") {
        e.preventDefault();
        const el = document.getElementById("download-search") as HTMLInputElement | null;
        el?.focus();
        el?.select();
      } else if (e.key === "Escape") {
        setConfirm(null);
        setNewOpen(false);
        setSettingsOpen(false);
        setSelected(new Set());
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // Clipboard watcher: suggest adding copied URLs / magnet links.
  useEffect(() => {
    if (!settings?.watch_clipboard) return;
    const tick = async () => {
      if (!document.hasFocus() || clipboardUrl) return;
      try {
        const text = await readText();
        const url = extractUrl(text);
        if (url && url !== lastDetected.current) {
          lastDetected.current = url;
          setClipboardUrl(url);
        }
      } catch {
        // clipboard unreadable; ignore
      }
    };
    const id = setInterval(tick, 2000);
    return () => clearInterval(id);
  }, [settings?.watch_clipboard, clipboardUrl]);

  const toggleSelect = (gid: string) =>
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(gid)) next.delete(gid);
      else next.add(gid);
      return next;
    });

  const askDelete = (d: Download) =>
    setConfirm({
      title: "Delete download",
      message: (
        <>
          Delete <strong>{d.filename || "(unknown)"}</strong> and its files from
          disk? This cannot be undone.
        </>
      ),
      confirmLabel: "Delete file",
      onConfirm: () => {
        void del(d.gid);
        setConfirm(null);
      },
    });

  const bulk = (action: "pause" | "resume" | "remove") => {
    for (const gid of selected) {
      if (action === "pause") void pause(gid);
      else if (action === "resume") void resume(gid);
      else void remove(gid);
    }
    setSelected(new Set());
  };

  const bulkDelete = () =>
    setConfirm({
      title: "Delete downloads",
      message: `Delete ${selected.size} download(s) and their files from disk? This cannot be undone.`,
      confirmLabel: `Delete ${selected.size}`,
      onConfirm: () => {
        for (const gid of selected) void del(gid);
        setSelected(new Set());
        setConfirm(null);
      },
    });

  const selectedCount = selected.size;

  return (
    <div className="app">
      <Sidebar filter={filter} onFilter={setFilter} />

      <main className="main">
        {snapshot.startup_error && (
          <div className="error-banner">
            Download engine failed to start: {snapshot.startup_error}
          </div>
        )}
        <Toolbar
          query={query}
          onQuery={setQuery}
          onNew={() => setNewOpen(true)}
          onSettings={() => setSettingsOpen(true)}
          sort={sort}
          onSort={setSort}
        />
        <div className="content">
          {selectedCount > 0 && (
            <div className="bulk-bar">
              <span className="bulk-count">
                {selectedCount} selected
              </span>
              <div className="bulk-actions">
                <button className="btn btn-sm" onClick={() => bulk("resume")} title="Resume selected">
                  <PlayIcon width={14} height={14} /> Resume
                </button>
                <button className="btn btn-sm" onClick={() => bulk("pause")} title="Pause selected">
                  <PauseIcon width={14} height={14} /> Pause
                </button>
                <button className="btn btn-sm" onClick={() => bulk("remove")} title="Remove selected from list">
                  Remove
                </button>
                <button className="btn btn-sm btn-danger" onClick={bulkDelete} title="Delete selected files">
                  <TrashIcon width={14} height={14} /> Delete
                </button>
                <button className="icon-btn" onClick={() => setSelected(new Set())} title="Clear selection">
                  <CloseIcon width={15} height={15} />
                </button>
              </div>
            </div>
          )}
          <DownloadList
            filter={filter}
            query={query}
            sort={sort}
            selected={selected}
            onToggleSelect={toggleSelect}
            onDelete={askDelete}
          />
        </div>
      </main>

      {newOpen && (
        <NewDownloadModal
          initialUri={clipboardUrl ?? undefined}
          onClose={() => {
            setNewOpen(false);
            setClipboardUrl(null);
          }}
        />
      )}

      {settingsOpen && <SettingsModal onClose={() => setSettingsOpen(false)} />}

      {confirm && (
        <ConfirmDialog
          title={confirm.title}
          message={confirm.message}
          confirmLabel={confirm.confirmLabel}
          onConfirm={confirm.onConfirm}
          onCancel={() => setConfirm(null)}
        />
      )}

      {clipboardUrl && !newOpen && (
        <div className="clipboard-chip">
          <div className="chip-text">
            <span className="chip-title">Link detected</span>
            <span className="chip-url" title={clipboardUrl}>
              {clipboardUrl}
            </span>
          </div>
          <div className="chip-actions">
            <button className="btn btn-sm btn-primary" onClick={() => setNewOpen(true)}>
              Add
            </button>
            <button
              className="icon-btn"
              onClick={() => setClipboardUrl(null)}
              title="Dismiss"
            >
              <CloseIcon width={15} height={15} />
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
