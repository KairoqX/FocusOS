import { invoke } from "@tauri-apps/api/core";

export interface AppInfo {
  name: string;
  version: string;
  os: string;
  arch: string;
  schemaVersion: number;
  dbPath: string;
}

export type Priority = "low" | "medium" | "high";
export type TaskView = "today" | "upcoming" | "completed" | "all";
export type TaskSort = "dueDate" | "priority" | "created" | "title";

export interface Task {
  id: number;
  title: string;
  description: string;
  subjectId: number | null;
  subjectName: string | null;
  chapterId: number | null;
  chapterTitle: string | null;
  priority: Priority;
  dueDate: string | null; // YYYY-MM-DD
  estimatedMinutes: number | null;
  completed: boolean;
  completedAt: string | null;
  overdue: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface TaskInput {
  title: string;
  description: string;
  subjectId: number | null;
  chapterId: number | null; // requires subjectId; must belong to that subject
  priority: Priority;
  dueDate: string | null;
  estimatedMinutes: number | null;
}

export interface TaskQuery {
  view: TaskView;
  sort: TaskSort;
  subjectId: number | null;
  priority: Priority | null;
  search: string;
}

export interface Subject {
  id: number;
  name: string;
  chapterCount: number;
  notStarted: number;
  inProgress: number;
  completed: number;
  needsRevision: number;
  chaptersWithoutTasks: number; // automatic-mode chapters with no linked tasks
  percentComplete: number; // unweighted mean of chapter progress, computed in Rust
  taskCount: number;
  openTasks: number;
  unassignedTasks: number; // subject tasks not linked to any chapter
}

export type ProgressMode = "automatic" | "manual";

export type ChapterStatus = "notStarted" | "inProgress" | "completed" | "needsRevision";

export interface Chapter {
  id: number;
  subjectId: number;
  title: string;
  status: ChapterStatus;
  progressMode: ProgressMode;
  manualProgress: number; // stored value; only used in manual mode
  taskTotal: number;
  taskCompleted: number;
  progressPercent: number; // effective progress, derived in Rust on every read
  createdAt: string;
  updatedAt: string;
}

export type SessionKind = "focus" | "break";
export type SessionState = "running" | "paused" | "completed" | "stopped";

export interface FocusSession {
  id: number;
  kind: SessionKind;
  state: SessionState;
  plannedMs: number;
  elapsedMs: number; // derived by the backend at snapshot time
  remainingMs: number; // 0 for finished sessions
  startedAtMs: number;
  endedAtMs: number | null;
  autoPaused: boolean; // paused because the app was closed / the computer slept
  taskId: number | null;
  taskTitle: string | null;
  subjectId: number | null;
  subjectName: string | null;
}

export interface TimerSnapshot {
  active: FocusSession | null;
  finished: FocusSession | null; // set when this call finished (or re-reported) a session
  nowMs: number;
}

export interface TimerSettings {
  focusMinutes: number;
  breakMinutes: number;
}

export interface FocusHistory {
  sessions: FocusSession[];
  todayFocusMs: number;
  todaySessions: number;
}

export type ErrorKind = "validation" | "notFound" | "conflict" | "database" | "internal";

export class ApiError extends Error {
  readonly kind: ErrorKind;
  readonly field?: string;

  constructor(kind: ErrorKind, message: string, field?: string) {
    super(message);
    this.name = "ApiError";
    this.kind = kind;
    this.field = field;
  }
}

interface RawError {
  kind?: string;
  message?: string;
  field?: string;
  entity?: string;
}

function toApiError(e: unknown): ApiError {
  if (e instanceof ApiError) return e;
  if (typeof e === "object" && e !== null && "kind" in e) {
    const raw = e as RawError;
    switch (raw.kind) {
      case "validation":
        return new ApiError("validation", raw.message ?? "Invalid input.", raw.field);
      case "notFound":
        return new ApiError(
          "notFound",
          `That ${raw.entity ?? "item"} no longer exists. It may have been deleted.`,
        );
      case "conflict":
        return new ApiError("conflict", raw.message ?? "That action is not possible right now.");
      case "database":
        return new ApiError("database", raw.message ?? "Database error.");
      default:
        return new ApiError("internal", raw.message ?? "Unexpected error.");
    }
  }
  return new ApiError("internal", typeof e === "string" ? e : "Unexpected error.");
}

// All backend calls live here; components never call invoke directly.
async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (e) {
    throw toApiError(e);
  }
}

export const getAppInfo = () => call<AppInfo>("get_app_info");
export const listTasks = (query: TaskQuery) => call<Task[]>("list_tasks", { query });
export const createTask = (input: TaskInput) => call<Task>("create_task", { input });
export const updateTask = (id: number, input: TaskInput) =>
  call<Task>("update_task", { id, input });
export const deleteTask = (id: number) => call<void>("delete_task", { id });
export const setTaskCompleted = (id: number, completed: boolean) =>
  call<Task>("set_task_completed", { id, completed });
export const listSubjects = () => call<Subject[]>("list_subjects");
export const createSubject = (name: string) =>
  call<Subject>("create_subject", { input: { name } });
export const updateSubject = (id: number, name: string) =>
  call<Subject>("update_subject", { id, input: { name } });
export const deleteSubject = (id: number) => call<void>("delete_subject", { id });

export const listChapters = (subjectId: number) =>
  call<Chapter[]>("list_chapters", { subjectId });
export const createChapter = (subjectId: number, title: string) =>
  call<Chapter>("create_chapter", { subjectId, input: { title } });
export const updateChapter = (id: number, title: string) =>
  call<Chapter>("update_chapter", { id, input: { title } });
export const updateChapterStatus = (id: number, status: ChapterStatus) =>
  call<Chapter>("update_chapter_status", { id, status });
export const updateChapterProgress = (
  id: number,
  mode: ProgressMode,
  manualProgress: number | null,
) => call<Chapter>("update_chapter_progress", { id, input: { mode, manualProgress } });
export const deleteChapter = (id: number) => call<void>("delete_chapter", { id });

export const getFocusState = () => call<TimerSnapshot>("get_focus_state");
export const focusHeartbeat = () => call<TimerSnapshot>("focus_heartbeat");
export const startFocusSession = (kind: SessionKind, taskId: number | null) =>
  call<TimerSnapshot>("start_focus_session", { input: { kind, taskId } });
export const pauseFocusSession = (id: number) => call<TimerSnapshot>("pause_focus_session", { id });
export const resumeFocusSession = (id: number) => call<TimerSnapshot>("resume_focus_session", { id });
export const completeFocusSession = (id: number) =>
  call<TimerSnapshot>("complete_focus_session", { id });
export const stopFocusSession = (id: number) => call<TimerSnapshot>("stop_focus_session", { id });
export const resetFocusSession = (id: number) => call<TimerSnapshot>("reset_focus_session", { id });
export const listFocusSessions = (limit = 30) =>
  call<FocusHistory>("list_focus_sessions", { limit });
export const getTimerSettings = () => call<TimerSettings>("get_timer_settings");
export const updateTimerSettings = (input: TimerSettings) =>
  call<TimerSettings>("update_timer_settings", { input });
