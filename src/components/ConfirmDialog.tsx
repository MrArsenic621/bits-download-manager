import type { ReactNode } from "react";
import { TrashIcon } from "../lib/icons";

interface Props {
  title: string;
  message: ReactNode;
  confirmLabel?: string;
  onConfirm: () => void;
  onCancel: () => void;
}

export default function ConfirmDialog({
  title,
  message,
  confirmLabel = "Delete",
  onConfirm,
  onCancel,
}: Props) {
  return (
    <div className="modal-overlay" onMouseDown={onCancel}>
      <div className="modal modal-sm" onMouseDown={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>{title}</h2>
        </div>
        <div className="modal-body">
          <p className="confirm-text">{message}</p>
        </div>
        <div className="modal-footer">
          <button className="btn" onClick={onCancel}>
            Cancel
          </button>
          <button className="btn btn-danger" onClick={onConfirm}>
            <TrashIcon width={15} height={15} />
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
