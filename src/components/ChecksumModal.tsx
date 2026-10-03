import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { CloseIcon, CheckCircleIcon, AlertIcon } from "../lib/icons";

interface Props {
  filePath: string;
  filename: string;
  onClose: () => void;
}

type Algorithm = "SHA-256" | "SHA-1" | "MD5";

export default function ChecksumModal({ filePath, filename, onClose }: Props) {
  const [algo, setAlgo] = useState<Algorithm>("SHA-256");
  const [hash, setHash] = useState<string>("");
  const [expected, setExpected] = useState<string>("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    setHash("");

    invoke<string>("calculate_checksum", {
      path: filePath,
      algorithm: algo,
    })
      .then((res) => {
        if (!cancelled) {
          setHash(res);
          setLoading(false);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setError(String(err));
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [filePath, algo]);

  const copyHash = async () => {
    if (!hash) return;
    try {
      await navigator.clipboard.writeText(hash);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // ignore
    }
  };

  const normalizedExpected = expected.trim().toLowerCase();
  const isMatch = hash && normalizedExpected && hash.toLowerCase() === normalizedExpected;
  const isMismatch = hash && normalizedExpected && hash.toLowerCase() !== normalizedExpected;

  return (
    <div className="modal-overlay" onMouseDown={onClose}>
      <div className="modal" onMouseDown={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>Verify Checksum</h2>
          <button className="icon-btn" onClick={onClose} title="Close">
            <CloseIcon width={16} height={16} />
          </button>
        </div>

        <div className="modal-body">
          <div className="checksum-target">
            <span className="checksum-target-label">File:</span>
            <span className="checksum-target-name" title={filePath}>
              {filename}
            </span>
          </div>

          <div className="field-row">
            <label className="field grow">
              <span className="field-label">Algorithm</span>
              <select
                className="select"
                value={algo}
                onChange={(e) => setAlgo(e.target.value as Algorithm)}
              >
                <option value="SHA-256">SHA-256 (Recommended)</option>
                <option value="SHA-1">SHA-1</option>
                <option value="MD5">MD5</option>
              </select>
            </label>
          </div>

          <label className="field">
            <div className="checksum-label-row">
              <span className="field-label">Calculated {algo}</span>
              {hash && (
                <button
                  type="button"
                  className="btn btn-sm"
                  onClick={copyHash}
                >
                  {copied ? "Copied!" : "Copy"}
                </button>
              )}
            </div>
            <textarea
              className="checksum-output"
              value={loading ? "Calculating hash…" : hash}
              readOnly
              rows={2}
              spellCheck={false}
            />
          </label>

          <label className="field">
            <span className="field-label">Compare with expected hash</span>
            <input
              value={expected}
              onChange={(e) => setExpected(e.target.value)}
              placeholder="Paste original hash to verify match…"
              spellCheck={false}
            />
          </label>

          {isMatch && (
            <div className="checksum-status match">
              <CheckCircleIcon width={16} height={16} />
              <span>Checksum matched! File integrity verified.</span>
            </div>
          )}

          {isMismatch && (
            <div className="checksum-status mismatch">
              <AlertIcon width={16} height={16} />
              <span>Checksum mismatch! File may be corrupted or modified.</span>
            </div>
          )}

          {error && <div className="form-error">{error}</div>}
        </div>

        <div className="modal-footer">
          <button className="btn btn-primary" onClick={onClose}>
            Done
          </button>
        </div>
      </div>
    </div>
  );
}
