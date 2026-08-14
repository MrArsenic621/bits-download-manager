import { useAria2, type AddDownloadOptions } from "../context/Aria2Provider";
import { openPath } from "@tauri-apps/plugin-opener";
import type { Download } from "../types";
import { formatBytes, formatEta, formatSpeed, hostOf } from "../lib/format";
import {
  AlertIcon,
  CheckCircleIcon,
  DownloadIcon,
  FolderIcon,
  PauseIcon,
  PlayIcon,
  TrashIcon,
  XCircleIcon,
} from "../lib/icons";

const STATUS_LABEL: Record<string, string> = {
  active: "Downloading",
  waiting: "Waiting",
  paused: "Paused",
  complete: "Completed",
  error: "Failed",
  removed: "Removed",
};

const FALLBACK_OPTIONS: Partial<AddDownloadOptions> = { split: 16 };

const SPEED_LIMITS: { label: string; value: number }[] = [
  { label: "∞", value: 0 },
  { label: "100 KB/s", value: 100 * 1024 },
  { label: "500 KB/s", value: 500 * 1024 },
  { label: "1 MB/s", value: 1024 * 1024 },
  { label: "5 MB/s", value: 5 * 1024 * 1024 },
];

interface Props {
  download: Download;
  onDelete: (d: Download) => void;
}

export default function DownloadItem({ download: d, onDelete }: Props) {
  const { pause, resume, remove, addDownload, setSpeedLimit } = useAria2();

  const statusLabel = STATUS_LABEL[d.status] ?? d.status;
  const isActive = d.status === "active";
  const isWaiting = d.status === "waiting";
  const isPaused = d.status === "paused";
  const isComplete = d.status === "complete";
  const isError = d.status === "error";

  const statusIcon = () => {
    if (isComplete) return <CheckCircleIcon width={20} height={20} />;
    if (isError) return <AlertIcon width={20} height={20} />;
    if (isPaused) return <PauseIcon width={20} height={20} />;
    return <DownloadIcon width={20} height={20} />;
  };

  const openFolder = () => {
    if (d.dir) void openPath(d.dir);
  };

  const retry = () => {
    void remove(d.gid);
    void addDownload({
      uri: d.uri,
      dir: d.dir || undefined,
      out: d.filename || undefined,
      ...FALLBACK_OPTIONS,
    });
  };

  const primaryAction = () => {
    if (isActive || isWaiting) {
      return (
        <button
          className="icon-btn"
          onClick={() => void pause(d.gid)}
          title="Pause"
        >
          <PauseIcon width={16} height={16} />
        </button>
      );
    }
    if (isPaused) {
      return (
        <button
          className="icon-btn"
          onClick={() => void resume(d.gid)}
          title="Resume"
        >
          <PlayIcon width={16} height={16} />
        </button>
      );
    }
    if (isComplete) {
      return (
        <button className="icon-btn" onClick={openFolder} title="Open folder">
          <FolderIcon width={16} height={16} />
        </button>
      );
    }
    if (isError) {
      return (
        <button
          className="icon-btn"
          onClick={retry}
          title="Retry download"
        >
          <PlayIcon width={16} height={16} />
        </button>
      );
    }
    return null;
  };

  const meta = [];
  if (isActive) {
    meta.push(
      <span key="spd" className="meta-speed">
        <DownloadIcon width={12} height={12} /> {formatSpeed(d.download_speed)}
      </span>,
    );
    if (d.eta_secs != null) {
      meta.push(<span key="eta">{formatEta(d.eta_secs)} left</span>);
    }
  }
  meta.push(
    <span key="size">
      {d.total_length > 0
        ? `${formatBytes(d.completed_length)} / ${formatBytes(d.total_length)}`
        : formatBytes(d.completed_length)}
    </span>,
  );

  const subtitle = d.error_message
    ? `Error: ${d.error_message}`
    : d.uri || hostOf(d.uri);

  const speedSelect = isActive ? (
    <select
      className="speed-limit"
      defaultValue="0"
      onChange={(e) => void setSpeedLimit(d.gid, Number(e.target.value))}
      title="Speed limit"
    >
      {SPEED_LIMITS.map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  ) : null;

  return (
    <article className={`download-item status-${d.status}`}>
      <div className="item-icon">{statusIcon()}</div>

      <div className="item-main">
        <div className="item-title-row">
          <span className="item-name" title={d.filename}>
            {d.filename || "(unknown)"}
          </span>
          <span className={`badge badge-${d.status}`}>{statusLabel}</span>
          {isActive && (
            <span className="item-percent">{Math.floor(d.progress)}%</span>
          )}
        </div>

        {d.status !== "complete" && (
          <div className="item-progress">
            <div
              className="progress-fill"
              style={{ width: `${Math.min(100, Math.max(0, d.progress))}%` }}
            />
          </div>
        )}

        <div className="item-meta">
          <span className="item-subtitle" title={d.uri}>
            {subtitle}
          </span>
          {meta.map((m, i) => (
            <span key={i} className="meta">
              {m}
            </span>
          ))}
          {speedSelect}
        </div>
      </div>

      <div className="item-actions">
        {primaryAction()}
        {!isComplete && (
          <button
            className="icon-btn"
            onClick={() => void remove(d.gid)}
            title="Remove from list"
          >
            <XCircleIcon width={16} height={16} />
          </button>
        )}
        <button
          className="icon-btn danger"
          onClick={() => onDelete(d)}
          title="Delete file"
        >
          <TrashIcon width={16} height={16} />
        </button>
      </div>
    </article>
  );
}
