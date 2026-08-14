import { useMemo } from "react";
import { useAria2 } from "../context/Aria2Provider";
import type { Download, FilterKey, SortKey } from "../types";
import DownloadItem from "./DownloadItem";
import { DownloadIcon, SearchIcon } from "../lib/icons";

interface Props {
  filter: FilterKey;
  query: string;
  sort: SortKey;
  selected: Set<string>;
  onToggleSelect: (gid: string) => void;
  onDelete: (d: Download) => void;
}

export default function DownloadList({
  filter,
  query,
  sort,
  selected,
  onToggleSelect,
  onDelete,
}: Props) {
  const { snapshot } = useAria2();

  const downloads = useMemo(() => {
    const q = query.trim().toLowerCase();
    const filtered = snapshot.downloads.filter((d) => {
      if (filter === "all") return true;
      return d.status === filter;
    });
    const matched = filtered.filter((d) => {
      if (!q) return true;
      return (
        d.filename.toLowerCase().includes(q) ||
        d.uri.toLowerCase().includes(q)
      );
    });

    const sorted = [...matched];
    switch (sort) {
      case "name":
        sorted.sort((a, b) => a.filename.localeCompare(b.filename));
        break;
      case "size":
        sorted.sort((a, b) => b.total_length - a.total_length);
        break;
      case "speed":
        sorted.sort((a, b) => b.download_speed - a.download_speed);
        break;
      case "progress":
        sorted.sort((a, b) => b.progress - a.progress);
        break;
      default:
        sorted.sort((a, b) => b.created_at - a.created_at);
        break;
    }
    return sorted;
  }, [snapshot.downloads, filter, query, sort]);

  if (snapshot.downloads.length === 0) {
    return (
      <div className="empty-state">
        <div className="empty-icon">
          <DownloadIcon width={40} height={40} />
        </div>
        <h2>No downloads yet</h2>
        <p>Paste a link and start downloading at full speed.</p>
      </div>
    );
  }

  if (downloads.length === 0) {
    return (
      <div className="empty-state">
        <div className="empty-icon">
          <SearchIcon width={40} height={40} />
        </div>
        <h2>Nothing matches</h2>
        <p>Try a different search or filter.</p>
      </div>
    );
  }

  return (
    <div className="download-list">
      {downloads.map((d) => (
        <DownloadItem
          key={d.gid}
          download={d}
          selected={selected.has(d.gid)}
          onToggleSelect={() => onToggleSelect(d.gid)}
          onDelete={onDelete}
        />
      ))}
    </div>
  );
}
