import { memo } from "react";
import { Pencil, Trash2 } from "lucide-react";
import type { Task } from "../api";
import { formatDate, formatMinutes } from "../format";

interface Props {
  task: Task;
  busy: boolean;
  onToggle: (task: Task) => void;
  onEdit: (task: Task) => void;
  onDelete: (task: Task) => void;
}

const PRIORITY_LABEL = { low: "Low", medium: "Medium", high: "High" } as const;

export const TaskRow = memo(function TaskRow({ task, busy, onToggle, onEdit, onDelete }: Props) {
  return (
    <li className={task.completed ? "task done" : "task"}>
      <input
        type="checkbox"
        className="task-check"
        checked={task.completed}
        disabled={busy}
        onChange={() => onToggle(task)}
        aria-label={`${task.completed ? "Reopen" : "Complete"} “${task.title}”`}
      />
      <div className="task-main">
        <div className="task-title">{task.title}</div>
        <div className="task-meta">
          {task.subjectName && <span>{task.subjectName}</span>}
          <span className={`prio prio-${task.priority}`}>{PRIORITY_LABEL[task.priority]}</span>
          {task.dueDate && (
            <span className={task.overdue ? "overdue" : undefined}>
              {task.overdue ? "Overdue · " : "Due "}
              {formatDate(task.dueDate)}
            </span>
          )}
          {task.estimatedMinutes != null && <span>{formatMinutes(task.estimatedMinutes)}</span>}
        </div>
        {task.description && <p className="task-desc muted">{task.description}</p>}
      </div>
      <div className="task-actions">
        <button className="icon-btn" onClick={() => onEdit(task)} aria-label={`Edit “${task.title}”`} title="Edit">
          <Pencil size={16} aria-hidden="true" />
        </button>
        <button className="icon-btn" onClick={() => onDelete(task)} aria-label={`Delete “${task.title}”`} title="Delete">
          <Trash2 size={16} aria-hidden="true" />
        </button>
      </div>
    </li>
  );
});
