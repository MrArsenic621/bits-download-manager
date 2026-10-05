import { useState, useEffect } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useAria2 } from "../context/Aria2Provider";
import type { Settings } from "../types";
import { CloseIcon, FolderIcon, TrashIcon, PlusIcon } from "../lib/icons";

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

type TabKey = "general" | "scheduler" | "extension";

export default function SettingsModal({ onClose }: Props) {
  const { settings, updateSettings, getVault, updateVault } = useAria2();
  const [activeTab, setActiveTab] = useState<TabKey>("general");
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
          schedule_enabled: false,
          schedule_start_time: "01:00",
          schedule_stop_time: "07:00",
          shutdown_on_finish: false,
          close_to_tray: true,
          global_proxy: "",
        },
  );
  const [vault, setVault] = useState<any[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (activeTab === "extension") {
        getVault().then(setVault).catch(() => {});
    }
  }, [activeTab, getVault]);

  const addVaultEntry = () => {
    const newEntry = { domain: "", auth_user: "", auth_pass: "", cookies: "" };
    setVault([...vault, newEntry]);
  };

  const updateVaultEntry = (index: number, key: string, value: string) => {
    const next = [...vault];
    next[index][key] = value;
    setVault(next);
  };

  const removeVaultEntry = (index: number) => {
    setVault(vault.filter((_, i) => i !== index));
  };


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
      await updateVault(vault);
      onClose();
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  };

  return (
    <div className="modal-overlay" onMouseDown={onClose}>
      <div className="modal modal-settings" onMouseDown={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>Settings</h2>
          <button className="icon-btn" onClick={onClose} title="Close">
            <CloseIcon width={16} height={16} />
          </button>
        </div>

        <div className="settings-tabs">
          <button
            className={`settings-tab ${activeTab === "general" ? "active" : ""}`}
            onClick={() => setActiveTab("general")}
          >
            General
          </button>
          <button
            className={`settings-tab ${activeTab === "scheduler" ? "active" : ""}`}
            onClick={() => setActiveTab("scheduler")}
          >
            Scheduler & Power
          </button>
          <button
            className={`settings-tab ${activeTab === "extension" ? "active" : ""}`}
            onClick={() => setActiveTab("extension")}
          >
            Browser Extension
          </button>
        </div>

        <div className="modal-body">
          {activeTab === "general" && (
            <>
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
                      set(
                        "default_split",
                        Math.min(64, Math.max(1, Number(e.target.value) || 16)),
                      )
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
                      set(
                        "max_concurrent_downloads",
                        Math.min(16, Math.max(1, Number(e.target.value) || 3)),
                      )
                    }
                  />
                </label>
              </div>

              <label className="field">
                <span className="field-label">Global download speed limit</span>
                <select
                  className="select"
                  value={form.global_speed_limit}
                  onChange={(e) =>
                    set("global_speed_limit", Number(e.target.value))
                  }
                >
                  {SPEED_LIMITS.map((o) => (
                    <option key={o.value} value={o.value}>
                      {o.label}
                    </option>
                  ))}
                </select>
              </label>

              <label className="field">
                <span className="field-label">Global Proxy (e.g. socks5://127.0.0.1:1080)</span>
                <input
                  value={form.global_proxy}
                  onChange={(e) => set("global_proxy", e.target.value)}
                  placeholder="Leave empty to use direct connection"
                  spellCheck={false}
                />
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
                  <span className="toggle-title">Close to System Tray</span>
                  <span className="toggle-sub">
                    Keep downloading in the background when closing the window
                  </span>
                </span>
                <input
                  type="checkbox"
                  checked={form.close_to_tray}
                  onChange={(e) => set("close_to_tray", e.target.checked)}
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
            </>
          )}

          {activeTab === "scheduler" && (
            <>
              <label className="toggle-row">
                <span>
                  <span className="toggle-title">Download Scheduler</span>
                  <span className="toggle-sub">
                    Automatically download only during scheduled off-peak hours
                  </span>
                </span>
                <input
                  type="checkbox"
                  checked={form.schedule_enabled}
                  onChange={(e) => set("schedule_enabled", e.target.checked)}
                />
              </label>

              {form.schedule_enabled && (
                <div className="field-row schedule-time-row">
                  <label className="field grow">
                    <span className="field-label">Start time</span>
                    <input
                      type="time"
                      value={form.schedule_start_time}
                      onChange={(e) => set("schedule_start_time", e.target.value)}
                    />
                  </label>
                  <label className="field grow">
                    <span className="field-label">Stop time</span>
                    <input
                      type="time"
                      value={form.schedule_stop_time}
                      onChange={(e) => set("schedule_stop_time", e.target.value)}
                    />
                  </label>
                </div>
              )}

              <label className="toggle-row">
                <span>
                  <span className="toggle-title">Auto-shutdown PC on finish</span>
                  <span className="toggle-sub">
                    Put PC to sleep or shutdown when all active downloads complete
                  </span>
                </span>
                <input
                  type="checkbox"
                  checked={form.shutdown_on_finish}
                  onChange={(e) => set("shutdown_on_finish", e.target.checked)}
                />
              </label>
            </>
          )}

          {activeTab === "extension" && (
            <div className="extension-guide">
              <div className="extension-guide-intro">
                <strong>Credentials Vault</strong>
                <p>Saved cookies & credentials for automatic header injection.</p>
              </div>
              
              <div className="vault-list">
                {vault.map((entry, i) => (
                    <div key={i} className="vault-entry">
                        <input value={entry.domain} placeholder="Domain (e.g. example.com)" onChange={(e) => updateVaultEntry(i, "domain", e.target.value)} />
                        <input value={entry.auth_user || ""} placeholder="Auth User" onChange={(e) => updateVaultEntry(i, "auth_user", e.target.value)} />
                        <input type="password" value={entry.auth_pass || ""} placeholder="Auth Pass" onChange={(e) => updateVaultEntry(i, "auth_pass", e.target.value)} />
                        <input value={entry.cookies || ""} placeholder="Cookies (name=value;...)" onChange={(e) => updateVaultEntry(i, "cookies", e.target.value)} />
                        <button className="icon-btn danger" onClick={() => removeVaultEntry(i)}><TrashIcon/></button>
                    </div>
                ))}
                <button className="btn btn-sm" onClick={addVaultEntry}><PlusIcon/> Add Entry</button>
              </div>
            </div>
          )}

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
