import { useAria2 } from "../context/Aria2Provider";
import { useToast } from "./Toasts";
import { useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { SortKey } from "../types";
import {
  EraserIcon,
  GearIcon,
  PauseIcon,
  PlayIcon,
  PlusIcon,
  SearchIcon,
  TorrentIcon,
} from "../lib/icons";

interface Props {
  query: string;
  onQuery: (q: string) => void;
  onNew: () => void;
  onSettings: () => void;
  sort: SortKey;
  onSort: (s: SortKey) => void;
}

const SORT_OPTIONS: { key: SortKey; label: string }[] = [
  { key: "newest", label: "Newest" },
  { key: "name", label: "Name" },
  { key: "size", label: "Size" },
  { key: "speed", label: "Speed" },
  { key: "progress", label: "Progress" },
];

export default function Toolbar({
  query,
  onQuery,
  onNew,
  onSettings,
  sort,
  onSort,
}: Props) {
  const { snapshot, pauseAll, resumeAll, clearFinished, connected, remove, addDownload } =
    useAria2();
  const { push } = useToast();

  const addTorrent = async () => {
    const path = await open({
      filters: [{ name: "Torrent", extensions: ["torrent"] }],
      title: "Choose a .torrent file",
      multiple: false,
    });
    if (typeof path !== "string") return;
    try {
      await invoke("add_torrent", { path, dir: null });
      push("Torrent added", "success");
    } catch (e) {
      push(`Failed to add torrent: ${e}`, "error");
    }
  };

  const hasActive = useMemo(
    () => snapshot.downloads.some((d) => d.status === "active"),
    [snapshot.downloads],
  );
  const hasFinished = useMemo(
    () =>
      snapshot.downloads.some(
        (d) => d.status === "complete" || d.status === "error",
      ),
    [snapshot.downloads],
  );
  const hasFailed = useMemo(
    () => snapshot.downloads.some((d) => d.status === "error"),
    [snapshot.downloads],
  );

  const retryFailed = () => {
    const failed = snapshot.downloads.filter((d) => d.status === "error");
    let ok = 0;
    for (const d of failed) {
      void remove(d.gid);
      void addDownload({
        uri: d.uri,
        dir: d.dir || undefined,
        out: d.filename || undefined,
        split: 16,
      })
        .then(() => ok++)
        .catch(() => {});
    }
    push(`Retrying ${failed.length} failed download(s)`, "info");
  };

  return (
    <header className="toolbar">
      <button className="btn btn-primary" onClick={onNew}>
        <PlusIcon width={16} height={16} />
        New download
      </button>

      <button className="btn" onClick={addTorrent} disabled={!connected} title="Add .torrent file">
        <TorrentIcon width={15} height={15} />
        Torrent
      </button>

      <div className="search">
        <SearchIcon width={15} height={15} />
        <input
          value={query}
          onChange={(e) => onQuery(e.target.value)}
          placeholder="Search downloads…"
          spellCheck={false}
        />
      </div>

      <select
        className="sort-select"
        value={sort}
        onChange={(e) => onSort(e.target.value as SortKey)}
        title="Sort by"
      >
        {SORT_OPTIONS.map((o) => (
          <option key={o.key} value={o.key}>
            {o.label}
          </option>
        ))}
      </select>

      <div className="toolbar-actions">
        <button
          className="btn"
          onClick={resumeAll}
          disabled={!connected}
          title="Resume all"
        >
          <PlayIcon width={15} height={15} />
        </button>
        <button
          className="btn"
          onClick={pauseAll}
          disabled={!connected || !hasActive}
          title="Pause all"
        >
          <PauseIcon width={15} height={15} />
        </button>
        <button
          className="btn"
          onClick={retryFailed}
          disabled={!connected || !hasFailed}
          title="Retry failed downloads"
        >
          Retry
        </button>
        <button
          className="btn"
          onClick={clearFinished}
          disabled={!connected || !hasFinished}
          title="Clear finished"
        >
          <EraserIcon width={15} height={15} />
        </button>
        <button className="btn" onClick={onSettings} title="Settings">
          <GearIcon width={15} height={15} />
        </button>
      </div>
    </header>
  );
}
