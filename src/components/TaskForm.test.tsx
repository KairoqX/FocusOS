import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Chapter } from "../api";
import { chapter, subject, task } from "../test/fixtures";

vi.mock("../api", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../api")>();
  return { ...actual, listChapters: vi.fn(), createTask: vi.fn(), updateTask: vi.fn() };
});

import * as api from "../api";
import { TaskForm } from "./TaskForm";

const SUBJECTS = [
  subject({ id: 1, name: "Biology" }),
  subject({ id: 2, name: "Chemistry" }),
  subject({ id: 3, name: "Empty subject" }),
];
const CHAPTERS: Record<number, Chapter[]> = {
  1: [chapter({ id: 11, subjectId: 1, title: "Cells" }), chapter({ id: 12, subjectId: 1, title: "Genetics" })],
  2: [chapter({ id: 21, subjectId: 2, title: "Atoms" })],
  3: [],
};

function renderForm(existing = null as ReturnType<typeof task> | null) {
  const onSaved = vi.fn();
  render(<TaskForm task={existing} subjects={SUBJECTS} onCancel={vi.fn()} onSaved={onSaved} />);
  return { onSaved };
}

const subjectSelect = () => screen.getByLabelText("Subject") as HTMLSelectElement;
const chapterSelect = () => screen.getByLabelText("Chapter") as HTMLSelectElement;
const optionTexts = (el: HTMLSelectElement) => Array.from(el.options).map((o) => o.textContent);
const chooseSubject = (id: string) => fireEvent.change(subjectSelect(), { target: { value: id } });
const submit = () => fireEvent.click(screen.getByRole("button", { name: "Add task" }));
const lastCreateInput = () => vi.mocked(api.createTask).mock.calls[0][0];

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(api.listChapters).mockImplementation(async (id: number) => CHAPTERS[id] ?? []);
  vi.mocked(api.createTask).mockResolvedValue(task());
  vi.mocked(api.updateTask).mockResolvedValue(task());
});

describe("TaskForm: dependent subject/chapter dropdowns", () => {
  it("hides the chapter dropdown until a subject is selected", async () => {
    renderForm();
    expect(screen.queryByLabelText("Chapter")).toBeNull();
    chooseSubject("1");
    expect(chapterSelect()).toBeTruthy();
    expect(api.listChapters).toHaveBeenCalledWith(1);
  });

  it("offers only the chapters of the selected subject", async () => {
    renderForm();
    chooseSubject("1");
    await waitFor(() => expect(optionTexts(chapterSelect())).toEqual(["No chapter", "Cells", "Genetics"]));
    chooseSubject("2");
    await waitFor(() => expect(optionTexts(chapterSelect())).toEqual(["No chapter", "Atoms"]));
  });

  it("clears the selected chapter when the subject changes", async () => {
    renderForm();
    chooseSubject("1");
    await waitFor(() => expect(optionTexts(chapterSelect())).toContain("Genetics"));
    fireEvent.change(chapterSelect(), { target: { value: "12" } });
    expect(chapterSelect().value).toBe("12");

    chooseSubject("2");
    expect(chapterSelect().value).toBe("");
    await waitFor(() => expect(optionTexts(chapterSelect())).toEqual(["No chapter", "Atoms"]));
    expect(chapterSelect().value).toBe("");

    submit();
    await waitFor(() => expect(api.createTask).toHaveBeenCalledTimes(1));
    expect(lastCreateInput()).toMatchObject({ subjectId: 2, chapterId: null });
  });

  it("allows a subject without a chapter", async () => {
    renderForm();
    chooseSubject("1");
    await waitFor(() => expect(optionTexts(chapterSelect())).toContain("Cells"));
    submit();
    await waitFor(() => expect(api.createTask).toHaveBeenCalledTimes(1));
    expect(lastCreateInput()).toMatchObject({ subjectId: 1, chapterId: null });
  });

  it("submits the chosen chapter id", async () => {
    renderForm();
    chooseSubject("1");
    await waitFor(() => expect(optionTexts(chapterSelect())).toContain("Genetics"));
    fireEvent.change(chapterSelect(), { target: { value: "12" } });
    submit();
    await waitFor(() => expect(api.createTask).toHaveBeenCalledTimes(1));
    expect(lastCreateInput()).toMatchObject({ subjectId: 1, chapterId: 12 });
  });

  it("hides the dropdown and sends no chapter when the subject is removed", async () => {
    renderForm();
    chooseSubject("1");
    await waitFor(() => expect(optionTexts(chapterSelect())).toContain("Cells"));
    fireEvent.change(chapterSelect(), { target: { value: "11" } });
    chooseSubject("");
    expect(screen.queryByLabelText("Chapter")).toBeNull();
    submit();
    await waitFor(() => expect(api.createTask).toHaveBeenCalledTimes(1));
    expect(lastCreateInput()).toMatchObject({ subjectId: null, chapterId: null });
  });

  it("explains when a subject has no chapters", async () => {
    renderForm();
    chooseSubject("3");
    expect(await screen.findByText(/no chapters yet/i)).toBeTruthy();
    expect(chapterSelect().disabled).toBe(true);
  });

  it("preselects the existing subject and chapter when editing", async () => {
    renderForm(task({ id: 9, subjectId: 1, subjectName: "Biology", chapterId: 12, chapterTitle: "Genetics" }));
    expect(subjectSelect().value).toBe("1");
    await waitFor(() => expect(chapterSelect().value).toBe("12"));
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));
    await waitFor(() => expect(api.updateTask).toHaveBeenCalledTimes(1));
    expect(vi.mocked(api.updateTask).mock.calls[0][1]).toMatchObject({ subjectId: 1, chapterId: 12 });
  });

  it("shows a backend validation error for the chapter field", async () => {
    vi.mocked(api.createTask).mockRejectedValue(
      new api.ApiError("validation", "The selected chapter does not belong to the selected subject.", "chapterId"),
    );
    renderForm();
    chooseSubject("1");
    await waitFor(() => expect(optionTexts(chapterSelect())).toContain("Cells"));
    submit();
    expect(await screen.findByText(/does not belong to the selected subject/i)).toBeTruthy();
  });
});
