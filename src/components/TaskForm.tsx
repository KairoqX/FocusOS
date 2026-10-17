import { useId, useState, type FormEvent } from "react";
import {
  ApiError,
  createTask,
  updateTask,
  type Priority,
  type Subject,
  type Task,
  type TaskInput,
} from "../api";
import { messageOf } from "../format";

interface Props {
  task: Task | null; // null = create
  subjects: Subject[];
  onCancel: () => void;
  onSaved: () => void;
}

export function TaskForm({ task, subjects, onCancel, onSaved }: Props) {
  const uid = useId();
  const [title, setTitle] = useState(task?.title ?? "");
  const [description, setDescription] = useState(task?.description ?? "");
  const [subjectId, setSubjectId] = useState(task?.subjectId != null ? String(task.subjectId) : "");
  const [priority, setPriority] = useState<Priority>(task?.priority ?? "medium");
  const [dueDate, setDueDate] = useState(task?.dueDate ?? "");
  const [minutes, setMinutes] = useState(
    task?.estimatedMinutes != null ? String(task.estimatedMinutes) : "",
  );
  const [saving, setSaving] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [formError, setFormError] = useState<string | null>(null);

  async function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const input: TaskInput = {
      title,
      description,
      subjectId: subjectId === "" ? null : Number(subjectId),
      priority,
      dueDate: dueDate === "" ? null : dueDate,
      estimatedMinutes: minutes.trim() === "" ? null : Number.parseInt(minutes, 10),
    };
    setSaving(true);
    setErrors({});
    setFormError(null);
    try {
      if (task) await updateTask(task.id, input);
      else await createTask(input);
      onSaved();
    } catch (err) {
      // Validation rules live in Rust; the UI only displays them.
      if (err instanceof ApiError && err.kind === "validation" && err.field) {
        setErrors({ [err.field]: err.message });
      } else {
        setFormError(messageOf(err));
      }
      setSaving(false);
    }
  }

  const fieldProps = (name: string) => ({
    id: `${uid}-${name}`,
    "aria-invalid": errors[name] ? true : undefined,
    "aria-describedby": errors[name] ? `${uid}-${name}-err` : undefined,
  });
  const fieldError = (name: string) =>
    errors[name] ? (
      <p className="field-error" id={`${uid}-${name}-err`} role="alert">
        {errors[name]}
      </p>
    ) : null;

  return (
    <form onSubmit={submit} className="form">
      <div className="field">
        <label htmlFor={`${uid}-title`}>Title</label>
        <input {...fieldProps("title")} value={title} onChange={(e) => setTitle(e.target.value)} autoFocus />
        {fieldError("title")}
      </div>

      <div className="field">
        <label htmlFor={`${uid}-description`}>Description</label>
        <textarea
          {...fieldProps("description")}
          rows={3}
          value={description}
          onChange={(e) => setDescription(e.target.value)}
        />
        {fieldError("description")}
      </div>

      <div className="field-row">
        <div className="field">
          <label htmlFor={`${uid}-subjectId`}>Subject</label>
          <select {...fieldProps("subjectId")} value={subjectId} onChange={(e) => setSubjectId(e.target.value)}>
            <option value="">No subject</option>
            {subjects.map((s) => (
              <option key={s.id} value={s.id}>
                {s.name}
              </option>
            ))}
          </select>
          {fieldError("subjectId")}
        </div>
        <div className="field">
          <label htmlFor={`${uid}-priority`}>Priority</label>
          <select
            {...fieldProps("priority")}
            value={priority}
            onChange={(e) => setPriority(e.target.value as Priority)}
          >
            <option value="low">Low</option>
            <option value="medium">Medium</option>
            <option value="high">High</option>
          </select>
        </div>
      </div>

      <div className="field-row">
        <div className="field">
          <label htmlFor={`${uid}-dueDate`}>Due date</label>
          <input {...fieldProps("dueDate")} type="date" value={dueDate} onChange={(e) => setDueDate(e.target.value)} />
          {fieldError("dueDate")}
        </div>
        <div className="field">
          <label htmlFor={`${uid}-estimatedMinutes`}>Estimate (minutes)</label>
          <input
            {...fieldProps("estimatedMinutes")}
            type="number"
            min={1}
            step={1}
            value={minutes}
            onChange={(e) => setMinutes(e.target.value)}
          />
          {fieldError("estimatedMinutes")}
        </div>
      </div>

      {formError && (
        <p role="alert" className="error">
          {formError}
        </p>
      )}

      <div className="modal-actions">
        <button type="button" className="btn" onClick={onCancel} disabled={saving}>
          Cancel
        </button>
        <button type="submit" className="btn btn-primary" disabled={saving}>
          {saving ? "Saving…" : task ? "Save changes" : "Add task"}
        </button>
      </div>
    </form>
  );
}
