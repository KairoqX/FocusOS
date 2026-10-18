import type { Chapter, Subject, Task } from "../api";

export function subject(over: Partial<Subject> = {}): Subject {
  return {
    id: 1,
    name: "Biology",
    chapterCount: 0,
    notStarted: 0,
    inProgress: 0,
    completed: 0,
    needsRevision: 0,
    chaptersWithoutTasks: 0,
    percentComplete: 0,
    taskCount: 0,
    openTasks: 0,
    unassignedTasks: 0,
    ...over,
  };
}

export function chapter(over: Partial<Chapter> = {}): Chapter {
  return {
    id: 11,
    subjectId: 1,
    title: "Cells",
    status: "notStarted",
    progressMode: "automatic",
    manualProgress: 0,
    taskTotal: 0,
    taskCompleted: 0,
    progressPercent: 0,
    createdAt: "2025-01-01T00:00:00Z",
    updatedAt: "2025-01-01T00:00:00Z",
    ...over,
  };
}

export function task(over: Partial<Task> = {}): Task {
  return {
    id: 1,
    title: "Read chapter",
    description: "",
    subjectId: null,
    subjectName: null,
    chapterId: null,
    chapterTitle: null,
    priority: "medium",
    dueDate: null,
    estimatedMinutes: null,
    completed: false,
    completedAt: null,
    overdue: false,
    createdAt: "2025-01-01T00:00:00Z",
    updatedAt: "2025-01-01T00:00:00Z",
    ...over,
  };
}
