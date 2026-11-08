import { useState, useEffect } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { useAria2 } from "../context/Aria2Provider";
import type { StreamMetadata, StreamFormat } from "../types";
import { CloseIcon, DownloadIcon, FolderIcon } from "../lib/icons";

// Helper for streaming detection
function isStreamingUrl(url: string): boolean {
  const lower = url.toLowerCase();
  return (
    lower.includes("youtube.com") ||
    lower.includes("youtu.be") ||
    lower.includes("twitch.tv") ||
    lower.includes("vimeo.com") ||
    lower.includes("soundcloud.com") ||
    lower.includes("dailymotion.com") ||
    lower.includes("tiktok.com") ||
    lower.includes("twitter.com") ||
    lower.includes("x.com") ||
    lower.includes("instagram.com")
  );
}

interface Props {
  onClose: () => void;
  initialUri?: string;
}

export default function NewDownloadModal({ onClose, initialUri }: Props) {
  const { addDownload, settings } = useAria2();
  const [uris, setUris] = useState(initialUri ?? "");
  const [dir, setDir] = useState("");
  const [out, setOut] = useState("");
  const [split, setSplit] = useState(16);
  const [referer, setReferer] = useState("");
  const [userAgent, setUserAgent] = useState("");
  const [cookie, setCookie] = useState("");
  const [authUser, setAuthUser] = useState("");
  const [authPass, setAuthPass] = useState("");
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  // Streaming state
  const [metadata, setMetadata] = useState<StreamMetadata | null>(null);
  const [selectedFormat, setSelectedFormat] = useState<StreamFormat | null>(null);
  const [fetching, setFetching] = useState(false);

  const browse = async () => {
    const selected = await open({ directory: true, title: "Choose folder" });
    if (typeof selected === "string") setDir(selected);
  };

  const lines = uris
    .split("\n")
    .map((l) => l.trim())
    .filter((l) => l.length > 0);

  // Auto-detect stream metadata when URIs change or cookie updates.
  // Debounced so typing/pasting a cookie file doesn't spawn a yt-dlp per keystroke;
  // stale responses from a superseded request are ignored.
  useEffect(() => {
    if (lines.length === 1 && isStreamingUrl(lines[0])) {
      let cancelled = false;
      setFetching(true);
      setError(null);
      const timer = setTimeout(() => {
        invoke<StreamMetadata>("fetch_stream_metadata", { url: lines[0], cookie: cookie.trim() || undefined })
          .then((data) => {
            if (cancelled) return;
            setMetadata(data);
            setFetching(false);
          })
          .catch((e) => {
            if (cancelled) return;
            const message = String(e);
            // Only talk about cookies when the failure is actually an auth failure,
            // so a working saved cookie never produces noise.
            const authIssue =
              /confirm you.{0,4}re not a bot|cookies are no longer valid|sign in to confirm|login required|use --cookies/i.test(
                message,
              );
            if (authIssue) {
              const local = cookie.trim();
              const saved = settings?.youtube_cookie?.trim();
              setError(
                local
                  ? "The cookie you entered was rejected (invalid or expired). Check it and try again."
                  : saved
                    ? "The saved YouTube cookie was rejected (invalid or expired). Update it in Settings → General."
                    : "YouTube requires a signed-in cookie. Add one once in Settings → General (or paste it in the Cookie field).",
              );
            } else {
              setError(`Stream detection failed: ${message}`);
            }
            setFetching(false);
            setMetadata(null);
          });
      }, 400);
      return () => {
        cancelled = true;
        clearTimeout(timer);
      };
    } else if (lines.length !== 1 || !isStreamingUrl(lines[0] ?? "")) {
      setMetadata(null);
      setFetching(false);
    }
  }, [uris, cookie, settings?.youtube_cookie]);

  const start = async () => {
    if (lines.length === 0) {
      setError("Paste at least one URL.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      // Recognized streaming URL: hand the whole job to yt-dlp (handles HLS/DASH
      // manifests, segment stitching, and audio/video muxing natively).
      if (metadata && lines.length === 1) {
        await invoke("download_stream", {
          url: lines[0],
          formatId: selectedFormat?.format_id || undefined,
          dir: dir.trim() || undefined,
          out: out.trim() || undefined,
          cookie: cookie.trim() || undefined,
        });
        onClose();
        return;
      }
      for (const uri of lines) {
        await addDownload({
          uri,
          dir: dir.trim() || undefined,
          out: lines.length === 1 ? (out.trim() || undefined) : undefined,
          split,
          referer: referer.trim() || undefined,
          userAgent: userAgent.trim() || undefined,
          cookie: cookie.trim() || undefined,
          authUser: authUser.trim() || undefined,
          authPass: authPass.trim() || undefined,
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
              <span className="field-label">
                Filename {lines.length > 1 ? "(disabled for batch)" : "(optional)"}
              </span>
              <input
                value={out}
                onChange={(e) => setOut(e.target.value)}
                placeholder={
                  lines.length > 1
                    ? "Auto-detected per URL"
                    : "Auto-detected from URL"
                }
                disabled={lines.length > 1}
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

          <div className="advanced-toggle-row">
            <button
              type="button"
              className="advanced-toggle-btn"
              onClick={() => setShowAdvanced(!showAdvanced)}
            >
              {showAdvanced ? "▾ Hide advanced options" : "▸ Advanced options (Headers & Auth)"}
            </button>
          </div>

          {showAdvanced && (
            <div className="advanced-fields">
              <label className="field">
                <span className="field-label">Referer URL</span>
                <input
                  value={referer}
                  onChange={(e) => setReferer(e.target.value)}
                  placeholder="https://original-site.com/page"
                  spellCheck={false}
                />
              </label>

              <label className="field">
                <span className="field-label">User-Agent</span>
                <input
                  value={userAgent}
                  onChange={(e) => setUserAgent(e.target.value)}
                  placeholder="Custom browser User-Agent header"
                  spellCheck={false}
                />
              </label>

              <label className="field">
                <span className="field-label">Cookie Header / cookies.txt</span>
                <textarea
                  value={cookie}
                  onChange={(e) => setCookie(e.target.value)}
                  placeholder={"Netscape cookie file contents (paste as-is), a cookies.txt path, or a browser name (chrome / firefox / edge)"}
                  spellCheck={false}
                  rows={4}
                />
              </label>

          {fetching && <div className="form-info">Fetching stream options...</div>}
          
          {metadata && (
            <label className="field">
              <span className="field-label">Stream Format: {metadata.title}</span>
              <select
                className="select"
                onChange={(e) => {
                  const fmt = metadata.formats.find(f => f.format_id === e.target.value);
                  setSelectedFormat(fmt || null);
                }}
              >
                <option value="">Select a format...</option>
                {metadata.formats.map((f) => (
                  <option key={f.format_id} value={f.format_id}>
                    {f.resolution || "Unknown Res"} - {f.ext} - {f.note || ""}
                  </option>
                ))}
              </select>
            </label>
          )}

          <div className="field-row">
                <label className="field grow">
                  <span className="field-label">HTTP Username</span>
                  <input
                    value={authUser}
                    onChange={(e) => setAuthUser(e.target.value)}
                    placeholder="Optional Basic Auth user"
                    spellCheck={false}
                  />
                </label>
                <label className="field grow">
                  <span className="field-label">HTTP Password</span>
                  <input
                    type="password"
                    value={authPass}
                    onChange={(e) => setAuthPass(e.target.value)}
                    placeholder="Optional Basic Auth password"
                    spellCheck={false}
                  />
                </label>
              </div>
            </div>
          )}

          {error && <div className="form-error">{error}</div>}
        </div>

        <div className="modal-footer">
          <button className="btn" onClick={onClose} disabled={busy}>
            Cancel
          </button>
          <button
            className="btn btn-primary"
            onClick={start}
            disabled={busy || lines.length === 0}
          >
            <DownloadIcon width={15} height={15} />
            {busy
              ? "Starting…"
              : lines.length > 1
                ? `Start downloads (${lines.length})`
                : "Start download"}
          </button>
        </div>
      </div>
    </div>
  );
}
