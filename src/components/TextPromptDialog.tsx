import { useId, useState, type FormEvent } from "react";
import { messageOf } from "../format";
import { Modal } from "./Modal";

interface Props {
  open: boolean;
  title: string;
  label: string;
  initialValue: string;
  submitLabel: string;
  onCancel: () => void;
  /** Throw (e.g. an ApiError) to show a message and keep the dialog open. */
  onSubmit: (value: string) => Promise<void>;
}

export function TextPromptDialog({ open, title, label, initialValue, submitLabel, onCancel, onSubmit }: Props) {
  return (
    <Modal open={open} title={title} onClose={onCancel}>
      <PromptForm label={label} initialValue={initialValue} submitLabel={submitLabel} onCancel={onCancel} onSubmit={onSubmit} />
    </Modal>
  );
}

function PromptForm({
  label,
  initialValue,
  submitLabel,
  onCancel,
  onSubmit,
}: Pick<Props, "label" | "initialValue" | "submitLabel" | "onCancel" | "onSubmit">) {
  const id = useId();
  const [value, setValue] = useState(initialValue);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  async function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setSaving(true);
    setError(null);
    try {
      await onSubmit(value);
    } catch (err) {
      setError(messageOf(err));
      setSaving(false);
    }
  }

  return (
    <form className="form" onSubmit={submit}>
      <div className="field">
        <label htmlFor={id}>{label}</label>
        <input
          id={id}
          value={value}
          onChange={(e) => setValue(e.target.value)}
          aria-invalid={error ? true : undefined}
          aria-describedby={error ? `${id}-err` : undefined}
          autoFocus
        />
        {error && (
          <p className="field-error" id={`${id}-err`} role="alert">
            {error}
          </p>
        )}
      </div>
      <div className="modal-actions">
        <button type="button" className="btn" onClick={onCancel} disabled={saving}>
          Cancel
        </button>
        <button type="submit" className="btn btn-primary" disabled={saving}>
          {saving ? "Saving…" : submitLabel}
        </button>
      </div>
    </form>
  );
}
