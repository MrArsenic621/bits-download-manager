import type { Download } from "../types";
import { TrashIcon } from "../lib/icons";

interface Props {
  download: Download;
  onConfirm: () => void;
  onCancel: () => void;
}

export default function ConfirmDialog({ download, onConfirm, onCancel }: Props) {
  return (
    <div className="modal-overlay" onMouseDown={onCancel}>
      <div className="modal modal-sm" onMouseDown={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>Delete download</h2>
        </div>
        <div className="modal-body">
          <p className="confirm-text">
            Delete <strong>{download.filename || "(unknown)"}</strong> and its
            files from disk? This cannot be undone.
          </p>
        </div>
        <div className="modal-footer">
          <button className="btn" onClick={onCancel}>
            Cancel
          </button>
          <button className="btn btn-danger" onClick={onConfirm}>
            <TrashIcon width={15} height={15} />
            Delete file
          </button>
        </div>
      </div>
    </div>
  );
}
