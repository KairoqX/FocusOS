import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { session, snapshot, task } from "../test/fixtures";

vi.mock("../api", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../api")>();
  return {
    ...actual,
    getFocusState: vi.fn(),
    focusHeartbeat: vi.fn(),
    startFocusSession: vi.fn(),
    pauseFocusSession: vi.fn(),
    resumeFocusSession: vi.fn(),
    completeFocusSession: vi.fn(),
    stopFocusSession: vi.fn(),
    resetFocusSession: vi.fn(),
    listFocusSessions: vi.fn(),
    getTimerSettings: vi.fn(),
    updateTimerSettings: vi.fn(),
    listTasks: vi.fn(),
  };
});

import * as api from "../api";
import { FocusProvider } from "../focus/FocusContext";
import { FocusView } from "./FocusView";

const EMPTY_HISTORY = { sessions: [], todayFocusMs: 0, todaySessions: 0 };

function renderView() {
  return render(
    <FocusProvider>
      <FocusView />
    </FocusProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(api.getFocusState).mockResolvedValue(snapshot(null));
  vi.mocked(api.listFocusSessions).mockResolvedValue(EMPTY_HISTORY);
  vi.mocked(api.getTimerSettings).mockResolvedValue({ focusMinutes: 25, breakMinutes: 5 });
  vi.mocked(api.listTasks).mockResolvedValue([
    task({ id: 5, title: "Read optics", subjectName: "Physics" }),
    task({ id: 6, title: "Already done", completed: true }),
  ]);
});

describe("FocusView", () => {
  it("is idle by default, showing the configured length and only open tasks", async () => {
    renderView();
    expect(await screen.findByRole("button", { name: "Start focus" })).toBeTruthy();
    expect(screen.getByText("25:00")).toBeTruthy();
    await screen.findByText("No focus sessions yet today.");
    expect(screen.queryByText("Already done")).toBeNull();
  });

  it("starts a focus session on the chosen task", async () => {
    vi.mocked(api.startFocusSession).mockResolvedValue(snapshot(session({ taskId: 5, taskTitle: "Read optics" })));
    renderView();
    const start = await screen.findByRole("button", { name: "Start focus" });
    await waitFor(() => expect(screen.getByRole("option", { name: /Read optics/ })).toBeTruthy());
    fireEvent.change(screen.getByLabelText("Focus on (optional)"), { target: { value: "5" } });
    fireEvent.click(start);
    await waitFor(() => expect(api.startFocusSession).toHaveBeenCalledWith("focus", 5));
    expect(await screen.findByText("Focusing on “Read optics”")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Pause" })).toBeTruthy();
  });

  it("restores a running session after a restart", async () => {
    vi.mocked(api.getFocusState).mockResolvedValue(
      snapshot(session({ elapsedMs: 5 * 60_000, remainingMs: 20 * 60_000 })),
    );
    renderView();
    expect(await screen.findByRole("button", { name: "Pause" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Start focus" })).toBeNull();
  });

  it("explains an automatic pause and offers Resume", async () => {
    vi.mocked(api.getFocusState).mockResolvedValue(
      snapshot(session({ state: "paused", autoPaused: true, elapsedMs: 120_000, remainingMs: 23 * 60_000 })),
    );
    renderView();
    expect(await screen.findByText(/Paused automatically because the app was closed/)).toBeTruthy();
    expect(screen.getByText("23:00")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Resume" })).toBeTruthy();
  });

  it("offers Stop & save only after a minute of elapsed focus time", async () => {
    vi.mocked(api.getFocusState).mockResolvedValue(
      snapshot(session({ state: "paused", elapsedMs: 30_000, remainingMs: 25 * 60_000 - 30_000 })),
    );
    const { unmount } = renderView();
    await screen.findByRole("button", { name: "Resume" });
    expect(screen.queryByRole("button", { name: "Stop & save" })).toBeNull();
    unmount();

    vi.mocked(api.getFocusState).mockResolvedValue(
      snapshot(session({ state: "paused", elapsedMs: 5 * 60_000, remainingMs: 20 * 60_000 })),
    );
    renderView();
    expect(await screen.findByRole("button", { name: "Stop & save" })).toBeTruthy();
  });

  it("shows breaks with 'Skip break' instead of Stop & save", async () => {
    vi.mocked(api.getFocusState).mockResolvedValue(
      snapshot(session({ kind: "break", plannedMs: 300_000, remainingMs: 280_000, elapsedMs: 20_000 })),
    );
    renderView();
    expect(await screen.findByRole("button", { name: "Skip break" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Stop & save" })).toBeNull();
  });

  it("reports a finished session once and refreshes history", async () => {
    const done = session({ state: "completed", elapsedMs: 25 * 60_000, remainingMs: 0, endedAtMs: 1_700_001_500_000 });
    vi.mocked(api.getFocusState).mockResolvedValue(snapshot(session({ remainingMs: 0, elapsedMs: 25 * 60_000 })));
    vi.mocked(api.completeFocusSession).mockResolvedValue(snapshot(null, done));
    renderView();
    expect(await screen.findByText(/Session complete: 25 min focused\./)).toBeTruthy();
    await waitFor(() => expect(api.completeFocusSession).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(vi.mocked(api.listFocusSessions).mock.calls.length).toBeGreaterThan(1));
  });

  it("shows a backend error and keeps the UI usable", async () => {
    vi.mocked(api.startFocusSession).mockRejectedValue(
      new api.ApiError("conflict", "A session is already in progress. Finish or reset it first."),
    );
    renderView();
    fireEvent.click(await screen.findByRole("button", { name: "Start focus" }));
    expect(await screen.findByText(/already in progress/)).toBeTruthy();
  });

  it("shows validation errors from the duration settings inline", async () => {
    vi.mocked(api.updateTimerSettings).mockRejectedValue(
      new api.ApiError("validation", "Focus duration must be between 1 and 240 minutes.", "focusMinutes"),
    );
    renderView();
    const input = await screen.findByLabelText("Focus (minutes)");
    await waitFor(() => expect((input as HTMLInputElement).value).toBe("25"));
    fireEvent.change(input, { target: { value: "500" } });
    fireEvent.click(screen.getByRole("button", { name: "Save durations" }));
    expect(await screen.findByText(/between 1 and 240 minutes/)).toBeTruthy();
  });
});
