import { memo } from "react";
import { Pencil, Trash2 } from "lucide-react";
import type { Chapter, ChapterStatus, ProgressMode } from "../api";
import { ChapterProgress } from "./ChapterProgress";

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
  onProgress: (chapter: Chapter, mode: ProgressMode, manualProgress: number | null) => void;
  onRename: (chapter: Chapter) => void;
  onDelete: (chapter: Chapter) => void;
}

export const ChapterRow = memo(function ChapterRow({
  chapter,
  busy,
  onStatus,
  onProgress,
  onRename,
  onDelete,
}: Props) {
  return (
    <li className={`chapter status-${chapter.status}`}>
      <div className="chapter-head">
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
      </div>
      <ChapterProgress chapter={chapter} busy={busy} onChange={(mode, manual) => onProgress(chapter, mode, manual)} />
    </li>
  );
});
