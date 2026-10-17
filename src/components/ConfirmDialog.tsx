import { useState } from "react";
import { messageOf } from "../format";
import { Modal } from "./Modal";

interface Props {
  open: boolean;
  title: string;
  message: string;
  confirmLabel: string;
  onCancel: () => void;
  onConfirm: () => Promise<void>;
}

export function ConfirmDialog({ open, title, message, confirmLabel, onCancel, onConfirm }: Props) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function confirm() {
    setBusy(true);
    setError(null);
    try {
      await onConfirm();
    } catch (e) {
      setError(messageOf(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Modal open={open} title={title} onClose={onCancel}>
      <p>{message}</p>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      <div className="modal-actions">
        <button type="button" className="btn" onClick={onCancel} disabled={busy}>
          Cancel
        </button>
        <button type="button" className="btn btn-danger" onClick={confirm} disabled={busy} autoFocus>
          {busy ? "Working…" : confirmLabel}
        </button>
      </div>
    </Modal>
  );
}
