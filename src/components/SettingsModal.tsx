import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useAria2 } from "../context/Aria2Provider";
import type { Settings } from "../types";
import { CloseIcon, FolderIcon } from "../lib/icons";

const SPEED_LIMITS: { label: string; value: number }[] = [
  { label: "Unlimited", value: 0 },
  { label: "128 KB/s", value: 128 * 1024 },
  { label: "512 KB/s", value: 512 * 1024 },
  { label: "1 MB/s", value: 1024 * 1024 },
  { label: "5 MB/s", value: 5 * 1024 * 1024 },
  { label: "10 MB/s", value: 10 * 1024 * 1024 },
  { label: "50 MB/s", value: 50 * 1024 * 1024 },
];

interface Props {
  onClose: () => void;
}

export default function SettingsModal({ onClose }: Props) {
  const { settings, updateSettings } = useAria2();
  const [form, setForm] = useState<Settings>(() =>
    settings
      ? { ...settings }
      : {
          default_dir: "",
          default_split: 16,
          max_concurrent_downloads: 3,
          global_speed_limit: 0,
          notify_on_complete: true,
          watch_clipboard: true,
          auto_categorize: true,
        },
  );
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const set = <K extends keyof Settings>(key: K, value: Settings[K]) =>
    setForm((f) => ({ ...f, [key]: value }));

  const browse = async () => {
    const dir = await open({ directory: true, title: "Choose default folder" });
    if (typeof dir === "string") set("default_dir", dir);
  };

  const save = async () => {
    setBusy(true);
    setError(null);
    try {
      await updateSettings(form);
      onClose();
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  };

  return (
    <div className="modal-overlay" onMouseDown={onClose}>
      <div className="modal" onMouseDown={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>Settings</h2>
          <button className="icon-btn" onClick={onClose} title="Close">
            <CloseIcon width={16} height={16} />
          </button>
        </div>

        <div className="modal-body">
          <div className="field-row">
            <label className="field grow">
              <span className="field-label">Default download folder</span>
              <input
                value={form.default_dir}
                onChange={(e) => set("default_dir", e.target.value)}
                placeholder="System Downloads folder"
                spellCheck={false}
              />
            </label>
            <button className="btn browse-btn" onClick={browse} title="Browse">
              <FolderIcon width={15} height={15} />
            </button>
          </div>

          <div className="field-row">
            <label className="field">
              <span className="field-label">Connections per download</span>
              <input
                type="number"
                min={1}
                max={64}
                value={form.default_split}
                onChange={(e) =>
                  set("default_split", Math.min(64, Math.max(1, Number(e.target.value) || 16)))
                }
              />
            </label>
            <label className="field">
              <span className="field-label">Max concurrent downloads</span>
              <input
                type="number"
                min={1}
                max={16}
                value={form.max_concurrent_downloads}
                onChange={(e) =>
                  set("max_concurrent_downloads", Math.min(16, Math.max(1, Number(e.target.value) || 3)))
                }
              />
            </label>
          </div>

          <label className="field">
            <span className="field-label">Global download speed limit</span>
            <select
              className="select"
              value={form.global_speed_limit}
              onChange={(e) => set("global_speed_limit", Number(e.target.value))}
            >
              {SPEED_LIMITS.map((o) => (
                <option key={o.value} value={o.value}>
                  {o.label}
                </option>
              ))}
            </select>
          </label>

          <label className="toggle-row">
            <span>
              <span className="toggle-title">Auto-categorize downloads</span>
              <span className="toggle-sub">
                Organize downloads into subfolders (Videos, Documents, Music, Archives, Programs)
              </span>
            </span>
            <input
              type="checkbox"
              checked={form.auto_categorize}
              onChange={(e) => set("auto_categorize", e.target.checked)}
            />
          </label>

          <label className="toggle-row">
            <span>
              <span className="toggle-title">Completion notifications</span>
              <span className="toggle-sub">Notify when a download finishes</span>
            </span>
            <input
              type="checkbox"
              checked={form.notify_on_complete}
              onChange={(e) => set("notify_on_complete", e.target.checked)}
            />
          </label>

          <label className="toggle-row">
            <span>
              <span className="toggle-title">Watch clipboard</span>
              <span className="toggle-sub">
                Suggest adding URLs / magnet links you copy
              </span>
            </span>
            <input
              type="checkbox"
              checked={form.watch_clipboard}
              onChange={(e) => set("watch_clipboard", e.target.checked)}
            />
          </label>

          {error && <div className="form-error">{error}</div>}
        </div>

        <div className="modal-footer">
          <button className="btn" onClick={onClose} disabled={busy}>
            Cancel
          </button>
          <button className="btn btn-primary" onClick={save} disabled={busy}>
            {busy ? "Saving…" : "Save settings"}
          </button>
        </div>
      </div>
    </div>
  );
}
