import { useMemo } from "react";
import { useAria2 } from "../context/Aria2Provider";
import type { Download, FilterKey } from "../types";
import DownloadItem from "./DownloadItem";
import { DownloadIcon, SearchIcon } from "../lib/icons";

interface Props {
  filter: FilterKey;
  query: string;
  onDelete: (d: Download) => void;
}

export default function DownloadList({ filter, query, onDelete }: Props) {
  const { snapshot } = useAria2();

  const downloads = useMemo(() => {
    const q = query.trim().toLowerCase();
    return snapshot.downloads
      .filter((d) => {
        if (filter === "all") return true;
        if (filter === "active") return d.status === "active";
        return d.status === filter;
      })
      .filter((d) => {
        if (!q) return true;
        return (
          d.filename.toLowerCase().includes(q) ||
          d.uri.toLowerCase().includes(q)
        );
      })
      .sort((a, b) => b.created_at - a.created_at);
  }, [snapshot.downloads, filter, query]);

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
        <DownloadItem key={d.gid} download={d} onDelete={onDelete} />
      ))}
    </div>
  );
}
