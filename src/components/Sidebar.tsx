import { useAria2 } from "../context/Aria2Provider";
import type { Download, FilterKey } from "../types";
import { formatSpeed } from "../lib/format";
import { ArrowDownIcon, ArrowUpIcon } from "../lib/icons";
import BandwidthGraph from "./BandwidthGraph";

const ITEMS: { key: FilterKey; label: string }[] = [
  { key: "all", label: "All downloads" },
  { key: "active", label: "Active" },
  { key: "waiting", label: "Waiting" },
  { key: "paused", label: "Paused" },
  { key: "complete", label: "Completed" },
  { key: "error", label: "Failed" },
];

function countOf(downloads: Download[], key: FilterKey): number {
  if (key === "all") return downloads.length;
  return downloads.filter((d) => d.status === key).length;
}

interface Props {
  filter: FilterKey;
  onFilter: (key: FilterKey) => void;
}

export default function Sidebar({ filter, onFilter }: Props) {
  const { snapshot, connected } = useAria2();
  const { global, aria2_version } = snapshot;

  return (
    <aside className="sidebar">
      <div className="brand">
        <div className="brand-icon brand-icon-logo">01</div>
        <div className="brand-text">
          <span className="brand-name">Bits</span>
          <span className="brand-sub">Download Manager</span>
        </div>
      </div>

      <nav className="nav">
        {ITEMS.map((item) => (
          <button
            key={item.key}
            className={`nav-item ${filter === item.key ? "active" : ""}`}
            onClick={() => onFilter(item.key)}
          >
            <span className="nav-label">{item.label}</span>
            <span className="nav-count">{countOf(snapshot.downloads, item.key)}</span>
          </button>
        ))}
      </nav>

      <div className="sidebar-footer">
        {!connected && <div className="engine-offline">aria2 offline</div>}
        <BandwidthGraph
          downloadSpeed={global.download_speed}
          uploadSpeed={global.upload_speed}
        />
        <div className="transfer">
          <span className="transfer-row">
            <ArrowDownIcon width={14} height={14} />
            {formatSpeed(global.download_speed)}
          </span>
          <span className="transfer-row">
            <ArrowUpIcon width={14} height={14} />
            {formatSpeed(global.upload_speed)}
          </span>
        </div>
        {aria2_version && (
          <div className="engine-version">aria2 v{aria2_version}</div>
        )}
      </div>
    </aside>
  );
}
