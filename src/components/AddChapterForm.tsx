import { useId, useState, type FormEvent } from "react";
import { Plus } from "lucide-react";
import { messageOf } from "../format";

interface Props {
  onAdd: (title: string) => Promise<void>;
}

export function AddChapterForm({ onAdd }: Props) {
  const id = useId();
  const [title, setTitle] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  async function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setSaving(true);
    setError(null);
    try {
      await onAdd(title);
      setTitle("");
    } catch (err) {
      setError(messageOf(err)); // validation rules live in Rust
    } finally {
      setSaving(false);
    }
  }

  return (
    <form className="add-chapter" onSubmit={submit}>
      <div className="add-chapter-row">
        <input
          id={id}
          placeholder="Add a chapter…"
          aria-label="New chapter title"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          aria-invalid={error ? true : undefined}
          aria-describedby={error ? `${id}-err` : undefined}
        />
        <button type="submit" className="btn" disabled={saving}>
          <Plus size={16} aria-hidden="true" /> Add
        </button>
      </div>
      {error && (
        <p className="field-error" id={`${id}-err`} role="alert">
          {error}
        </p>
      )}
    </form>
  );
}
