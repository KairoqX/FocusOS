import { memo } from "react";
import { Pencil, Trash2 } from "lucide-react";
import type { Chapter, ChapterStatus } from "../api";

export const CHAPTER_STATUSES: { id: ChapterStatus; label: string }[] = [
  { id: "notStarted", label: "Not started" },
  { id: "inProgress", label: "In progress" },
  { id: "completed", label: "Completed" },
  { id: "needsRevision", label: "Needs revision" },
];

interface Props {
  chapter: Chapter;
  busy: boolean;
  onStatus: (chapter: Chapter, status: ChapterStatus) => void;
  onRename: (chapter: Chapter) => void;
  onDelete: (chapter: Chapter) => void;
}

export const ChapterRow = memo(function ChapterRow({ chapter, busy, onStatus, onRename, onDelete }: Props) {
  return (
    <li className={`chapter status-${chapter.status}`}>
      <span className="chapter-title">{chapter.title}</span>
      <select
        aria-label={`Status of “${chapter.title}”`}
        value={chapter.status}
        disabled={busy}
        onChange={(e) => onStatus(chapter, e.target.value as ChapterStatus)}
      >
        {CHAPTER_STATUSES.map((s) => (
          <option key={s.id} value={s.id}>
            {s.label}
          </option>
        ))}
      </select>
      <button className="icon-btn" onClick={() => onRename(chapter)} aria-label={`Rename “${chapter.title}”`} title="Rename">
        <Pencil size={16} aria-hidden="true" />
      </button>
      <button className="icon-btn" onClick={() => onDelete(chapter)} aria-label={`Delete “${chapter.title}”`} title="Delete">
        <Trash2 size={16} aria-hidden="true" />
      </button>
    </li>
  );
});
