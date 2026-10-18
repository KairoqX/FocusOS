import { useEffect, useId, useState, type FormEvent } from "react";
import {
  ApiError,
  createTask,
  listChapters,
  updateTask,
  type Chapter,
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
  const [chapterId, setChapterId] = useState(task?.chapterId != null ? String(task.chapterId) : "");
  const [chapters, setChapters] = useState<Chapter[]>([]);
  const [chaptersLoading, setChaptersLoading] = useState(false);
  const [chaptersError, setChaptersError] = useState<string | null>(null);
  const [priority, setPriority] = useState<Priority>(task?.priority ?? "medium");
  const [dueDate, setDueDate] = useState(task?.dueDate ?? "");
  const [minutes, setMinutes] = useState(
    task?.estimatedMinutes != null ? String(task.estimatedMinutes) : "",
  );
  const [saving, setSaving] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [formError, setFormError] = useState<string | null>(null);

  // Chapters are loaded for the selected subject only, so the dropdown can never offer a chapter
  // from another subject.
  useEffect(() => {
    if (subjectId === "") {
      setChapters([]);
      setChaptersLoading(false);
      setChaptersError(null);
      return;
    }
    let cancelled = false;
    setChaptersLoading(true);
    setChaptersError(null);
    listChapters(Number(subjectId))
      .then((items) => {
        if (cancelled) return;
        setChapters(items);
        // Keep the selection only if it belongs to this subject.
        setChapterId((cur) => (items.some((c) => String(c.id) === cur) ? cur : ""));
      })
      .catch((e: unknown) => {
        if (cancelled) return;
        setChapters([]);
        setChapterId("");
        setChaptersError(messageOf(e));
      })
      .finally(() => {
        if (!cancelled) setChaptersLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [subjectId]);

  function changeSubject(value: string) {
    setSubjectId(value);
    setChapterId(""); // a chapter always belongs to exactly one subject
  }

  async function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const input: TaskInput = {
      title,
      description,
      subjectId: subjectId === "" ? null : Number(subjectId),
      chapterId: subjectId === "" || chapterId === "" ? null : Number(chapterId),
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
          <select {...fieldProps("subjectId")} value={subjectId} onChange={(e) => changeSubject(e.target.value)}>
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

      {subjectId !== "" && (
        <div className="field">
          <label htmlFor={`${uid}-chapterId`}>Chapter</label>
          <select
            {...fieldProps("chapterId")}
            value={chapterId}
            disabled={chaptersLoading || chapters.length === 0}
            onChange={(e) => setChapterId(e.target.value)}
          >
            <option value="">{chaptersLoading ? "Loading chapters…" : "No chapter"}</option>
            {chapters.map((c) => (
              <option key={c.id} value={c.id}>
                {c.title}
              </option>
            ))}
          </select>
          {fieldError("chapterId")}
          {chaptersError && (
            <p className="field-error" role="alert">
              {chaptersError}
            </p>
          )}
          {!chaptersLoading && !chaptersError && chapters.length === 0 && (
            <p className="muted field-hint">This subject has no chapters yet. Add some on the Subjects screen.</p>
          )}
        </div>
      )}

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
