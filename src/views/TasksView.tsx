import { useCallback, useEffect, useRef, useState } from "react";
import { Plus, Search } from "lucide-react";
import {
  deleteTask,
  listSubjects,
  listTasks,
  setTaskCompleted,
  type Priority,
  type Subject,
  type Task,
  type TaskSort,
  type TaskView,
} from "../api";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { Modal } from "../components/Modal";
import { TaskForm } from "../components/TaskForm";
import { TaskRow } from "../components/TaskRow";
import { messageOf } from "../format";

const VIEWS: { id: TaskView; label: string }[] = [
  { id: "today", label: "Today" },
  { id: "upcoming", label: "Upcoming" },
  { id: "completed", label: "Completed" },
  { id: "all", label: "All tasks" },
];

const SORTS: { id: TaskSort; label: string }[] = [
  { id: "dueDate", label: "Due date" },
  { id: "priority", label: "Priority" },
  { id: "created", label: "Newest first" },
  { id: "title", label: "Title" },
];

const EMPTY: Record<TaskView, string> = {
  today: "Nothing due today. Enjoy the calm, or add a task.",
  upcoming: "No upcoming tasks with a future due date.",
  completed: "No completed tasks yet.",
  all: "No tasks yet. Add your first task to get started.",
};

export function TasksView() {
  const [view, setView] = useState<TaskView>("today");
  const [subjectId, setSubjectId] = useState<number | null>(null);
  const [priority, setPriority] = useState<Priority | null>(null);
  const [sort, setSort] = useState<TaskSort>("dueDate");
  const [searchInput, setSearchInput] = useState("");
  const [search, setSearch] = useState("");

  const [tasks, setTasks] = useState<Task[] | null>(null);
  const [subjects, setSubjects] = useState<Subject[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);

  const [editor, setEditor] = useState<{ task: Task | null } | null>(null);
  const [toDelete, setToDelete] = useState<Task | null>(null);
  const [busyId, setBusyId] = useState<number | null>(null);
  const latestRequest = useRef(0);

  const reload = useCallback(() => setReloadKey((k) => k + 1), []);

  // Debounce search so we don't query on every keystroke.
  useEffect(() => {
    const t = setTimeout(() => setSearch(searchInput.trim()), 200);
    return () => clearTimeout(t);
  }, [searchInput]);

  useEffect(() => {
    listSubjects()
      .then(setSubjects)
      .catch((e: unknown) => setError(messageOf(e)));
  }, []);

  useEffect(() => {
    const id = ++latestRequest.current;
    listTasks({ view, sort, subjectId, priority, search })
      .then((result) => {
        if (id !== latestRequest.current) return; // a newer request superseded this one
        setTasks(result);
        setError(null);
      })
      .catch((e: unknown) => {
        if (id === latestRequest.current) setError(messageOf(e));
      });
  }, [view, sort, subjectId, priority, search, reloadKey]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.ctrlKey && !e.shiftKey && !e.altKey && e.key.toLowerCase() === "n") {
        e.preventDefault();
        setEditor({ task: null });
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const toggle = useCallback(
    async (task: Task) => {
      setBusyId(task.id);
      try {
        await setTaskCompleted(task.id, !task.completed);
        reload();
      } catch (e) {
        setError(messageOf(e));
        reload();
      } finally {
        setBusyId(null);
      }
    },
    [reload],
  );
  const edit = useCallback((task: Task) => setEditor({ task }), []);
  const askDelete = useCallback((task: Task) => setToDelete(task), []);

  async function confirmDelete() {
    if (!toDelete) return;
    await deleteTask(toDelete.id); // errors are shown inside the dialog
    setToDelete(null);
    reload();
  }

  const filtered = subjectId !== null || priority !== null || search !== "";

  return (
    <>
      <header className="page-head">
        <div>
          <h1>Tasks</h1>
          <p className="muted">Plan what to study and when.</p>
        </div>
        <button className="btn btn-primary" onClick={() => setEditor({ task: null })} title="New task (Ctrl+N)">
          <Plus size={16} aria-hidden="true" /> New task
        </button>
      </header>

      <div className="tabs" role="group" aria-label="Task view">
        {VIEWS.map((v) => (
          <button key={v.id} className="tab" aria-pressed={view === v.id} onClick={() => setView(v.id)}>
            {v.label}
          </button>
        ))}
      </div>

      <div className="toolbar">
        <label className="search">
          <Search size={16} aria-hidden="true" />
          <input
            type="search"
            placeholder="Search tasks"
            aria-label="Search tasks"
            value={searchInput}
            onChange={(e) => setSearchInput(e.target.value)}
          />
        </label>
        <select
          aria-label="Filter by subject"
          value={subjectId ?? ""}
          onChange={(e) => setSubjectId(e.target.value === "" ? null : Number(e.target.value))}
        >
          <option value="">All subjects</option>
          {subjects.map((s) => (
            <option key={s.id} value={s.id}>
              {s.name}
            </option>
          ))}
        </select>
        <select
          aria-label="Filter by priority"
          value={priority ?? ""}
          onChange={(e) => setPriority(e.target.value === "" ? null : (e.target.value as Priority))}
        >
          <option value="">Any priority</option>
          <option value="high">High</option>
          <option value="medium">Medium</option>
          <option value="low">Low</option>
        </select>
        <select aria-label="Sort tasks" value={sort} onChange={(e) => setSort(e.target.value as TaskSort)}>
          {SORTS.map((s) => (
            <option key={s.id} value={s.id}>
              Sort: {s.label}
            </option>
          ))}
        </select>
      </div>

      {error && (
        <div role="alert" className="banner">
          <span>{error}</span>
          <button className="btn" onClick={reload}>
            Retry
          </button>
        </div>
      )}

      {tasks === null && !error && <p className="muted">Loading tasks…</p>}

      {tasks !== null && tasks.length === 0 && (
        <section className="panel empty">
          <p>{filtered ? "No tasks match your filters." : EMPTY[view]}</p>
          {!filtered && view !== "completed" && (
            <button className="btn btn-primary" onClick={() => setEditor({ task: null })}>
              <Plus size={16} aria-hidden="true" /> Add a task
            </button>
          )}
        </section>
      )}

      {tasks !== null && tasks.length > 0 && (
        <ul className="task-list" aria-label="Tasks">
          {tasks.map((t) => (
            <TaskRow
              key={t.id}
              task={t}
              busy={busyId === t.id}
              onToggle={toggle}
              onEdit={edit}
              onDelete={askDelete}
            />
          ))}
        </ul>
      )}

      <Modal
        open={editor !== null}
        title={editor?.task ? "Edit task" : "New task"}
        onClose={() => setEditor(null)}
      >
        {editor && (
          <TaskForm
            key={editor.task?.id ?? "new"}
            task={editor.task}
            subjects={subjects}
            onCancel={() => setEditor(null)}
            onSaved={() => {
              setEditor(null);
              reload();
            }}
          />
        )}
      </Modal>

      <ConfirmDialog
        open={toDelete !== null}
        title="Delete task?"
        message={toDelete ? `“${toDelete.title}” will be permanently deleted.` : ""}
        confirmLabel="Delete"
        onCancel={() => setToDelete(null)}
        onConfirm={confirmDelete}
      />
    </>
  );
}
