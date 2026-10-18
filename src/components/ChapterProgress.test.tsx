import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Chapter } from "../api";
import { chapter } from "../test/fixtures";
import { ChapterProgress } from "./ChapterProgress";

const onChange = vi.fn();
beforeEach(() => onChange.mockClear());

function renderProgress(over: Partial<Chapter> = {}) {
  render(<ChapterProgress chapter={chapter(over)} onChange={onChange} />);
}
const bar = () => screen.getByRole("progressbar");
const modeButton = (name: "Automatic" | "Manual") => screen.getByRole("button", { name });
const slider = () => screen.getByRole("slider") as HTMLInputElement;

describe("ChapterProgress: automatic mode (default)", () => {
  it("shows 0% and a helpful empty state for a chapter with no tasks", () => {
    renderProgress();
    expect(screen.getByText("0%")).toBeTruthy();
    expect(screen.getByText(/No tasks yet/)).toBeTruthy();
    expect(bar().getAttribute("aria-valuenow")).toBe("0");
    expect(bar().className).toContain("progress-empty");
    expect(modeButton("Automatic").getAttribute("aria-pressed")).toBe("true");
    expect(screen.queryByRole("slider")).toBeNull();
  });

  it("distinguishes 0% with tasks from having no tasks", () => {
    renderProgress({ taskTotal: 3, taskCompleted: 0, progressPercent: 0 });
    expect(screen.getByText("0%")).toBeTruthy();
    expect(screen.getByText("0 of 3 tasks completed")).toBeTruthy();
    expect(screen.queryByText(/No tasks yet/)).toBeNull();
    expect(bar().className).not.toContain("progress-empty");
  });

  it("shows the backend-computed percentage and task counts", () => {
    renderProgress({ taskTotal: 5, taskCompleted: 2, progressPercent: 40 });
    expect(screen.getByText("40%")).toBeTruthy();
    expect(screen.getByText("2 of 5 tasks completed")).toBeTruthy();
    expect(bar().getAttribute("aria-valuenow")).toBe("40");
  });

  it("switches to manual starting from the current percentage", () => {
    renderProgress({ taskTotal: 5, taskCompleted: 2, progressPercent: 40 });
    fireEvent.click(modeButton("Manual"));
    expect(onChange).toHaveBeenCalledWith("manual", 40);
  });

  it("does nothing when the active mode is clicked again", () => {
    renderProgress();
    fireEvent.click(modeButton("Automatic"));
    expect(onChange).not.toHaveBeenCalled();
  });
});

describe("ChapterProgress: manual mode", () => {
  const manual = { progressMode: "manual", manualProgress: 70, progressPercent: 70, taskTotal: 4, taskCompleted: 4 } as const;

  it("shows the manual value, a slider, and explains why it differs from tasks", () => {
    renderProgress(manual);
    expect(screen.getByText("70%")).toBeTruthy(); // not 100% even though all tasks are done
    expect(slider().value).toBe("70");
    expect(modeButton("Manual").getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByText(/set by you/i)).toBeTruthy();
    expect(screen.getByText(/Tasks: 4 of 4 tasks completed/)).toBeTruthy();
  });

  it("commits the slider value on release/blur, not on every drag step", () => {
    renderProgress(manual);
    fireEvent.change(slider(), { target: { value: "85" } });
    expect(screen.getByText("85%")).toBeTruthy(); // live preview while dragging
    expect(onChange).not.toHaveBeenCalled();
    fireEvent.blur(slider());
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith("manual", 85);
  });

  it("does not save when the value did not change", () => {
    renderProgress(manual);
    fireEvent.blur(slider());
    expect(onChange).not.toHaveBeenCalled();
  });

  it("switches back to automatic without sending a manual value", () => {
    renderProgress(manual);
    fireEvent.click(modeButton("Automatic"));
    expect(onChange).toHaveBeenCalledWith("automatic", null);
  });

  it("says so when a manual chapter has no linked tasks", () => {
    renderProgress({ progressMode: "manual", manualProgress: 30, progressPercent: 30 });
    expect(screen.getByText(/No linked tasks/)).toBeTruthy();
    expect(screen.queryByText(/No tasks yet/)).toBeNull();
  });
});
