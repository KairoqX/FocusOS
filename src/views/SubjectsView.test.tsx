import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { chapter, subject } from "../test/fixtures";

vi.mock("../api", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../api")>();
  return {
    ...actual,
    listSubjects: vi.fn(),
    listChapters: vi.fn(),
    updateChapterProgress: vi.fn(),
  };
});

import * as api from "../api";
import { SubjectsView } from "./SubjectsView";

const SUBJECT = subject({
  id: 1,
  name: "Biology",
  chapterCount: 3,
  percentComplete: 33, // (100 + 0 + 0) / 3
  needsRevision: 1,
  chaptersWithoutTasks: 1,
  taskCount: 5,
  openTasks: 3,
  unassignedTasks: 1,
});
const CHAPTERS = [
  chapter({ id: 11, title: "Cells", taskTotal: 2, taskCompleted: 2, progressPercent: 100 }),
  chapter({ id: 12, title: "Genetics", taskTotal: 2, taskCompleted: 0, progressPercent: 0 }),
  chapter({ id: 13, title: "Ecology" }), // no tasks
];

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(api.listSubjects).mockResolvedValue([SUBJECT]);
  vi.mocked(api.listChapters).mockResolvedValue(CHAPTERS);
  vi.mocked(api.updateChapterProgress).mockResolvedValue(chapter());
});

describe("SubjectsView progress display", () => {
  it("shows overall subject progress, counts and attention hints", async () => {
    render(<SubjectsView />);
    expect(await screen.findByText("33%")).toBeTruthy();
    expect(screen.getByText(/average of 3 chapters/i)).toBeTruthy();
    expect(screen.getByText(/2 of 5 tasks completed/)).toBeTruthy();
    expect(screen.getByText(/1 task not in a chapter/)).toBeTruthy();
    expect(screen.getByText(/1 marked needs revision · 1 without tasks/)).toBeTruthy();
  });

  it("shows each chapter's own progress and tells empty chapters apart from 0%", async () => {
    render(<SubjectsView />);
    await screen.findByText("Genetics");
    expect(screen.getByText("100%")).toBeTruthy();
    expect(screen.getByText("0 of 2 tasks completed")).toBeTruthy();
    expect(screen.getAllByText(/No tasks yet/)).toHaveLength(1);
  });

  it("saves a mode change and reloads the derived values", async () => {
    render(<SubjectsView />);
    await screen.findByText("Genetics");
    const [firstManual] = screen.getAllByRole("button", { name: "Manual" });
    fireEvent.click(firstManual);
    await waitFor(() => expect(api.updateChapterProgress).toHaveBeenCalledWith(11, "manual", 100));
    await waitFor(() => expect(vi.mocked(api.listSubjects).mock.calls.length).toBeGreaterThan(1));
  });
});
