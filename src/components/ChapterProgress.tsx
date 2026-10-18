import { useEffect, useState } from "react";
import type { Chapter, ProgressMode } from "../api";
import { plural } from "../format";
import { ProgressBar } from "./ProgressBar";

interface Props {
  chapter: Chapter;
  busy?: boolean;
  /** `manualProgress` is null when switching to automatic. */
  onChange: (mode: ProgressMode, manualProgress: number | null) => void;
}

/**
 * Shows a chapter's progress and its tracking mode. The percentage itself is always the value
 * computed by the backend (`chapter.progressPercent`); the UI never recomputes it, except while
 * a manual slider is being dragged.
 */
export function ChapterProgress({ chapter, busy = false, onChange }: Props) {
  const isManual = chapter.progressMode === "manual";
  const [draft, setDraft] = useState(chapter.manualProgress);
  useEffect(() => setDraft(chapter.manualProgress), [chapter.manualProgress]);

  const noTasks = !isManual && chapter.taskTotal === 0;
  const shown = isManual ? draft : chapter.progressPercent;
  const taskSummary = `${chapter.taskCompleted} of ${plural(chapter.taskTotal, "task")} completed`;

  // Commit the slider value when the user lets go (pointer, keyboard or focus loss).
  const commit = () => {
    if (isManual && draft !== chapter.manualProgress) onChange("manual", draft);
  };

  return (
    <div className="chapter-progress">
      <div className="progress-line">
        <ProgressBar percent={shown} empty={noTasks} label={`Progress of “${chapter.title}”`} />
        <span className="chapter-percent">{shown}%</span>
        <div className="mode-toggle" role="group" aria-label={`Progress tracking for “${chapter.title}”`}>
          <button
            type="button"
            className="mode-btn"
            aria-pressed={!isManual}
            disabled={busy}
            onClick={() => {
              if (isManual) onChange("automatic", null);
            }}
          >
            Automatic
          </button>
          <button
            type="button"
            className="mode-btn"
            aria-pressed={isManual}
            disabled={busy}
            onClick={() => {
              // Start from what the user currently sees so switching does not jump.
              if (!isManual) onChange("manual", chapter.progressPercent);
            }}
          >
            Manual
          </button>
        </div>
      </div>

      {isManual && (
        <div className="manual-control">
          <input
            type="range"
            min={0}
            max={100}
            step={1}
            value={draft}
            disabled={busy}
            aria-label={`Manual progress for “${chapter.title}”`}
            onChange={(e) => setDraft(Number(e.target.value))}
            onPointerUp={commit}
            onKeyUp={commit}
            onBlur={commit}
          />
        </div>
      )}

      {noTasks && (
        <p className="muted progress-note empty-note">
          No tasks yet. Link tasks to this chapter from the Tasks screen to track progress automatically.
        </p>
      )}
      {!isManual && chapter.taskTotal > 0 && <p className="muted progress-note">{taskSummary}</p>}
      {isManual && (
        <p className="muted progress-note">
          Manual progress, set by you, so it can differ from task completion.{" "}
          {chapter.taskTotal > 0 ? `Tasks: ${taskSummary}.` : "No linked tasks."}
        </p>
      )}
    </div>
  );
}
