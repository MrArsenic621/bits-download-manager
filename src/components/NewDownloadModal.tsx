import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useAria2 } from "../context/Aria2Provider";
import { CloseIcon, DownloadIcon, FolderIcon } from "../lib/icons";

interface Props {
  onClose: () => void;
  initialUri?: string;
}

export default function NewDownloadModal({ onClose, initialUri }: Props) {
  const { addDownload } = useAria2();
  const [uris, setUris] = useState(initialUri ?? "");
  const [dir, setDir] = useState("");
  const [out, setOut] = useState("");
  const [split, setSplit] = useState(16);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const browse = async () => {
    const selected = await open({ directory: true, title: "Choose folder" });
    if (typeof selected === "string") setDir(selected);
  };

  const start = async () => {
    const lines = uris
      .split("\n")
      .map((l) => l.trim())
      .filter((l) => l.length > 0);
    if (lines.length === 0) {
      setError("Paste at least one URL.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      for (const uri of lines) {
        await addDownload({
          uri,
          dir: dir.trim() || undefined,
          out: out.trim() || undefined,
          split,
        });
      }
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
          <h2>New download</h2>
          <button className="icon-btn" onClick={onClose} title="Close">
            <CloseIcon width={16} height={16} />
          </button>
        </div>

        <div className="modal-body">
          <label className="field">
            <span className="field-label">URLs (one per line)</span>
            <textarea
              value={uris}
              onChange={(e) => setUris(e.target.value)}
              rows={4}
              placeholder="https://example.com/file.zip"
              spellCheck={false}
              autoFocus
            />
          </label>

          <div className="field-row">
            <label className="field grow">
              <span className="field-label">Save to</span>
              <input
                value={dir}
                onChange={(e) => setDir(e.target.value)}
                placeholder="Default download folder"
                spellCheck={false}
              />
            </label>
            <button className="btn browse-btn" onClick={browse} title="Browse">
              <FolderIcon width={15} height={15} />
            </button>
          </div>

          <div className="field-row">
            <label className="field grow">
              <span className="field-label">Filename (optional)</span>
              <input
                value={out}
                onChange={(e) => setOut(e.target.value)}
                placeholder="Auto-detected from URL"
                spellCheck={false}
              />
            </label>
            <label className="field split-field">
              <span className="field-label">Connections</span>
              <input
                type="number"
                min={1}
                max={64}
                value={split}
                onChange={(e) =>
                  setSplit(
                    Math.min(64, Math.max(1, Number(e.target.value) || 16)),
                  )
                }
              />
            </label>
          </div>

          {error && <div className="form-error">{error}</div>}
        </div>

        <div className="modal-footer">
          <button className="btn" onClick={onClose} disabled={busy}>
            Cancel
          </button>
          <button
            className="btn btn-primary"
            onClick={start}
            disabled={busy || uris.trim().length === 0}
          >
            <DownloadIcon width={15} height={15} />
            {busy ? "Starting…" : "Start download"}
          </button>
        </div>
      </div>
    </div>
  );
}
