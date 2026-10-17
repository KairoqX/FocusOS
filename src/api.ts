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
  percentComplete: number; // computed in Rust from chapter records
  taskCount: number;
  openTasks: number;
}

export type ChapterStatus = "notStarted" | "inProgress" | "completed" | "needsRevision";

export interface Chapter {
  id: number;
  subjectId: number;
  title: string;
  status: ChapterStatus;
  createdAt: string;
  updatedAt: string;
}

export type ErrorKind = "validation" | "notFound" | "database" | "internal";

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
export const deleteChapter = (id: number) => call<void>("delete_chapter", { id });
