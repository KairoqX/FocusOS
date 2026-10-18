import { useEffect, useId, useState, type FormEvent } from "react";
import { Play } from "lucide-react";
import {
  ApiError,
  getTimerSettings,
  listFocusSessions,
  listTasks,
  updateTimerSettings,
  type FocusHistory,
  type Task,
  type TimerSettings,
} from "../api";
import { SessionControls } from "../components/SessionControls";
import { TimerDisplay } from "../components/TimerDisplay";
import { useFocus } from "../focus/FocusContext";
import { formatDateTime, formatFocusTime, messageOf } from "../format";

export function FocusView() {
  const { snapshot, finished, busy, error, historyVersion, start, dismissFinished } = useFocus();
  const active = snapshot?.active ?? null;

  const [settings, setSettings] = useState<TimerSettings | null>(null);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [taskId, setTaskId] = useState("");
  const [history, setHistory] = useState<FocusHistory | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const taskSelectId = useId();

  useEffect(() => {
    getTimerSettings()
      .then(setSettings)
      .catch((e: unknown) => setLoadError(messageOf(e)));
    listTasks({ view: "all", sort: "dueDate", subjectId: null, priority: null, search: "" })
      .then((all) => setTasks(all.filter((t) => !t.completed)))
      .catch((e: unknown) => setLoadError(messageOf(e)));
  }, []);

  useEffect(() => {
    listFocusSessions(30)
      .then(setHistory)
      .catch((e: unknown) => setLoadError(messageOf(e)));
  }, [historyVersion]);

  const idleMs = (settings?.focusMinutes ?? 25) * 60_000;
  const headline = !active
    ? "Ready when you are"
    : active.state === "paused"
      ? "Paused"
      : active.kind === "break"
        ? "Break"
        : active.taskTitle
          ? `Focusing on “${active.taskTitle}”`
          : "Focusing";

  return (
    <>
      <header className="page-head">
        <div>
          <h1>Focus</h1>
          <p className="muted">One timer, saved as you go. It survives restarts.</p>
        </div>
      </header>

      {(error || loadError) && (
        <div role="alert" className="banner">
          <span>{error ?? loadError}</span>
        </div>
      )}

      <section className="timer-card" aria-label="Timer">
        <p className="timer-headline">{headline}</p>
        <TimerDisplay idleMs={idleMs} />

        {active?.autoPaused && active.state === "paused" && (
          <p className="notice" role="status">
            Paused automatically because the app was closed or the computer went to sleep. Time while
            away was not counted. Resume when you are ready.
          </p>
        )}

        {!active && (
          <>
            <div className="field timer-task">
              <label htmlFor={taskSelectId}>Focus on (optional)</label>
              <select id={taskSelectId} value={taskId} onChange={(e) => setTaskId(e.target.value)}>
                <option value="">No specific task</option>
                {tasks.map((t) => (
                  <option key={t.id} value={t.id}>
                    {t.title}
                    {t.subjectName ? ` · ${t.subjectName}` : ""}
                  </option>
                ))}
              </select>
            </div>
            <div className="timer-actions">
              <button
                className="btn btn-primary"
                disabled={busy || snapshot === null}
                onClick={() => void start("focus", taskId === "" ? null : Number(taskId))}
              >
                <Play size={16} aria-hidden="true" /> Start focus
              </button>
              <button className="btn" disabled={busy || snapshot === null} onClick={() => void start("break", null)}>
                Start break
              </button>
            </div>
          </>
        )}

        <SessionControls />

        {finished && !active && (
          <div className="finished" role="status">
            <p>
              {finished.kind === "break"
                ? "Break finished."
                : finished.state === "stopped"
                  ? `Session saved: ${formatFocusTime(finished.elapsedMs)} of ${formatFocusTime(finished.plannedMs)} planned.`
                  : `Session complete: ${formatFocusTime(finished.elapsedMs)} focused.`}
              {finished.taskTitle ? ` “${finished.taskTitle}”` : ""}
            </p>
            <p className="muted">
              {finished.kind === "focus" ? "Take a break when you are ready." : "Ready for the next focus session?"}
            </p>
            <button className="btn" onClick={dismissFinished}>
              Dismiss
            </button>
          </div>
        )}
      </section>

      <TimerSettingsForm settings={settings} onSaved={setSettings} />

      <section className="history" aria-label="Session history">
        <h2>History</h2>
        {history === null && !loadError && <p className="muted">Loading…</p>}
        {history !== null && (
          <p className="muted">
            {history.todaySessions === 0
              ? "No focus sessions yet today."
              : `Today: ${formatFocusTime(history.todayFocusMs)} across ${history.todaySessions} ${
                  history.todaySessions === 1 ? "session" : "sessions"
                }.`}
          </p>
        )}
        {history !== null && history.sessions.length === 0 && (
          <p className="muted empty-note">Finished focus sessions will appear here.</p>
        )}
        {history !== null && history.sessions.length > 0 && (
          <ul className="history-list">
            {history.sessions.map((s) => (
              <li key={s.id} className="history-row">
                <span className="history-when">{formatDateTime(s.startedAtMs)}</span>
                <span className="history-main">
                  {s.taskTitle ?? "Focus session"}
                  {s.subjectName && <span className="muted"> · {s.subjectName}</span>}
                </span>
                <span className="history-dur">
                  {formatFocusTime(s.elapsedMs)}
                  {s.state === "stopped" && <span className="muted"> of {formatFocusTime(s.plannedMs)} (stopped early)</span>}
                </span>
              </li>
            ))}
          </ul>
        )}
      </section>
    </>
  );
}

function TimerSettingsForm({
  settings,
  onSaved,
}: {
  settings: TimerSettings | null;
  onSaved: (s: TimerSettings) => void;
}) {
  const uid = useId();
  const [focusMin, setFocusMin] = useState("");
  const [breakMin, setBreakMin] = useState("");
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [message, setMessage] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  // Fill the inputs once the saved values arrive.
  useEffect(() => {
    if (settings) {
      setFocusMin(String(settings.focusMinutes));
      setBreakMin(String(settings.breakMinutes));
    }
  }, [settings]);

  async function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setSaving(true);
    setErrors({});
    setMessage(null);
    try {
      const saved = await updateTimerSettings({
        focusMinutes: Number.parseInt(focusMin, 10),
        breakMinutes: Number.parseInt(breakMin, 10),
      });
      onSaved(saved);
      setMessage("Saved. New durations apply to the next session.");
    } catch (err) {
      if (err instanceof ApiError && err.kind === "validation" && err.field) {
        setErrors({ [err.field]: err.message });
      } else {
        setMessage(messageOf(err));
      }
    } finally {
      setSaving(false);
    }
  }

  return (
    <section className="panel settings-panel" aria-label="Timer durations">
      <h2>Durations</h2>
      <form className="form" onSubmit={submit}>
        <div className="field-row">
          <div className="field">
            <label htmlFor={`${uid}-focus`}>Focus (minutes)</label>
            <input
              id={`${uid}-focus`}
              type="number"
              min={1}
              step={1}
              value={focusMin}
              onChange={(e) => setFocusMin(e.target.value)}
              aria-invalid={errors.focusMinutes ? true : undefined}
            />
            {errors.focusMinutes && <p className="field-error" role="alert">{errors.focusMinutes}</p>}
          </div>
          <div className="field">
            <label htmlFor={`${uid}-break`}>Break (minutes)</label>
            <input
              id={`${uid}-break`}
              type="number"
              min={1}
              step={1}
              value={breakMin}
              onChange={(e) => setBreakMin(e.target.value)}
              aria-invalid={errors.breakMinutes ? true : undefined}
            />
            {errors.breakMinutes && <p className="field-error" role="alert">{errors.breakMinutes}</p>}
          </div>
        </div>
        <div className="modal-actions">
          {message && <span className="muted" role="status">{message}</span>}
          <button type="submit" className="btn" disabled={saving || settings === null}>
            {saving ? "Saving…" : "Save durations"}
          </button>
        </div>
      </form>
    </section>
  );
}
